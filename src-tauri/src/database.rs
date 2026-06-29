use crate::runtime::{self, GameKind, RunnerSelection};
use crate::steam::SteamGame;
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use std::{fs, path::Path};

struct GameDefaults {
    windows_version: &'static str,
    display_mode: &'static str,
    dxvk_enabled: bool,
    virtual_desktop: bool,
    gamescope_enabled: bool,
    resolution: &'static str,
    gamescope_scaler: &'static str,
    ddraw_override: bool,
    dll_override: Option<&'static str>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GameInstallRequest {
    pub name: String,
    pub game_kind: GameKind,
    pub library_type: Option<LibraryType>,
    pub preferred_runner: Option<String>,
    pub prefix_mode: Option<PrefixMode>,
    pub installer_path: String,
    pub install_dir: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum PrefixMode {
    Isolated,
    SharedWindowsApps,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum LibraryType {
    Game,
    WindowsApp,
    Tool,
    Installer,
}

impl LibraryType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Game => "game",
            Self::WindowsApp => "windows-app",
            Self::Tool => "tool",
            Self::Installer => "installer",
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GameRecord {
    pub id: i64,
    pub game_id: String,
    pub name: String,
    pub game_kind: String,
    pub library_type: String,
    pub runner: String,
    pub installer_path: String,
    pub install_dir: String,
    pub prefix_path: Option<String>,
    pub executable: Option<String>,
    pub arguments: Vec<String>,
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
    pub gamescope_scaler: String,
    pub protondb_note: Option<String>,
    pub ddraw_override: bool,
    pub windows_version: String,
    pub cncnet_installed: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppSetting {
    pub key: String,
    pub value: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum DisplayMode {
    Windowed,
    Fullscreen,
}

impl DisplayMode {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Windowed => "windowed",
            Self::Fullscreen => "fullscreen",
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GameModeOptions {
    pub display_mode: DisplayMode,
    pub fps_overlay: bool,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GameSettingsUpdate {
    pub game_kind: GameKind,
    pub preferred_runner: String,
    pub dxvk_enabled: bool,
    pub dll_override: Option<String>,
    pub display_mode: DisplayMode,
    pub virtual_desktop: bool,
    pub gamescope_enabled: bool,
    pub resolution: String,
    pub gamescope_scaler: String,
    pub protondb_note: Option<String>,
    pub ddraw_override: bool,
    pub windows_version: String,
}

pub fn initialize() -> Result<(), String> {
    let connection = connection()?;
    connection
        .execute_batch(
            "
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
            ",
        )
        .map_err(|error| format!("Cannot initialize SQLite database: {error}"))?;

    migrate_column(
        &connection,
        "ALTER TABLE games ADD COLUMN last_played_at INTEGER",
    )?;
    migrate_column(
        &connection,
        "ALTER TABLE games ADD COLUMN display_mode TEXT NOT NULL DEFAULT 'windowed'",
    )?;
    migrate_column(
        &connection,
        "ALTER TABLE games ADD COLUMN fps_overlay INTEGER NOT NULL DEFAULT 0",
    )?;
    migrate_column(
        &connection,
        "ALTER TABLE games ADD COLUMN total_playtime_seconds INTEGER NOT NULL DEFAULT 0",
    )?;
    migrate_column(
        &connection,
        "ALTER TABLE games ADD COLUMN active_session_id INTEGER",
    )?;
    migrate_column(
        &connection,
        "ALTER TABLE games ADD COLUMN preferred_runner TEXT NOT NULL DEFAULT ''",
    )?;
    migrate_column(
        &connection,
        "ALTER TABLE games ADD COLUMN dxvk_enabled INTEGER NOT NULL DEFAULT 1",
    )?;
    migrate_column(
        &connection,
        "ALTER TABLE games ADD COLUMN dll_override TEXT",
    )?;
    migrate_column(
        &connection,
        "ALTER TABLE games ADD COLUMN virtual_desktop INTEGER NOT NULL DEFAULT 1",
    )?;
    migrate_column(
        &connection,
        "ALTER TABLE games ADD COLUMN gamescope_enabled INTEGER NOT NULL DEFAULT 0",
    )?;
    migrate_column(
        &connection,
        "ALTER TABLE games ADD COLUMN resolution TEXT NOT NULL DEFAULT 'auto'",
    )?;
    migrate_column(
        &connection,
        "ALTER TABLE games ADD COLUMN gamescope_scaler TEXT NOT NULL DEFAULT 'fit'",
    )?;
    migrate_column(
        &connection,
        "ALTER TABLE games ADD COLUMN protondb_note TEXT",
    )?;
    migrate_column(
        &connection,
        "ALTER TABLE games ADD COLUMN ddraw_override INTEGER NOT NULL DEFAULT 1",
    )?;
    migrate_column(
        &connection,
        "ALTER TABLE games ADD COLUMN windows_version TEXT NOT NULL DEFAULT ''",
    )?;
    migrate_column(
        &connection,
        "ALTER TABLE games ADD COLUMN library_type TEXT NOT NULL DEFAULT 'game'",
    )?;
    seed_default_settings(&connection)?;

    Ok(())
}

pub fn add_game(
    request: GameInstallRequest,
    selection: &RunnerSelection,
    prefix_path: Option<String>,
) -> Result<GameRecord, String> {
    initialize()?;

    let game_id = unique_game_id(&request.name)?;
    let install_dir = match request.install_dir {
        Some(path) if !path.trim().is_empty() => path,
        _ if matches!(
            request.game_kind,
            GameKind::WindowsExe | GameKind::WindowsMsi
        ) =>
        {
            Path::new(&request.installer_path)
                .parent()
                .map(|path| path.to_string_lossy().into_owned())
                .unwrap_or_else(|| {
                    runtime::game_install_dir(&game_id)
                        .map(|path| path.to_string_lossy().into_owned())
                        .unwrap_or_default()
                })
        }
        _ => runtime::game_install_dir(&game_id)?
            .to_string_lossy()
            .into_owned(),
    };
    fs::create_dir_all(&install_dir)
        .map_err(|error| format!("Cannot create game install directory: {error}"))?;

    let arguments_json = serde_json::to_string(&selection.arguments)
        .map_err(|error| format!("Cannot serialize runner arguments: {error}"))?;
    let defaults = game_defaults(
        &request.game_kind,
        &request.name,
        &install_dir,
        &selection.arguments,
    );
    let library_type = request
        .library_type
        .as_ref()
        .map(LibraryType::as_str)
        .unwrap_or("game");
    let detected_runner = if is_cncnet_game(&request.name, &install_dir, &selection.arguments) {
        "cncnet"
    } else {
        selection.runner.as_str()
    };
    let preferred_runner = request
        .preferred_runner
        .as_deref()
        .map(str::trim)
        .filter(|runner| {
            matches!(
                *runner,
                "wine" | "proton" | "steam-proton" | "openra" | "dosbox-x" | "cncnet"
            )
        })
        .unwrap_or(detected_runner);
    let connection = connection()?;
    connection
        .execute(
            "
            INSERT INTO games (
              game_id, name, game_kind, runner, installer_path, install_dir,
              prefix_path, executable, arguments_json, preferred_runner, dxvk_enabled,
              virtual_desktop, gamescope_enabled, resolution, gamescope_scaler,
              ddraw_override, windows_version, dll_override, library_type
            )
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19)
            ",
            params![
                game_id,
                request.name,
                request.game_kind.as_str(),
                detected_runner,
                request.installer_path,
                install_dir,
                prefix_path,
                selection.executable,
                arguments_json,
                preferred_runner,
                defaults.dxvk_enabled as i64,
                defaults.virtual_desktop as i64,
                defaults.gamescope_enabled as i64,
                defaults.resolution,
                defaults.gamescope_scaler,
                defaults.ddraw_override as i64,
                defaults.windows_version,
                defaults.dll_override,
                library_type,
            ],
        )
        .map_err(|error| format!("Cannot insert game record: {error}"))?;
    let inserted_id = connection.last_insert_rowid();
    connection
        .execute(
            "UPDATE games SET display_mode = ?2 WHERE id = ?1",
            params![inserted_id, defaults.display_mode],
        )
        .map_err(|error| format!("Cannot apply default display mode: {error}"))?;

    get_game(inserted_id)
}

pub fn list_games() -> Result<Vec<GameRecord>, String> {
    initialize()?;
    let connection = connection()?;
    let mut statement = connection
        .prepare(
            "
            SELECT g.id, g.game_id, g.name, g.game_kind, g.runner, g.installer_path, g.install_dir,
                   g.prefix_path, g.executable, g.arguments_json, g.created_at, g.last_played_at,
                   g.display_mode, g.fps_overlay, g.total_playtime_seconds, g.active_session_id,
                   m.cover_path, m.genre, m.release_year, m.description, g.preferred_runner,
                   g.dxvk_enabled, g.dll_override, g.virtual_desktop, g.gamescope_enabled, g.resolution,
                   g.protondb_note, g.ddraw_override, g.windows_version, g.gamescope_scaler, g.library_type
            FROM games g
            LEFT JOIN game_metadata m ON m.game_id = g.id
            ORDER BY g.created_at DESC, g.id DESC
            ",
        )
        .map_err(|error| format!("Cannot prepare game list query: {error}"))?;

    let rows = statement
        .query_map([], row_to_game)
        .map_err(|error| format!("Cannot query games: {error}"))?;

    let mut games = Vec::new();
    for row in rows {
        games.push(row.map_err(|error| format!("Cannot read game record: {error}"))?);
    }
    Ok(games)
}

pub fn upsert_steam_game(
    game: &SteamGame,
    proton_path: Option<&str>,
) -> Result<GameRecord, String> {
    initialize()?;
    let game_id = crate::steam::steam_game_id(&game.app_id);
    let executable = proton_path.map(ToString::to_string);
    let arguments_json = serde_json::to_string(&vec![game.install_dir.clone()])
        .map_err(|error| format!("Cannot serialize Steam launch arguments: {error}"))?;
    let connection = connection()?;

    connection
        .execute(
            "
            INSERT INTO games (
              game_id, name, game_kind, runner, installer_path, install_dir,
              prefix_path, executable, arguments_json, library_type
            )
            VALUES (?1, ?2, 'steam', 'steam-proton', ?3, ?4, NULL, ?5, ?6, 'game')
            ON CONFLICT(game_id) DO UPDATE SET
              name = excluded.name,
              runner = excluded.runner,
              installer_path = excluded.installer_path,
              install_dir = excluded.install_dir,
              executable = excluded.executable,
              arguments_json = excluded.arguments_json
            ",
            params![
                game_id,
                game.name,
                game.manifest_path,
                game.install_dir,
                executable,
                arguments_json
            ],
        )
        .map_err(|error| format!("Cannot sync Steam game record: {error}"))?;

    get_game_by_game_id(&game_id)
}

fn get_game(id: i64) -> Result<GameRecord, String> {
    let connection = connection()?;
    connection
        .query_row(
            "
            SELECT g.id, g.game_id, g.name, g.game_kind, g.runner, g.installer_path, g.install_dir,
                   g.prefix_path, g.executable, g.arguments_json, g.created_at, g.last_played_at,
                   g.display_mode, g.fps_overlay, g.total_playtime_seconds, g.active_session_id,
                   m.cover_path, m.genre, m.release_year, m.description, g.preferred_runner,
                   g.dxvk_enabled, g.dll_override, g.virtual_desktop, g.gamescope_enabled, g.resolution,
                   g.protondb_note, g.ddraw_override, g.windows_version, g.gamescope_scaler, g.library_type
            FROM games g
            LEFT JOIN game_metadata m ON m.game_id = g.id
            WHERE g.id = ?1
            ",
            params![id],
            row_to_game,
        )
        .map_err(|error| format!("Cannot read inserted game record: {error}"))
}

fn get_game_by_game_id(game_id: &str) -> Result<GameRecord, String> {
    let connection = connection()?;
    connection
        .query_row(
            "
            SELECT g.id, g.game_id, g.name, g.game_kind, g.runner, g.installer_path, g.install_dir,
                   g.prefix_path, g.executable, g.arguments_json, g.created_at, g.last_played_at,
                   g.display_mode, g.fps_overlay, g.total_playtime_seconds, g.active_session_id,
                   m.cover_path, m.genre, m.release_year, m.description, g.preferred_runner,
                   g.dxvk_enabled, g.dll_override, g.virtual_desktop, g.gamescope_enabled, g.resolution,
                   g.protondb_note, g.ddraw_override, g.windows_version, g.gamescope_scaler, g.library_type
            FROM games g
            LEFT JOIN game_metadata m ON m.game_id = g.id
            WHERE g.game_id = ?1
            ",
            params![game_id],
            row_to_game,
        )
        .map_err(|error| format!("Cannot read synced Steam game record: {error}"))
}

fn row_to_game(row: &rusqlite::Row<'_>) -> rusqlite::Result<GameRecord> {
    let arguments_json: String = row.get(9)?;
    let arguments: Vec<String> = serde_json::from_str(&arguments_json).unwrap_or_default();
    let install_dir: String = row.get(6)?;
    let name: String = row.get(2)?;
    let library_type = row
        .get::<_, String>(30)
        .ok()
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| "game".into());
    let manual_cover_path: Option<String> = row.get::<_, Option<String>>(16)?;
    let cover_path = if library_type == "windows-app" || library_type == "tool" {
        manual_cover_path
    } else {
        manual_cover_path.or_else(|| local_cover_path(&install_dir, &arguments))
    };
    let windows_version = row
        .get::<_, String>(28)
        .ok()
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| recommended_windows_version(&name, &install_dir, &arguments));
    let cncnet_installed = cncnet_client_path(&install_dir).exists();

    Ok(GameRecord {
        id: row.get(0)?,
        game_id: row.get(1)?,
        name: name.clone(),
        game_kind: row.get(3)?,
        library_type,
        runner: row.get(4)?,
        installer_path: row.get(5)?,
        install_dir,
        prefix_path: row.get(7)?,
        executable: row.get(8)?,
        arguments,
        created_at: row.get(10)?,
        last_played_at: row.get(11)?,
        display_mode: row.get(12)?,
        fps_overlay: row.get::<_, i64>(13)? == 1,
        total_playtime_seconds: row.get(14)?,
        active_session_id: row.get(15)?,
        cover_path,
        genre: row.get(17)?,
        release_year: row.get(18)?,
        description: row.get(19)?,
        preferred_runner: row
            .get::<_, String>(20)
            .ok()
            .filter(|value| !value.trim().is_empty())
            .unwrap_or_else(|| row.get::<_, String>(4).unwrap_or_else(|_| "wine".into())),
        dxvk_enabled: row.get::<_, i64>(21)? == 1,
        dll_override: row.get(22)?,
        virtual_desktop: row.get::<_, i64>(23)? == 1,
        gamescope_enabled: row.get::<_, i64>(24)? == 1,
        resolution: row.get(25)?,
        gamescope_scaler: row
            .get::<_, String>(29)
            .ok()
            .filter(|value| !value.trim().is_empty())
            .unwrap_or_else(|| "fit".into()),
        protondb_note: row.get(26)?,
        ddraw_override: row.get::<_, i64>(27)? == 1,
        windows_version,
        cncnet_installed,
    })
}

pub fn cncnet_client_path(install_dir: &str) -> std::path::PathBuf {
    Path::new(install_dir)
        .join("Resources")
        .join("clientogl.exe")
}

fn local_cover_path(install_dir: &str, arguments: &[String]) -> Option<String> {
    let mut roots = vec![Path::new(install_dir).to_path_buf()];
    for argument in arguments {
        let path = Path::new(argument);
        if path.is_file() {
            if let Some(parent) = path.parent() {
                roots.push(parent.to_path_buf());
            }
        }
    }

    for root in roots {
        if let Some(path) = find_cover_candidate(&root) {
            return Some(path.to_string_lossy().into_owned());
        }
    }

    None
}

fn find_cover_candidate(root: &Path) -> Option<std::path::PathBuf> {
    let entries = fs::read_dir(root)
        .ok()?
        .filter_map(Result::ok)
        .collect::<Vec<_>>();
    let mut files = entries
        .iter()
        .map(|entry| entry.path())
        .filter(|path| path.is_file() && is_cover_image(path))
        .collect::<Vec<_>>();

    files.sort_by_key(|path| {
        let name = path
            .file_name()
            .and_then(|value| value.to_str())
            .unwrap_or_default()
            .to_lowercase();
        if name == "launcher.bmp" || name == "launchermd.bmp" {
            0
        } else if name.contains("launcher") || name.contains("cover") || name.contains("icon") {
            1
        } else {
            2
        }
    });

    files.into_iter().next()
}

fn is_cover_image(path: &Path) -> bool {
    let Some(extension) = path.extension().and_then(|value| value.to_str()) else {
        return false;
    };
    matches!(
        extension.to_lowercase().as_str(),
        "png" | "jpg" | "jpeg" | "bmp" | "webp"
    )
}

fn recommended_windows_version(name: &str, install_dir: &str, arguments: &[String]) -> String {
    if is_serious_sam_game(name, install_dir, arguments) {
        "win7".into()
    } else if is_cncnet_launcher(name, install_dir, arguments) {
        "win10".into()
    } else if is_legacy_directdraw_game(name, install_dir, arguments) {
        "winxp".into()
    } else {
        "win10".into()
    }
}

fn game_defaults(
    game_kind: &GameKind,
    name: &str,
    install_dir: &str,
    arguments: &[String],
) -> GameDefaults {
    if matches!(game_kind, GameKind::WindowsExe)
        && is_serious_sam_game(name, install_dir, arguments)
    {
        return GameDefaults {
            windows_version: "win7",
            display_mode: "fullscreen",
            dxvk_enabled: true,
            virtual_desktop: true,
            gamescope_enabled: true,
            resolution: "1024x768",
            gamescope_scaler: "stretch",
            ddraw_override: false,
            dll_override: None,
        };
    }

    if is_popcap_game(name, install_dir, arguments) {
        return GameDefaults {
            windows_version: "win7",
            display_mode: "fullscreen",
            dxvk_enabled: true,
            virtual_desktop: true,
            gamescope_enabled: true,
            resolution: "800x600",
            gamescope_scaler: "stretch",
            ddraw_override: false,
            dll_override: None,
        };
    }

    if is_cncnet_launcher(name, install_dir, arguments) {
        return GameDefaults {
            windows_version: "win10",
            display_mode: "fullscreen",
            dxvk_enabled: true,
            virtual_desktop: false,
            gamescope_enabled: false,
            resolution: "auto",
            gamescope_scaler: "fit",
            ddraw_override: false,
            dll_override: None,
        };
    }

    let legacy_directdraw = is_legacy_directdraw_game(name, install_dir, arguments);
    if legacy_directdraw {
        return GameDefaults {
            windows_version: "winxp",
            display_mode: "fullscreen",
            dxvk_enabled: false,
            virtual_desktop: true,
            gamescope_enabled: true,
            resolution: "auto",
            gamescope_scaler: "stretch",
            ddraw_override: true,
            dll_override: Some("ddraw=n,b;dinput8=n,b"),
        };
    }

    GameDefaults {
        windows_version: "win10",
        display_mode: "windowed",
        dxvk_enabled: true,
        virtual_desktop: false,
        gamescope_enabled: false,
        resolution: "auto",
        gamescope_scaler: "fit",
        ddraw_override: false,
        dll_override: None,
    }
}

fn is_legacy_directdraw_game(name: &str, install_dir: &str, arguments: &[String]) -> bool {
    is_cnc_legacy_game(name, install_dir, arguments)
}

fn is_serious_sam_game(name: &str, install_dir: &str, arguments: &[String]) -> bool {
    let combined = searchable_game_text(name, install_dir, arguments);
    combined.contains("serioussam")
        || combined.contains("serioussamclassic")
        || combined.contains("samse")
        || combined.contains("samfe")
}

fn is_popcap_game(name: &str, install_dir: &str, arguments: &[String]) -> bool {
    let combined = searchable_game_text(name, install_dir, arguments);
    [
        "zuma",
        "bejeweled",
        "peggle",
        "insaniquarium",
        "feedingfrenzy",
        "plantsvszombies",
        "popcap",
    ]
    .iter()
    .any(|needle| combined.contains(needle))
}

fn is_cncnet_game(name: &str, install_dir: &str, arguments: &[String]) -> bool {
    is_cncnet_launcher(name, install_dir, arguments)
        || is_cnc_legacy_game(name, install_dir, arguments)
}

fn is_cncnet_launcher(name: &str, install_dir: &str, arguments: &[String]) -> bool {
    searchable_game_text(name, install_dir, arguments).contains("cncnet")
}

fn is_cnc_legacy_game(name: &str, install_dir: &str, arguments: &[String]) -> bool {
    let combined = searchable_game_text(name, install_dir, arguments);
    if combined.contains("cncnet") {
        return false;
    }

    [
        "ra2",
        "redalert",
        "redalert2",
        "yurisrevenge",
        "yuri",
        "tiberiansun",
        "commandandconquer",
    ]
    .iter()
    .any(|needle| combined.contains(needle))
}

fn searchable_game_text(name: &str, install_dir: &str, arguments: &[String]) -> String {
    format!(
        "{} {} {} {}",
        name,
        install_dir,
        arguments.join(" "),
        install_dir_file_hints(install_dir)
    )
    .to_lowercase()
    .replace([' ', '-', '_'], "")
}

fn install_dir_file_hints(install_dir: &str) -> String {
    let Ok(entries) = fs::read_dir(install_dir) else {
        return String::new();
    };

    entries
        .filter_map(Result::ok)
        .take(80)
        .filter_map(|entry| entry.file_name().to_str().map(ToString::to_string))
        .filter(|name| {
            let lower = name.to_ascii_lowercase();
            lower.ends_with(".exe")
                || lower.ends_with(".dll")
                || lower.ends_with(".ini")
                || lower.ends_with(".cfg")
                || lower.ends_with(".mix")
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn normalized_gamescope_scaler(value: &str) -> &'static str {
    match value.trim() {
        "stretch" => "stretch",
        "fill" => "fill",
        "integer" => "integer",
        "auto" => "auto",
        _ => "fit",
    }
}

pub fn get_game_by_id(id: i64) -> Result<GameRecord, String> {
    get_game(id)
}

pub fn mark_game_launched(id: i64, options: &GameModeOptions) -> Result<GameRecord, String> {
    let connection = connection()?;
    connection
        .execute(
            "UPDATE games SET active_session_id = NULL WHERE id = ?1 AND active_session_id IS NOT NULL",
            params![id],
        )
        .map_err(|error| format!("Cannot reset stale play session: {error}"))?;
    connection
        .execute(
            "INSERT INTO play_sessions (game_id) VALUES (?1)",
            params![id],
        )
        .map_err(|error| format!("Cannot create play session: {error}"))?;
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
            params![
                id,
                options.display_mode.as_str(),
                options.fps_overlay as i64,
                session_id
            ],
        )
        .map_err(|error| format!("Cannot update last played time: {error}"))?;
    get_game(id)
}

pub fn finish_play_session(id: i64) -> Result<GameRecord, String> {
    let game = get_game(id)?;
    let Some(session_id) = game.active_session_id else {
        return Ok(game);
    };

    let connection = connection()?;
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
        .map_err(|error| format!("Cannot finish play session: {error}"))?;
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
            params![id, session_id],
        )
        .map_err(|error| format!("Cannot update total playtime: {error}"))?;
    get_game(id)
}

pub fn clear_play_session(id: i64) -> Result<GameRecord, String> {
    let game = get_game(id)?;
    let Some(session_id) = game.active_session_id else {
        return Ok(game);
    };

    let connection = connection()?;
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
        .map_err(|error| format!("Cannot clear play session: {error}"))?;
    connection
        .execute(
            "UPDATE games SET active_session_id = NULL WHERE id = ?1",
            params![id],
        )
        .map_err(|error| format!("Cannot clear active play session: {error}"))?;
    get_game(id)
}

pub fn list_settings() -> Result<Vec<AppSetting>, String> {
    initialize()?;
    let connection = connection()?;
    let mut statement = connection
        .prepare("SELECT key, value FROM settings ORDER BY key")
        .map_err(|error| format!("Cannot prepare settings query: {error}"))?;
    let rows = statement
        .query_map([], |row| {
            Ok(AppSetting {
                key: row.get(0)?,
                value: row.get(1)?,
            })
        })
        .map_err(|error| format!("Cannot query settings: {error}"))?;

    let mut settings = Vec::new();
    for row in rows {
        settings.push(row.map_err(|error| format!("Cannot read setting: {error}"))?);
    }
    Ok(settings)
}

pub fn set_setting(key: String, value: String) -> Result<AppSetting, String> {
    initialize()?;
    let connection = connection()?;
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
        .map_err(|error| format!("Cannot write setting: {error}"))?;

    let mut statement = connection
        .prepare("SELECT key, value FROM settings WHERE key = ?1")
        .map_err(|error| format!("Cannot prepare setting lookup: {error}"))?;
    statement
        .query_row(params![key], |row| {
            Ok(AppSetting {
                key: row.get(0)?,
                value: row.get(1)?,
            })
        })
        .map_err(|error| format!("Cannot read setting: {error}"))
}

pub fn get_setting_value(key: &str) -> Result<Option<String>, String> {
    initialize()?;
    let connection = connection()?;
    match connection.query_row(
        "SELECT value FROM settings WHERE key = ?1",
        params![key],
        |row| row.get(0),
    ) {
        Ok(value) => Ok(Some(value)),
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
        Err(error) => Err(format!("Cannot read setting: {error}")),
    }
}

pub fn save_metadata(
    id: i64,
    cover_path: Option<String>,
    title: Option<String>,
    genre: Option<String>,
    release_year: Option<i64>,
    description: Option<String>,
    source: String,
) -> Result<GameRecord, String> {
    initialize()?;
    let connection = connection()?;
    connection
        .execute(
            "
            INSERT INTO game_metadata (
              game_id, cover_path, title, genre, release_year, description, source, updated_at
            )
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, unixepoch())
            ON CONFLICT(game_id) DO UPDATE SET
              cover_path = COALESCE(excluded.cover_path, game_metadata.cover_path),
              title = COALESCE(excluded.title, game_metadata.title),
              genre = COALESCE(excluded.genre, game_metadata.genre),
              release_year = COALESCE(excluded.release_year, game_metadata.release_year),
              description = COALESCE(excluded.description, game_metadata.description),
              source = excluded.source,
              updated_at = excluded.updated_at
            ",
            params![
                id,
                cover_path,
                title,
                genre,
                release_year,
                description,
                source
            ],
        )
        .map_err(|error| format!("Cannot save game metadata: {error}"))?;
    get_game(id)
}

pub fn update_game_mode(id: i64, options: &GameModeOptions) -> Result<GameRecord, String> {
    let connection = connection()?;
    connection
        .execute(
            "UPDATE games SET display_mode = ?2, fps_overlay = ?3 WHERE id = ?1",
            params![
                id,
                options.display_mode.as_str(),
                options.fps_overlay as i64
            ],
        )
        .map_err(|error| format!("Cannot update game mode options: {error}"))?;
    get_game(id)
}

pub fn update_game_settings(id: i64, settings: &GameSettingsUpdate) -> Result<GameRecord, String> {
    let connection = connection()?;
    let dxvk_enabled = settings.dxvk_enabled && !settings.ddraw_override;
    connection
        .execute(
            "
            UPDATE games
            SET preferred_runner = ?2,
                dxvk_enabled = ?3,
                dll_override = NULLIF(?4, ''),
                display_mode = ?5,
                virtual_desktop = ?6,
                gamescope_enabled = ?7,
                resolution = ?8,
                protondb_note = NULLIF(?9, ''),
                ddraw_override = ?10,
                windows_version = ?11,
                game_kind = ?12,
                gamescope_scaler = ?13
            WHERE id = ?1
            ",
            params![
                id,
                settings.preferred_runner.trim(),
                dxvk_enabled as i64,
                settings.dll_override.clone().unwrap_or_default().trim(),
                settings.display_mode.as_str(),
                settings.virtual_desktop as i64,
                settings.gamescope_enabled as i64,
                settings.resolution.trim(),
                settings.protondb_note.clone().unwrap_or_default().trim(),
                settings.ddraw_override as i64,
                settings.windows_version.trim(),
                settings.game_kind.as_str(),
                normalized_gamescope_scaler(&settings.gamescope_scaler),
            ],
        )
        .map_err(|error| format!("Cannot update game settings: {error}"))?;
    get_game(id)
}

pub fn mark_cncnet_installed(id: i64) -> Result<GameRecord, String> {
    let connection = connection()?;
    connection
        .execute(
            "
            UPDATE games
            SET runner = 'cncnet',
                preferred_runner = 'cncnet',
                game_kind = 'cncnet'
            WHERE id = ?1
            ",
            params![id],
        )
        .map_err(|error| format!("Cannot update CnCNet runner: {error}"))?;
    get_game(id)
}

pub fn remove_game(id: i64, remove_files: bool) -> Result<(), String> {
    let game = get_game(id)?;
    let connection = connection()?;
    connection
        .execute("DELETE FROM games WHERE id = ?1", params![id])
        .map_err(|error| format!("Cannot remove game record: {error}"))?;

    if remove_files {
        let install_dir = Path::new(&game.install_dir);
        if install_dir.exists() {
            fs::remove_dir_all(install_dir)
                .map_err(|error| format!("Cannot remove game install directory: {error}"))?;
        }
    }

    Ok(())
}

fn connection() -> Result<Connection, String> {
    let paths = runtime::runtime_paths()?;
    if let Some(parent) = Path::new(&paths.database_path).parent() {
        fs::create_dir_all(parent)
            .map_err(|error| format!("Cannot create database directory: {error}"))?;
    }
    Connection::open(paths.database_path)
        .map_err(|error| format!("Cannot open SQLite database: {error}"))
}

fn migrate_column(connection: &Connection, sql: &str) -> Result<(), String> {
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
        .map_err(|error| format!("Cannot migrate SQLite database: {error}"))
}

fn seed_default_settings(connection: &Connection) -> Result<(), String> {
    for (key, value) in [
        ("default_display_mode", "windowed"),
        ("library_runner_filter", "all"),
        ("fps_overlay", "false"),
        ("auto_sync_steam", "false"),
        ("steamgriddb_api_key", ""),
    ] {
        connection
            .execute(
                "INSERT OR IGNORE INTO settings (key, value) VALUES (?1, ?2)",
                params![key, value],
            )
            .map_err(|error| format!("Cannot seed default setting: {error}"))?;
    }
    Ok(())
}

fn unique_game_id(name: &str) -> Result<String, String> {
    let base = runtime::sanitize_game_id(name);
    if base.is_empty() {
        return Err("Game name must include at least one letter or number.".into());
    }

    let connection = connection()?;
    let mut candidate = base.clone();
    let mut suffix = 2;

    loop {
        let count: i64 = connection
            .query_row(
                "SELECT COUNT(*) FROM games WHERE game_id = ?1",
                params![candidate],
                |row| row.get(0),
            )
            .map_err(|error| format!("Cannot check game id: {error}"))?;

        if count == 0 {
            return Ok(candidate);
        }

        candidate = format!("{base}-{suffix}");
        suffix += 1;
    }
}
