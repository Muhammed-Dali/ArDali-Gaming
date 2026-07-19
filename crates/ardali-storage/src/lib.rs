//! UI-independent SQLite connection and schema management.

use ardali_core::ArdaliError;
use rusqlite::{params, Connection};
use std::{fs, path::Path};

mod games;
mod sessions;
mod settings;

pub use games::{
    delete_game, game_id_exists, get_game, get_game_by_game_id, insert_game, list_games,
    mark_cncnet_installed, reconcile_steam_games, save_metadata, update_game_mode,
    update_game_settings, upsert_game, GameSettings, NewGame, StoredGame,
};
pub use sessions::{clear_play_session, finish_play_session, start_play_session};
pub use settings::{get_setting_value, list_settings, set_setting, StoredSetting};

pub type StorageConnection = Connection;

const BASE_SCHEMA: &str = "
CREATE TABLE IF NOT EXISTS games (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  game_id TEXT NOT NULL UNIQUE,
  name TEXT NOT NULL,
  game_kind TEXT NOT NULL,
  runner TEXT NOT NULL,
  installer_path TEXT NOT NULL,
  install_dir TEXT NOT NULL,
  prefix_path TEXT,
  executable TEXT,
  arguments_json TEXT NOT NULL,
  created_at INTEGER NOT NULL DEFAULT (unixepoch())
);
CREATE TABLE IF NOT EXISTS play_sessions (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  game_id INTEGER NOT NULL,
  started_at INTEGER NOT NULL DEFAULT (unixepoch()),
  ended_at INTEGER,
  duration_seconds INTEGER,
  FOREIGN KEY(game_id) REFERENCES games(id) ON DELETE CASCADE
);
CREATE TABLE IF NOT EXISTS settings (
  key TEXT PRIMARY KEY,
  value TEXT NOT NULL,
  updated_at INTEGER NOT NULL DEFAULT (unixepoch())
);
CREATE TABLE IF NOT EXISTS game_metadata (
  game_id INTEGER PRIMARY KEY,
  cover_path TEXT,
  title TEXT,
  genre TEXT,
  release_year INTEGER,
  description TEXT,
  source TEXT,
  updated_at INTEGER NOT NULL DEFAULT (unixepoch()),
  FOREIGN KEY(game_id) REFERENCES games(id) ON DELETE CASCADE
);
";

const GAME_MIGRATIONS: &[&str] = &[
    "ALTER TABLE games ADD COLUMN last_played_at INTEGER",
    "ALTER TABLE games ADD COLUMN display_mode TEXT NOT NULL DEFAULT 'windowed'",
    "ALTER TABLE games ADD COLUMN fps_overlay INTEGER NOT NULL DEFAULT 0",
    "ALTER TABLE games ADD COLUMN total_playtime_seconds INTEGER NOT NULL DEFAULT 0",
    "ALTER TABLE games ADD COLUMN active_session_id INTEGER",
    "ALTER TABLE games ADD COLUMN preferred_runner TEXT NOT NULL DEFAULT ''",
    "ALTER TABLE games ADD COLUMN dxvk_enabled INTEGER NOT NULL DEFAULT 1",
    "ALTER TABLE games ADD COLUMN dll_override TEXT",
    "ALTER TABLE games ADD COLUMN virtual_desktop INTEGER NOT NULL DEFAULT 1",
    "ALTER TABLE games ADD COLUMN gamescope_enabled INTEGER NOT NULL DEFAULT 0",
    "ALTER TABLE games ADD COLUMN resolution TEXT NOT NULL DEFAULT 'auto'",
    "ALTER TABLE games ADD COLUMN gamescope_scaler TEXT NOT NULL DEFAULT 'fit'",
    "ALTER TABLE games ADD COLUMN protondb_note TEXT",
    "ALTER TABLE games ADD COLUMN ddraw_override INTEGER NOT NULL DEFAULT 1",
    "ALTER TABLE games ADD COLUMN windows_version TEXT NOT NULL DEFAULT ''",
    "ALTER TABLE games ADD COLUMN library_type TEXT NOT NULL DEFAULT 'game'",
    "ALTER TABLE games ADD COLUMN gamemode_enabled INTEGER NOT NULL DEFAULT 0",
    "ALTER TABLE games ADD COLUMN steam_launch_options TEXT NOT NULL DEFAULT ''",
    "ALTER TABLE games ADD COLUMN source_available INTEGER NOT NULL DEFAULT 1",
];

const DEFAULT_SETTINGS: &[(&str, &str)] = &[
    ("default_display_mode", "windowed"),
    ("library_runner_filter", "all"),
    ("fps_overlay", "false"),
    ("auto_sync_steam", "false"),
    ("steamgriddb_api_key", ""),
];

