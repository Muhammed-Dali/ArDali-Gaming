use ardali_core::ArdaliError;
use rusqlite::{params, Connection, Row};

const GAME_SELECT: &str = "
    SELECT g.id, g.game_id, g.name, g.game_kind, g.runner, g.installer_path, g.install_dir,
           g.prefix_path, g.executable, g.arguments_json, g.created_at, g.last_played_at,
           g.display_mode, g.fps_overlay, g.total_playtime_seconds, g.active_session_id,
           m.cover_path, m.genre, m.release_year, m.description, g.preferred_runner,
           g.dxvk_enabled, g.dll_override, g.virtual_desktop, g.gamescope_enabled, g.resolution,
           g.protondb_note, g.ddraw_override, g.windows_version, g.gamescope_scaler, g.library_type,
           g.gamemode_enabled, g.steam_launch_options, g.source_available
    FROM games g
    LEFT JOIN game_metadata m ON m.game_id = g.id
";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoredGame {
    pub id: i64,
    pub game_id: String,
    pub name: String,
    pub game_kind: String,
    pub runner: String,
    pub installer_path: String,
    pub install_dir: String,
    pub prefix_path: Option<String>,
    pub executable: Option<String>,
    pub arguments_json: String,
    pub created_at: i64,
    pub last_played_at: Option<i64>,
    pub display_mode: String,
    pub fps_overlay: bool,
    pub total_playtime_seconds: i64,
    pub active_session_id: Option<i64>,
    pub cover_path: Option<String>,
    pub genre: Option<String>,
    pub release_year: Option<i64>,
    pub description: Option<String>,
    pub preferred_runner: String,
    pub dxvk_enabled: bool,
    pub dll_override: Option<String>,
    pub virtual_desktop: bool,
    pub gamescope_enabled: bool,
    pub resolution: String,
    pub protondb_note: Option<String>,
    pub ddraw_override: bool,
    pub windows_version: String,
    pub gamescope_scaler: String,
    pub library_type: String,
    pub gamemode_enabled: bool,
    pub steam_launch_options: String,
    pub source_available: bool,
}

pub struct NewGame<'a> {
    pub game_id: &'a str,
    pub name: &'a str,
    pub game_kind: &'a str,
    pub runner: &'a str,
    pub installer_path: &'a str,
    pub install_dir: &'a str,
    pub prefix_path: Option<&'a str>,
    pub executable: Option<&'a str>,
    pub arguments_json: &'a str,
    pub preferred_runner: &'a str,
    pub dxvk_enabled: bool,
    pub virtual_desktop: bool,
    pub gamescope_enabled: bool,
    pub resolution: &'a str,
    pub gamescope_scaler: &'a str,
    pub ddraw_override: bool,
    pub windows_version: &'a str,
    pub dll_override: Option<&'a str>,
    pub library_type: &'a str,
    pub display_mode: &'a str,
    pub gamemode_enabled: bool,
}

pub struct GameSettings<'a> {
    pub preferred_runner: &'a str,
    pub dxvk_enabled: bool,
    pub dll_override: Option<&'a str>,
    pub display_mode: &'a str,
    pub virtual_desktop: bool,
    pub gamescope_enabled: bool,
    pub resolution: &'a str,
    pub protondb_note: Option<&'a str>,
    pub ddraw_override: bool,
    pub windows_version: &'a str,
    pub game_kind: &'a str,
    pub gamescope_scaler: &'a str,
    pub gamemode_enabled: bool,
    pub steam_launch_options: &'a str,
}

pub fn insert_game(connection: &Connection, game: &NewGame<'_>) -> Result<i64, ArdaliError> {
    connection
        .execute(
            "INSERT INTO games (
          game_id, name, game_kind, runner, installer_path, install_dir,
          prefix_path, executable, arguments_json, preferred_runner, dxvk_enabled,
          virtual_desktop, gamescope_enabled, resolution, gamescope_scaler,
          ddraw_override, windows_version, dll_override, library_type, display_mode,
          gamemode_enabled
        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14,
                  ?15, ?16, ?17, ?18, ?19, ?20, ?21)",
            params![
                game.game_id,
                game.name,
                game.game_kind,
                game.runner,
                game.installer_path,
                game.install_dir,
                game.prefix_path,
                game.executable,
                game.arguments_json,
                game.preferred_runner,
                game.dxvk_enabled as i64,
                game.virtual_desktop as i64,
                game.gamescope_enabled as i64,
                game.resolution,
                game.gamescope_scaler,
                game.ddraw_override as i64,
                game.windows_version,
                game.dll_override,
                game.library_type,
                game.display_mode,
                game.gamemode_enabled as i64
            ],
        )
        .map_err(|error| storage_error("Cannot insert game record", error))?;
    Ok(connection.last_insert_rowid())
}

