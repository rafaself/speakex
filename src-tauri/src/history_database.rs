use rusqlite::Connection;
use std::{
    fs,
    path::{Path, PathBuf},
};
use tauri::{AppHandle, Manager};

#[derive(Clone, Debug)]
pub struct HistoryDatabase {
    path: PathBuf,
}

impl HistoryDatabase {
    pub fn path(&self) -> &Path {
        &self.path
    }
}

struct Migration {
    version: i32,
    sql: &'static str,
}

const MIGRATIONS: [Migration; 1] = [Migration {
    version: 1,
    sql: include_str!("../migrations/0001_create_transcriptions.sql"),
}];

pub fn initialize(app: &AppHandle) -> Result<HistoryDatabase, String> {
    let path = resolve_database_path(app)?;

    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|error| {
            format!(
                "failed to create database directory at {}: {error}",
                parent.display()
            )
        })?;
    }

    let mut connection = Connection::open(&path).map_err(|error| {
        format!(
            "failed to open history database at {}: {error}",
            path.display()
        )
    })?;

    apply_migrations(&mut connection)?;

    Ok(HistoryDatabase { path })
}

fn resolve_database_path(app: &AppHandle) -> Result<PathBuf, String> {
    app.path()
        .app_data_dir()
        .map(|directory| directory.join("history.sqlite3"))
        .map_err(|error| format!("failed to resolve app data directory: {error}"))
}

fn apply_migrations(connection: &mut Connection) -> Result<(), String> {
    let current_version = connection
        .pragma_query_value(None, "user_version", |row| row.get::<_, i32>(0))
        .map_err(|error| format!("failed to read history database schema version: {error}"))?;

    for migration in MIGRATIONS
        .iter()
        .filter(|migration| migration.version > current_version)
    {
        let transaction = connection.transaction().map_err(|error| {
            format!(
                "failed to start history database migration {}: {error}",
                migration.version
            )
        })?;

        transaction.execute_batch(migration.sql).map_err(|error| {
            format!(
                "failed to apply history database migration {}: {error}",
                migration.version
            )
        })?;

        transaction
            .pragma_update(None, "user_version", migration.version)
            .map_err(|error| {
                format!(
                    "failed to update history database schema version to {}: {error}",
                    migration.version
                )
            })?;

        transaction.commit().map_err(|error| {
            format!(
                "failed to commit history database migration {}: {error}",
                migration.version
            )
        })?;
    }

    Ok(())
}