pub fn open(path: impl AsRef<Path>) -> Result<Connection, ArdaliError> {
    let path = path.as_ref();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|error| {
            ArdaliError::Storage(format!("Cannot create database directory: {error}"))
        })?;
    }
    Connection::open(path)
        .map_err(|error| ArdaliError::Storage(format!("Cannot open SQLite database: {error}")))
}

pub fn initialize(path: impl AsRef<Path>) -> Result<(), ArdaliError> {
    let connection = open(path)?;
    initialize_connection(&connection)
}

pub fn initialize_connection(connection: &Connection) -> Result<(), ArdaliError> {
    connection.execute_batch(BASE_SCHEMA).map_err(|error| {
        ArdaliError::Storage(format!("Cannot initialize SQLite database: {error}"))
    })?;

    for migration in GAME_MIGRATIONS {
        migrate_column(connection, migration)?;
    }
    seed_default_settings(connection)
}

fn migrate_column(connection: &Connection, sql: &str) -> Result<(), ArdaliError> {
    connection
        .execute(sql, [])
        .or_else(|error| {
            if error.to_string().contains("duplicate column name") {
                Ok(0)
            } else {
                Err(error)
            }
        })
        .map(|_| ())
        .map_err(|error| ArdaliError::Storage(format!("Cannot migrate SQLite database: {error}")))
}

fn seed_default_settings(connection: &Connection) -> Result<(), ArdaliError> {
    for (key, value) in DEFAULT_SETTINGS {
        connection
            .execute(
                "INSERT OR IGNORE INTO settings (key, value) VALUES (?1, ?2)",
                params![key, value],
            )
            .map_err(|error| {
                ArdaliError::Storage(format!("Cannot seed default setting: {error}"))
            })?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{initialize, initialize_connection, open, DEFAULT_SETTINGS};
    use rusqlite::Connection;
    use std::{fs, path::PathBuf, process, time::SystemTime};

    fn temporary_database(name: &str) -> PathBuf {
        let nonce = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!(
            "ardali-storage-{name}-{}-{nonce}.db",
            process::id()
        ))
    }

    fn game_columns(connection: &Connection) -> Vec<String> {
        let mut statement = connection.prepare("PRAGMA table_info(games)").unwrap();
        statement
            .query_map([], |row| row.get(1))
            .unwrap()
            .collect::<Result<Vec<String>, _>>()
            .unwrap()
    }

    #[test]
    fn creates_current_schema_and_default_settings() {
        let path = temporary_database("current");
        initialize(&path).unwrap();
        let connection = open(&path).unwrap();

        let columns = game_columns(&connection);
        assert!(columns.contains(&"library_type".to_string()));
        assert!(columns.contains(&"windows_version".to_string()));
        assert!(columns.contains(&"gamemode_enabled".to_string()));

        let setting_count: i64 = connection
            .query_row("SELECT COUNT(*) FROM settings", [], |row| row.get(0))
            .unwrap();
        assert_eq!(setting_count, DEFAULT_SETTINGS.len() as i64);

        drop(connection);
        fs::remove_file(path).unwrap();
    }

    #[test]
    fn migrates_a_legacy_games_table_without_losing_records() {
        let connection = Connection::open_in_memory().unwrap();
        connection
            .execute_batch(
                "CREATE TABLE games (
                    id INTEGER PRIMARY KEY AUTOINCREMENT,
                    game_id TEXT NOT NULL UNIQUE,
                    name TEXT NOT NULL,
                    game_kind TEXT NOT NULL,
                    runner TEXT NOT NULL,
                    installer_path TEXT NOT NULL,
                    install_dir TEXT NOT NULL,
                    prefix_path TEXT,
                    executable TEXT,
                    arguments_json TEXT NOT NULL,
                    created_at INTEGER NOT NULL DEFAULT (unixepoch())
                );
                INSERT INTO games (
                    game_id, name, game_kind, runner, installer_path,
                    install_dir, arguments_json
                ) VALUES ('legacy', 'Legacy Game', 'windows-exe', 'wine',
                          '/tmp/setup.exe', '/tmp/legacy', '[]');",
            )
            .unwrap();

        initialize_connection(&connection).unwrap();

        let name: String = connection
            .query_row(
                "SELECT name FROM games WHERE game_id = 'legacy'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(name, "Legacy Game");
        assert!(game_columns(&connection).contains(&"library_type".to_string()));
        assert!(game_columns(&connection).contains(&"gamemode_enabled".to_string()));
    }

    #[test]
    fn initialization_is_idempotent() {
        let connection = Connection::open_in_memory().unwrap();
        initialize_connection(&connection).unwrap();
        initialize_connection(&connection).unwrap();

        let setting_count: i64 = connection
            .query_row("SELECT COUNT(*) FROM settings", [], |row| row.get(0))
            .unwrap();
        assert_eq!(setting_count, DEFAULT_SETTINGS.len() as i64);
    }
}