pub fn upsert_game(connection: &Connection, game: &NewGame<'_>) -> Result<(), ArdaliError> {
    connection
        .execute(
            "INSERT INTO games (game_id, name, game_kind, runner, installer_path, install_dir,
          prefix_path, executable, arguments_json, preferred_runner, dxvk_enabled,
          virtual_desktop, gamescope_enabled, resolution, gamescope_scaler,
          ddraw_override, windows_version, dll_override, library_type, display_mode,
          gamemode_enabled)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14,
                 ?15, ?16, ?17, ?18, ?19, ?20, ?21)
         ON CONFLICT(game_id) DO UPDATE SET name = excluded.name,
          game_kind = excluded.game_kind, runner = excluded.runner,
          installer_path = excluded.installer_path, install_dir = excluded.install_dir,
          prefix_path = excluded.prefix_path, executable = excluded.executable,
          arguments_json = excluded.arguments_json, library_type = excluded.library_type,
          source_available = 1,
          preferred_runner = CASE
            WHEN games.preferred_runner IN ('', 'steam-proton') THEN excluded.preferred_runner
            ELSE games.preferred_runner
          END",
            params![
                game.game_id,
                game.name,
                game.game_kind,
                game.runner,
                game.installer_path,
                game.install_dir,
                game.prefix_path,
                game.executable,
                game.arguments_json,
                game.preferred_runner,
                game.dxvk_enabled as i64,
                game.virtual_desktop as i64,
                game.gamescope_enabled as i64,
                game.resolution,
                game.gamescope_scaler,
                game.ddraw_override as i64,
                game.windows_version,
                game.dll_override,
                game.library_type,
                game.display_mode,
                game.gamemode_enabled as i64,
            ],
        )
        .map(|_| ())
        .map_err(|error| storage_error("Cannot upsert game record", error))
}

pub fn list_games(connection: &Connection) -> Result<Vec<StoredGame>, ArdaliError> {
    let mut statement = connection
        .prepare(&format!(
            "{GAME_SELECT} ORDER BY g.created_at DESC, g.id DESC"
        ))
        .map_err(|error| storage_error("Cannot prepare game list query", error))?;
    let rows = statement
        .query_map([], row_to_game)
        .map_err(|error| storage_error("Cannot query games", error))?;
    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|error| storage_error("Cannot read game record", error))
}

pub fn reconcile_steam_games(
    connection: &mut Connection,
    available_game_ids: &[String],
) -> Result<(), ArdaliError> {
    let transaction = connection
        .transaction()
        .map_err(|error| storage_error("Cannot start Steam reconciliation", error))?;
    transaction
        .execute(
            "UPDATE games SET source_available = 0 WHERE game_kind = 'steam'",
            [],
        )
        .map_err(|error| storage_error("Cannot mark Steam games unavailable", error))?;
    for game_id in available_game_ids {
        transaction
            .execute(
                "UPDATE games SET source_available = 1 WHERE game_id = ?1 AND game_kind = 'steam'",
                params![game_id],
            )
            .map_err(|error| storage_error("Cannot mark Steam game available", error))?;
    }
    transaction
        .commit()
        .map_err(|error| storage_error("Cannot commit Steam reconciliation", error))
}

pub fn get_game(connection: &Connection, id: i64) -> Result<StoredGame, ArdaliError> {
    connection
        .query_row(
            &format!("{GAME_SELECT} WHERE g.id = ?1"),
            params![id],
            row_to_game,
        )
        .map_err(|error| storage_error("Cannot read game record", error))
}

pub fn get_game_by_game_id(
    connection: &Connection,
    game_id: &str,
) -> Result<StoredGame, ArdaliError> {
    connection
        .query_row(
            &format!("{GAME_SELECT} WHERE g.game_id = ?1"),
            params![game_id],
            row_to_game,
        )
        .map_err(|error| storage_error("Cannot read game record", error))
}

pub fn game_id_exists(connection: &Connection, game_id: &str) -> Result<bool, ArdaliError> {
    connection
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM games WHERE game_id = ?1)",
            params![game_id],
            |row| row.get(0),
        )
        .map_err(|error| storage_error("Cannot check game id", error))
}

#[allow(clippy::too_many_arguments)]
pub fn save_metadata(
    connection: &Connection,
    id: i64,
    cover_path: Option<&str>,
    title: Option<&str>,
    genre: Option<&str>,
    release_year: Option<i64>,
    description: Option<&str>,
    source: &str,
) -> Result<(), ArdaliError> {
    connection.execute(
        "INSERT INTO game_metadata (game_id, cover_path, title, genre, release_year, description, source, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, unixepoch())
         ON CONFLICT(game_id) DO UPDATE SET
          cover_path = COALESCE(excluded.cover_path, game_metadata.cover_path),
          title = COALESCE(excluded.title, game_metadata.title),
          genre = COALESCE(excluded.genre, game_metadata.genre),
          release_year = COALESCE(excluded.release_year, game_metadata.release_year),
          description = COALESCE(excluded.description, game_metadata.description),
          source = excluded.source, updated_at = excluded.updated_at",
        params![id, cover_path, title, genre, release_year, description, source],
    ).map(|_| ()).map_err(|error| storage_error("Cannot save game metadata", error))
}

