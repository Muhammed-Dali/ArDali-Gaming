use ardali_core::ArdaliError;
use rusqlite::{params, Connection, OptionalExtension};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoredSetting {
    pub key: String,
    pub value: String,
}

pub fn list_settings(connection: &Connection) -> Result<Vec<StoredSetting>, ArdaliError> {
    let mut statement = connection
        .prepare("SELECT key, value FROM settings ORDER BY key")
        .map_err(storage_error("Cannot prepare settings query"))?;
    let rows = statement
        .query_map([], |row| {
            Ok(StoredSetting {
                key: row.get(0)?,
                value: row.get(1)?,
            })
        })
        .map_err(storage_error("Cannot query settings"))?;

    rows.collect::<Result<Vec<_>, _>>()
        .map_err(storage_error("Cannot read setting"))
}

pub fn set_setting(
    connection: &Connection,
    key: &str,
    value: &str,
) -> Result<StoredSetting, ArdaliError> {
    connection
        .execute(
            "
            INSERT INTO settings (key, value, updated_at)
            VALUES (?1, ?2, unixepoch())
            ON CONFLICT(key) DO UPDATE SET
              value = excluded.value,
              updated_at = excluded.updated_at
            ",
            params![key, value],
        )
        .map_err(storage_error("Cannot write setting"))?;

    connection
        .query_row(
            "SELECT key, value FROM settings WHERE key = ?1",
            params![key],
            |row| {
                Ok(StoredSetting {
                    key: row.get(0)?,
                    value: row.get(1)?,
                })
            },
        )
        .map_err(storage_error("Cannot read setting"))
}

pub fn get_setting_value(
    connection: &Connection,
    key: &str,
) -> Result<Option<String>, ArdaliError> {
    connection
        .query_row(
            "SELECT value FROM settings WHERE key = ?1",
            params![key],
            |row| row.get(0),
        )
        .optional()
        .map_err(storage_error("Cannot read setting"))
}

fn storage_error(context: &'static str) -> impl FnOnce(rusqlite::Error) -> ArdaliError {
    move |error| ArdaliError::Storage(format!("{context}: {error}"))
}

#[cfg(test)]
mod tests {
    use super::{get_setting_value, list_settings, set_setting};
    use crate::initialize_connection;
    use rusqlite::Connection;

    #[test]
    fn lists_defaults_and_upserts_values() {
        let connection = Connection::open_in_memory().unwrap();
        initialize_connection(&connection).unwrap();

        assert_eq!(list_settings(&connection).unwrap().len(), 5);
        let saved = set_setting(&connection, "fps_overlay", "true").unwrap();
        assert_eq!(saved.value, "true");
        assert_eq!(
            get_setting_value(&connection, "fps_overlay").unwrap(),
            Some("true".into())
        );
        assert_eq!(get_setting_value(&connection, "missing").unwrap(), None);
    }
}
