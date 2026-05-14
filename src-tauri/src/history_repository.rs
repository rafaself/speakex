use crate::history_database::HistoryDatabase;
use rusqlite::{params, Connection, OptionalExtension, Row};
use serde::Serialize;
use std::time::{SystemTime, UNIX_EPOCH};

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

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ErrorLogEntry {
    pub id: String,
    pub scope: String,
    pub source: String,
    pub summary: String,
    pub detail: String,
    pub created_at: String,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ClearErrorLogsResult {
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

#[derive(Clone, Debug)]
pub struct NewErrorLog {
    pub scope: String,
    pub source: String,
    pub summary: String,
    pub detail: String,
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

    pub fn get_error_logs(&self) -> Result<Vec<ErrorLogEntry>, String> {
        let connection = self.open_connection()?;
        let mut statement = connection
            .prepare(
                "SELECT
                    id,
                    scope,
                    source,
                    summary,
                    detail,
                    created_at
                 FROM error_logs
                 ORDER BY created_at DESC, id DESC",
            )
            .map_err(|error| format!("failed to prepare error log list query: {error}"))?;

        let rows = statement
            .query_map([], map_error_log)
            .map_err(|error| format!("failed to query error logs: {error}"))?;

        rows.collect::<Result<Vec<_>, _>>()
            .map_err(|error| format!("failed to map error logs: {error}"))
    }

    pub fn clear_error_logs(&self) -> Result<ClearErrorLogsResult, String> {
        let connection = self.open_connection()?;
        let deleted_count = connection
            .execute("DELETE FROM error_logs", [])
            .map_err(|error| format!("failed to clear error logs: {error}"))?;

        Ok(ClearErrorLogsResult { deleted_count })
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

    pub fn save_error_log(&self, entry: &NewErrorLog) -> Result<(), String> {
        let connection = self.open_connection()?;
        let id = generate_record_id(&entry.scope, "error log")?;

        connection
            .execute(
                "INSERT INTO error_logs (
                    id,
                    scope,
                    source,
                    summary,
                    detail,
                    created_at
                 ) VALUES (?1, ?2, ?3, ?4, ?5, strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))",
                params![id, entry.scope, entry.source, entry.summary, entry.detail],
            )
            .map_err(|error| format!("failed to save error log: {error}"))?;

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

fn generate_record_id(prefix: &str, label: &str) -> Result<String, String> {
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|error| format!("failed to generate {label} identifier: {error}"))?;

    Ok(format!(
        "{}-{}-{}",
        prefix.trim().to_lowercase(),
        timestamp.as_secs(),
        timestamp.subsec_nanos()
    ))
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

fn map_error_log(row: &Row<'_>) -> rusqlite::Result<ErrorLogEntry> {
    Ok(ErrorLogEntry {
        id: row.get("id")?,
        scope: row.get("scope")?,
        source: row.get("source")?,
        summary: row.get("summary")?,
        detail: row.get("detail")?,
        created_at: row.get("created_at")?,
    })
}

#[cfg(test)]
mod tests {
    use super::{HistoryRepository, NewErrorLog};
    use crate::history_database::HistoryDatabase;
    use std::{env, fs, path::PathBuf, time::{SystemTime, UNIX_EPOCH}};

    #[test]
    fn saves_lists_and_clears_error_logs() {
        let repository = HistoryRepository::new(HistoryDatabase::from_path(create_test_database_path()));

        initialize_error_logs_table(&repository);

        repository
            .save_error_log(&NewErrorLog {
                scope: "transcription".to_string(),
                source: "frontend".to_string(),
                summary: "Transcription did not finish".to_string(),
                detail: "Gemini API returned 400 Bad Request".to_string(),
            })
            .expect("error log should be saved");

        let logs = repository.get_error_logs().expect("error logs should load");
        assert_eq!(logs.len(), 1);
        assert_eq!(logs[0].scope, "transcription");
        assert_eq!(logs[0].source, "frontend");
        assert_eq!(logs[0].summary, "Transcription did not finish");
        assert_eq!(logs[0].detail, "Gemini API returned 400 Bad Request");

        let result = repository.clear_error_logs().expect("error logs should clear");
        assert_eq!(result.deleted_count, 1);
        assert!(repository.get_error_logs().expect("error logs should reload").is_empty());
    }

    fn create_test_database_path() -> PathBuf {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time should be after epoch");
        let path = env::temp_dir().join(format!(
            "speakex-history-repository-test-{}-{}.sqlite3",
            timestamp.as_secs(),
            timestamp.subsec_nanos()
        ));

        if path.exists() {
            fs::remove_file(&path).expect("stale test database should be removable");
        }

        path
    }

    fn initialize_error_logs_table(repository: &HistoryRepository) {
        let connection = repository.open_connection().expect("database should open");
        connection
            .execute_batch(
                "CREATE TABLE error_logs (
                    id TEXT PRIMARY KEY,
                    scope TEXT NOT NULL,
                    source TEXT NOT NULL,
                    summary TEXT NOT NULL,
                    detail TEXT NOT NULL,
                    created_at TEXT NOT NULL
                );",
            )
            .expect("error logs table should be created");
    }
}