pub fn update_game_mode(
    connection: &Connection,
    id: i64,
    display_mode: &str,
    fps_overlay: bool,
) -> Result<(), ArdaliError> {
    connection
        .execute(
            "UPDATE games SET display_mode = ?2, fps_overlay = ?3 WHERE id = ?1",
            params![id, display_mode, fps_overlay as i64],
        )
        .map(|_| ())
        .map_err(|error| storage_error("Cannot update game mode options", error))
}

pub fn update_game_settings(
    connection: &Connection,
    id: i64,
    settings: &GameSettings<'_>,
) -> Result<(), ArdaliError> {
    connection.execute(
        "UPDATE games SET preferred_runner = ?2, dxvk_enabled = ?3, dll_override = NULLIF(?4, ''),
         display_mode = ?5, virtual_desktop = ?6, gamescope_enabled = ?7, resolution = ?8,
         protondb_note = NULLIF(?9, ''), ddraw_override = ?10, windows_version = ?11,
         game_kind = ?12, gamescope_scaler = ?13, gamemode_enabled = ?14,
         steam_launch_options = ?15 WHERE id = ?1",
        params![id, settings.preferred_runner, settings.dxvk_enabled as i64,
            settings.dll_override.unwrap_or_default(), settings.display_mode,
            settings.virtual_desktop as i64, settings.gamescope_enabled as i64,
            settings.resolution, settings.protondb_note.unwrap_or_default(),
            settings.ddraw_override as i64, settings.windows_version, settings.game_kind,
            settings.gamescope_scaler, settings.gamemode_enabled as i64,
            settings.steam_launch_options],
    ).map(|_| ()).map_err(|error| storage_error("Cannot update game settings", error))
}

pub fn mark_cncnet_installed(connection: &Connection, id: i64) -> Result<(), ArdaliError> {
    connection.execute("UPDATE games SET runner = 'cncnet', preferred_runner = 'cncnet', game_kind = 'cncnet' WHERE id = ?1", params![id])
        .map(|_| ()).map_err(|error| storage_error("Cannot update CnCNet runner", error))
}

pub fn delete_game(connection: &Connection, id: i64) -> Result<(), ArdaliError> {
    connection
        .execute("DELETE FROM games WHERE id = ?1", params![id])
        .map(|_| ())
        .map_err(|error| storage_error("Cannot remove game record", error))
}

fn row_to_game(row: &Row<'_>) -> rusqlite::Result<StoredGame> {
    Ok(StoredGame {
        id: row.get(0)?,
        game_id: row.get(1)?,
        name: row.get(2)?,
        game_kind: row.get(3)?,
        runner: row.get(4)?,
        installer_path: row.get(5)?,
        install_dir: row.get(6)?,
        prefix_path: row.get(7)?,
        executable: row.get(8)?,
        arguments_json: row.get(9)?,
        created_at: row.get(10)?,
        last_played_at: row.get(11)?,
        display_mode: row.get(12)?,
        fps_overlay: row.get::<_, i64>(13)? == 1,
        total_playtime_seconds: row.get(14)?,
        active_session_id: row.get(15)?,
        cover_path: row.get(16)?,
        genre: row.get(17)?,
        release_year: row.get(18)?,
        description: row.get(19)?,
        preferred_runner: row.get(20)?,
        dxvk_enabled: row.get::<_, i64>(21)? == 1,
        dll_override: row.get(22)?,
        virtual_desktop: row.get::<_, i64>(23)? == 1,
        gamescope_enabled: row.get::<_, i64>(24)? == 1,
        resolution: row.get(25)?,
        protondb_note: row.get(26)?,
        ddraw_override: row.get::<_, i64>(27)? == 1,
        windows_version: row.get(28)?,
        gamescope_scaler: row.get(29)?,
        library_type: row.get(30)?,
        gamemode_enabled: row.get::<_, i64>(31)? == 1,
        steam_launch_options: row.get(32)?,
        source_available: row.get::<_, i64>(33)? == 1,
    })
}

