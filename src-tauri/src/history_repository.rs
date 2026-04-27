use crate::history_database::HistoryDatabase;
use rusqlite::{params, Connection, OptionalExtension, Row};
use serde::Serialize;

#[derive(Clone, Debug)]
pub struct HistoryRepository {
    database: HistoryDatabase,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HistoryTranscriptionSummary {
    pub id: String,
    pub text: String,
    pub provider: String,
    pub model: Option<String>,
    pub language: Option<String>,
    pub duration_ms: Option<i64>,
    pub copied_to_clipboard: bool,
    pub has_audio_file: bool,
    pub has_error: bool,
    pub created_at: String,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HistoryTranscription {
    pub id: String,
    pub text: String,
    pub provider: String,
    pub model: Option<String>,
    pub language: Option<String>,
    pub duration_ms: Option<i64>,
    pub audio_path: Option<String>,
    pub audio_deleted: bool,
    pub copied_to_clipboard: bool,
    pub error: Option<String>,
    pub created_at: String,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DeleteTranscriptionResult {
    pub deleted: bool,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ClearHistoryResult {
    pub deleted_count: usize,
}

#[derive(Clone, Debug)]
pub struct NewHistoryTranscription {
    pub id: String,
    pub text: String,
    pub provider: String,
    pub model: Option<String>,
    pub language: Option<String>,
    pub duration_ms: Option<i64>,
    pub audio_path: Option<String>,
    pub audio_deleted: bool,
    pub copied_to_clipboard: bool,
    pub error: Option<String>,
}

impl HistoryRepository {
    pub fn new(database: HistoryDatabase) -> Self {
        Self { database }
    }

    pub fn get_history(&self) -> Result<Vec<HistoryTranscriptionSummary>, String> {
        let connection = self.open_connection()?;
        let mut statement = connection
            .prepare(
                "SELECT
                    id,
                    text,
                    provider,
                    model,
                    language,
                    duration_ms,
                    audio_path,
                    audio_deleted,
                    copied_to_clipboard,
                    error,
                    created_at
                 FROM transcriptions
                 ORDER BY created_at DESC, id DESC",
            )
            .map_err(|error| format!("failed to prepare history list query: {error}"))?;

        let rows = statement
            .query_map([], map_history_summary)
            .map_err(|error| format!("failed to query history entries: {error}"))?;

        rows.collect::<Result<Vec<_>, _>>()
            .map_err(|error| format!("failed to map history entries: {error}"))
    }

    pub fn get_transcription(&self, id: &str) -> Result<Option<HistoryTranscription>, String> {
        let connection = self.open_connection()?;
        let mut statement = connection
            .prepare(
                "SELECT
                    id,
                    text,
                    provider,
                    model,
                    language,
                    duration_ms,
                    audio_path,
                    audio_deleted,
                    copied_to_clipboard,
                    error,
                    created_at
                 FROM transcriptions
                 WHERE id = ?1
                 LIMIT 1",
            )
            .map_err(|error| format!("failed to prepare transcription lookup query: {error}"))?;

        statement
            .query_row([id], map_transcription)
            .optional()
            .map_err(|error| format!("failed to query transcription {id}: {error}"))
    }

    pub fn delete_transcription(&self, id: &str) -> Result<DeleteTranscriptionResult, String> {
        let connection = self.open_connection()?;
        let deleted_count = connection
            .execute("DELETE FROM transcriptions WHERE id = ?1", [id])
            .map_err(|error| format!("failed to delete transcription {id}: {error}"))?;

        Ok(DeleteTranscriptionResult {
            deleted: deleted_count > 0,
        })
    }

    pub fn clear_history(&self) -> Result<ClearHistoryResult, String> {
        let connection = self.open_connection()?;
        let deleted_count = connection
            .execute("DELETE FROM transcriptions", [])
            .map_err(|error| format!("failed to clear history: {error}"))?;

        Ok(ClearHistoryResult { deleted_count })
    }

    pub fn save_transcription(&self, entry: &NewHistoryTranscription) -> Result<(), String> {
        let connection = self.open_connection()?;

        connection
            .execute(
                "INSERT INTO transcriptions (
                    id,
                    text,
                    provider,
                    model,
                    language,
                    duration_ms,
                    audio_path,
                    audio_deleted,
                    copied_to_clipboard,
                    error,
                    created_at
                 ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))",
                params![
                    entry.id,
                    entry.text,
                    entry.provider,
                    entry.model,
                    entry.language,
                    entry.duration_ms,
                    entry.audio_path,
                    entry.audio_deleted,
                    entry.copied_to_clipboard,
                    entry.error
                ],
            )
            .map_err(|error| format!("failed to save transcription {}: {error}", entry.id))?;

        Ok(())
    }

    fn open_connection(&self) -> Result<Connection, String> {
        Connection::open(self.database.path()).map_err(|error| {
            format!(
                "failed to open history database at {}: {error}",
                self.database.path().display()
            )
        })
    }
}

fn map_history_summary(row: &Row<'_>) -> rusqlite::Result<HistoryTranscriptionSummary> {
    let audio_path: Option<String> = row.get("audio_path")?;
    let audio_deleted: bool = row.get("audio_deleted")?;
    let error: Option<String> = row.get("error")?;

    Ok(HistoryTranscriptionSummary {
        id: row.get("id")?,
        text: row.get("text")?,
        provider: row.get("provider")?,
        model: row.get("model")?,
        language: row.get("language")?,
        duration_ms: row.get("duration_ms")?,
        copied_to_clipboard: row.get("copied_to_clipboard")?,
        has_audio_file: audio_path.is_some() && !audio_deleted,
        has_error: error.as_ref().is_some_and(|value| !value.is_empty()),
        created_at: row.get("created_at")?,
    })
}

fn map_transcription(row: &Row<'_>) -> rusqlite::Result<HistoryTranscription> {
    Ok(HistoryTranscription {
        id: row.get("id")?,
        text: row.get("text")?,
        provider: row.get("provider")?,
        model: row.get("model")?,
        language: row.get("language")?,
        duration_ms: row.get("duration_ms")?,
        audio_path: row.get("audio_path")?,
        audio_deleted: row.get("audio_deleted")?,
        copied_to_clipboard: row.get("copied_to_clipboard")?,
        error: row.get("error")?,
        created_at: row.get("created_at")?,
    })
}
