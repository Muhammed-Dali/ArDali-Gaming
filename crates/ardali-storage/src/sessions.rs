use ardali_core::ArdaliError;
use rusqlite::{params, Connection};

pub fn start_play_session(
    connection: &Connection,
    game_id: i64,
    display_mode: &str,
    fps_overlay: bool,
) -> Result<i64, ArdaliError> {
    connection
        .execute(
            "UPDATE games SET active_session_id = NULL
             WHERE id = ?1 AND active_session_id IS NOT NULL",
            params![game_id],
        )
        .map_err(storage_error("Cannot reset stale play session"))?;
    connection
        .execute(
            "INSERT INTO play_sessions (game_id) VALUES (?1)",
            params![game_id],
        )
        .map_err(storage_error("Cannot create play session"))?;
    let session_id = connection.last_insert_rowid();
    connection
        .execute(
            "
            UPDATE games
            SET last_played_at = unixepoch(),
                display_mode = ?2,
                fps_overlay = ?3,
                active_session_id = ?4
            WHERE id = ?1
            ",
            params![game_id, display_mode, fps_overlay as i64, session_id],
        )
        .map_err(storage_error("Cannot update last played time"))?;
    Ok(session_id)
}

pub fn finish_play_session(
    connection: &Connection,
    game_id: i64,
    session_id: i64,
) -> Result<(), ArdaliError> {
    connection
        .execute(
            "
            UPDATE play_sessions
            SET ended_at = unixepoch(),
                duration_seconds = unixepoch() - started_at
            WHERE id = ?1 AND ended_at IS NULL
            ",
            params![session_id],
        )
        .map_err(storage_error("Cannot finish play session"))?;
    connection
        .execute(
            "
            UPDATE games
            SET total_playtime_seconds = total_playtime_seconds + COALESCE((
                  SELECT duration_seconds FROM play_sessions WHERE id = ?2
                ), 0),
                active_session_id = NULL
            WHERE id = ?1
            ",
            params![game_id, session_id],
        )
        .map_err(storage_error("Cannot update total playtime"))?;
    Ok(())
}

pub fn clear_play_session(
    connection: &Connection,
    game_id: i64,
    session_id: i64,
) -> Result<(), ArdaliError> {
    connection
        .execute(
            "
            UPDATE play_sessions
            SET ended_at = unixepoch(),
                duration_seconds = COALESCE(duration_seconds, unixepoch() - started_at)
            WHERE id = ?1 AND ended_at IS NULL
            ",
            params![session_id],
        )
        .map_err(storage_error("Cannot clear play session"))?;
    connection
        .execute(
            "UPDATE games SET active_session_id = NULL WHERE id = ?1",
            params![game_id],
        )
        .map_err(storage_error("Cannot clear active play session"))?;
    Ok(())
}

fn storage_error(context: &'static str) -> impl FnOnce(rusqlite::Error) -> ArdaliError {
    move |error| ArdaliError::Storage(format!("{context}: {error}"))
}

#[cfg(test)]
mod tests {
    use super::{clear_play_session, finish_play_session, start_play_session};
    use crate::initialize_connection;
    use rusqlite::{params, Connection};

    fn database_with_game() -> Connection {
        let connection = Connection::open_in_memory().unwrap();
        initialize_connection(&connection).unwrap();
        connection
            .execute(
                "INSERT INTO games (
                    game_id, name, game_kind, runner, installer_path,
                    install_dir, arguments_json
                 ) VALUES ('test', 'Test Game', 'windows-exe', 'wine',
                           '/tmp/test.exe', '/tmp/test', '[]')",
                [],
            )
            .unwrap();
        connection
    }

    #[test]
    fn starts_and_finishes_a_session() {
        let connection = database_with_game();
        let id: i64 = connection
            .query_row("SELECT id FROM games WHERE game_id = 'test'", [], |row| {
                row.get(0)
            })
            .unwrap();

        let session_id = start_play_session(&connection, id, "fullscreen", true).unwrap();
        let active: i64 = connection
            .query_row(
                "SELECT active_session_id FROM games WHERE id = ?1",
                params![id],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(active, session_id);

        finish_play_session(&connection, id, session_id).unwrap();
        let active: Option<i64> = connection
            .query_row(
                "SELECT active_session_id FROM games WHERE id = ?1",
                params![id],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(active, None);
    }

    #[test]
    fn clears_a_detached_session() {
        let connection = database_with_game();
        let id: i64 = connection
            .query_row("SELECT id FROM games WHERE game_id = 'test'", [], |row| {
                row.get(0)
            })
            .unwrap();
        let session_id = start_play_session(&connection, id, "windowed", false).unwrap();

        clear_play_session(&connection, id, session_id).unwrap();
        let active: Option<i64> = connection
            .query_row(
                "SELECT active_session_id FROM games WHERE id = ?1",
                params![id],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(active, None);
    }
}