fn storage_error(context: &str, error: rusqlite::Error) -> ArdaliError {
    ArdaliError::Storage(format!("{context}: {error}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::initialize_connection;

    fn database() -> Connection {
        let connection = Connection::open_in_memory().unwrap();
        initialize_connection(&connection).unwrap();
        connection
    }

    fn new_game<'a>(game_id: &'a str, name: &'a str) -> NewGame<'a> {
        NewGame {
            game_id,
            name,
            game_kind: "windows-exe",
            runner: "wine",
            installer_path: "/games/setup.exe",
            install_dir: "/games/demo",
            prefix_path: None,
            executable: Some("/games/demo/game.exe"),
            arguments_json: "[]",
            preferred_runner: "wine",
            dxvk_enabled: true,
            virtual_desktop: false,
            gamescope_enabled: false,
            resolution: "auto",
            gamescope_scaler: "fit",
            ddraw_override: false,
            windows_version: "win10",
            dll_override: None,
            library_type: "game",
            display_mode: "windowed",
            gamemode_enabled: false,
        }
    }

    #[test]
    fn inserts_lists_and_deletes_games() {
        let connection = database();
        let id = insert_game(&connection, &new_game("demo", "Demo")).unwrap();
        assert!(game_id_exists(&connection, "demo").unwrap());
        assert_eq!(get_game(&connection, id).unwrap().name, "Demo");
        assert_eq!(list_games(&connection).unwrap().len(), 1);
        delete_game(&connection, id).unwrap();
        assert!(list_games(&connection).unwrap().is_empty());
    }

    #[test]
    fn updates_metadata_mode_and_settings() {
        let connection = database();
        let id = insert_game(&connection, &new_game("demo", "Demo")).unwrap();
        save_metadata(
            &connection,
            id,
            Some("/covers/demo.jpg"),
            None,
            Some("Action"),
            Some(2026),
            None,
            "test",
        )
        .unwrap();
        update_game_mode(&connection, id, "fullscreen", true).unwrap();
        update_game_settings(
            &connection,
            id,
            &GameSettings {
                preferred_runner: "proton",
                dxvk_enabled: true,
                dll_override: None,
                display_mode: "fullscreen",
                virtual_desktop: true,
                gamescope_enabled: true,
                resolution: "1920x1080",
                protondb_note: Some("gold"),
                ddraw_override: false,
                windows_version: "win11",
                game_kind: "windows-exe",
                gamescope_scaler: "fill",
                gamemode_enabled: true,
                steam_launch_options: "-novid -console",
            },
        )
        .unwrap();
        let game = get_game(&connection, id).unwrap();
        assert_eq!(game.cover_path.as_deref(), Some("/covers/demo.jpg"));
        assert_eq!(game.genre.as_deref(), Some("Action"));
        assert_eq!(game.preferred_runner, "proton");
        assert!(game.fps_overlay);
        assert_eq!(game.gamescope_scaler, "fill");
        assert!(game.gamemode_enabled);
        assert_eq!(game.steam_launch_options, "-novid -console");
    }

    #[test]
    fn upserts_games_without_duplicates_and_preserves_user_settings() {
        let connection = database();
        let mut steam = new_game("steam-10", "First");
        steam.game_kind = "steam";
        steam.runner = "steam";
        steam.preferred_runner = "steam";
        steam.installer_path = "/steam/10.acf";
        steam.install_dir = "/steam/first";
        upsert_game(&connection, &steam).unwrap();
        let id = get_game_by_game_id(&connection, "steam-10").unwrap().id;
        save_metadata(
            &connection,
            id,
            Some("/covers/steam-10.jpg"),
            None,
            Some("Action"),
            Some(1998),
            None,
            "manual",
        )
        .unwrap();
        update_game_mode(&connection, id, "fullscreen", true).unwrap();
        steam.name = "Updated";
        steam.install_dir = "/steam/updated";
        upsert_game(&connection, &steam).unwrap();
        assert_eq!(list_games(&connection).unwrap().len(), 1);
        let updated = get_game_by_game_id(&connection, "steam-10").unwrap();
        assert_eq!(updated.name, "Updated");
        assert_eq!(updated.install_dir, "/steam/updated");
        assert_eq!(updated.game_kind, "steam");
        assert_eq!(updated.runner, "steam");
        assert_eq!(updated.cover_path.as_deref(), Some("/covers/steam-10.jpg"));
        assert_eq!(updated.genre.as_deref(), Some("Action"));
        assert_eq!(updated.display_mode, "fullscreen");
        assert!(updated.fps_overlay);
    }

    #[test]
    fn reconciles_steam_availability_without_deleting_records() {
        let mut connection = database();
        let mut steam = new_game("steam-10", "Steam Game");
        steam.game_kind = "steam";
        steam.runner = "steam";
        upsert_game(&connection, &steam).unwrap();

        reconcile_steam_games(&mut connection, &[]).unwrap();
        let unavailable = get_game_by_game_id(&connection, "steam-10").unwrap();
        assert!(!unavailable.source_available);

        reconcile_steam_games(&mut connection, &["steam-10".into()]).unwrap();
        assert!(
            get_game_by_game_id(&connection, "steam-10")
                .unwrap()
                .source_available
        );
    }
}
