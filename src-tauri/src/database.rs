use crate::runtime::{self, GameKind, RunnerKind, RunnerSelection};
use crate::steam::SteamGame;
pub use ardali_core::{DisplayMode, LibraryType, PrefixMode};
use serde::{Deserialize, Serialize};
use std::{
    env, fs,
    io::Write,
    path::{Path, PathBuf},
    process::{Command, Stdio},
};

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
    pub gamemode_enabled: bool,
    pub steam_launch_options: String,
    pub source_available: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppSetting {
    pub key: String,
    pub value: String,
}

const ALLOWED_SETTING_KEYS: &[&str] = &[
    "default_display_mode",
    "library_runner_filter",
    "fps_overlay",
    "auto_sync_steam",
    "steamgriddb_api_key",
];
const SENSITIVE_SETTING_KEYS: &[&str] = &["steamgriddb_api_key"];

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
    pub gamemode_enabled: bool,
    pub steam_launch_options: String,
}

pub fn initialize() -> Result<(), String> {
    ardali_storage::initialize(database_path()?).map_err(|error| error.to_string())
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
                "wine" | "steam" | "proton" | "openra" | "dosbox-x" | "cncnet"
            )
        })
        .unwrap_or(detected_runner);
    let connection = connection()?;
    let inserted_id = ardali_storage::insert_game(
        &connection,
        &ardali_storage::NewGame {
            game_id: &game_id,
            name: &request.name,
            game_kind: request.game_kind.as_str(),
            runner: detected_runner,
            installer_path: &request.installer_path,
            install_dir: &install_dir,
            prefix_path: prefix_path.as_deref(),
            executable: selection.executable.as_deref(),
            arguments_json: &arguments_json,
            preferred_runner,
            dxvk_enabled: defaults.dxvk_enabled,
            virtual_desktop: defaults.virtual_desktop,
            gamescope_enabled: defaults.gamescope_enabled,
            resolution: defaults.resolution,
            gamescope_scaler: defaults.gamescope_scaler,
            ddraw_override: defaults.ddraw_override,
            windows_version: defaults.windows_version,
            dll_override: defaults.dll_override,
            library_type,
            display_mode: defaults.display_mode,
            gamemode_enabled: false,
        },
    )
    .map_err(|error| error.to_string())?;

    get_game(inserted_id)
}

pub fn list_games() -> Result<Vec<GameRecord>, String> {
    initialize()?;
    let connection = connection()?;
    ardali_storage::list_games(&connection)
        .map(|games| games.into_iter().map(stored_game_to_record).collect())
        .map_err(|error| error.to_string())
}

pub fn upsert_steam_game(
    game: &SteamGame,
    launcher: Option<&crate::steam::SteamLauncher>,
) -> Result<GameRecord, String> {
    initialize()?;
    let game_id = crate::steam::steam_game_id(&game.app_id);
    let executable = launcher.map(|launcher| launcher.executable.clone());
    let arguments = launcher
        .map(|launcher| crate::steam::launch_arguments(launcher, &game.app_id))
        .unwrap_or_else(|| vec!["-applaunch".into(), game.app_id.clone()]);
    let arguments_json = serde_json::to_string(&arguments)
        .map_err(|error| format!("Cannot serialize Steam launch arguments: {error}"))?;
    let connection = connection()?;

    ardali_storage::upsert_game(
        &connection,
        &ardali_storage::NewGame {
            game_id: &game_id,
            name: &game.name,
            game_kind: GameKind::Steam.as_str(),
            runner: RunnerKind::Steam.as_str(),
            installer_path: &game.manifest_path,
            install_dir: &game.install_dir,
            prefix_path: None,
            executable: executable.as_deref(),
            arguments_json: &arguments_json,
            preferred_runner: RunnerKind::Steam.as_str(),
            dxvk_enabled: true,
            virtual_desktop: false,
            gamescope_enabled: false,
            resolution: "auto",
            gamescope_scaler: "fit",
            ddraw_override: false,
            windows_version: "",
            dll_override: None,
            library_type: LibraryType::Game.as_str(),
            display_mode: DisplayMode::Windowed.as_str(),
            gamemode_enabled: false,
        },
    )
    .map_err(|error| error.to_string())?;

    let record = get_game_by_game_id(&game_id)?;
    if record.cover_path.is_none() {
        if let Some(cover_path) = game.cover_path.as_deref() {
            ardali_storage::save_metadata(
                &connection,
                record.id,
                Some(cover_path),
                None,
                None,
                None,
                None,
                "steam-local",
            )
            .map_err(|error| error.to_string())?;
            return get_game_by_game_id(&game_id);
        }
    }
    Ok(record)
}

pub fn reconcile_steam_games(games: &[SteamGame]) -> Result<(), String> {
    initialize()?;
    let available = games
        .iter()
        .filter(|game| game.installed)
        .map(|game| crate::steam::steam_game_id(&game.app_id))
        .collect::<Vec<_>>();
    let mut connection = connection()?;
    ardali_storage::reconcile_steam_games(&mut connection, &available)
        .map_err(|error| error.to_string())
}

fn get_game(id: i64) -> Result<GameRecord, String> {
    let connection = connection()?;
    ardali_storage::get_game(&connection, id)
        .map(stored_game_to_record)
        .map_err(|error| error.to_string())
}

fn get_game_by_game_id(game_id: &str) -> Result<GameRecord, String> {
    let connection = connection()?;
    ardali_storage::get_game_by_game_id(&connection, game_id)
        .map(stored_game_to_record)
        .map_err(|error| error.to_string())
}

fn stored_game_to_record(game: ardali_storage::StoredGame) -> GameRecord {
    let arguments_json = game.arguments_json;
    let arguments: Vec<String> = serde_json::from_str(&arguments_json).unwrap_or_default();
    let install_dir = game.install_dir;
    let name = game.name;
    let library_type = Some(game.library_type)
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| "game".into());
    let manual_cover_path = game.cover_path;
    let cover_path = if library_type == "windows-app" || library_type == "tool" {
        manual_cover_path
    } else {
        manual_cover_path.or_else(|| local_cover_path(&install_dir, &arguments))
    };
    let windows_version = Some(game.windows_version)
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| recommended_windows_version(&name, &install_dir, &arguments));
    let cncnet_installed = cncnet_client_path(&install_dir).exists();

    GameRecord {
        id: game.id,
        game_id: game.game_id,
        name: name.clone(),
        game_kind: game.game_kind,
        library_type,
        runner: game.runner.clone(),
        installer_path: game.installer_path,
        install_dir,
        prefix_path: game.prefix_path,
        executable: game.executable,
        arguments,
        created_at: game.created_at,
        last_played_at: game.last_played_at,
        display_mode: game.display_mode,
        fps_overlay: game.fps_overlay,
        total_playtime_seconds: game.total_playtime_seconds,
        active_session_id: game.active_session_id,
        cover_path,
        genre: game.genre,
        release_year: game.release_year,
        description: game.description,
        preferred_runner: Some(game.preferred_runner)
            .filter(|value| !value.trim().is_empty())
            .unwrap_or(game.runner),
        dxvk_enabled: game.dxvk_enabled,
        dll_override: game.dll_override,
        virtual_desktop: game.virtual_desktop,
        gamescope_enabled: game.gamescope_enabled,
        resolution: game.resolution,
        gamescope_scaler: Some(game.gamescope_scaler)
            .filter(|value| !value.trim().is_empty())
            .unwrap_or_else(|| "fit".into()),
        protondb_note: game.protondb_note,
        ddraw_override: game.ddraw_override,
        windows_version,
        cncnet_installed,
        gamemode_enabled: game.gamemode_enabled,
        steam_launch_options: game.steam_launch_options,
        source_available: game.source_available,
    }
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
    ardali_storage::start_play_session(
        &connection,
        id,
        options.display_mode.as_str(),
        options.fps_overlay,
    )
    .map_err(|error| error.to_string())?;
    get_game(id)
}

pub fn finish_play_session(id: i64) -> Result<GameRecord, String> {
    let game = get_game(id)?;
    let Some(session_id) = game.active_session_id else {
        return Ok(game);
    };

    let connection = connection()?;
    ardali_storage::finish_play_session(&connection, id, session_id)
        .map_err(|error| error.to_string())?;
    get_game(id)
}

pub fn clear_play_session(id: i64) -> Result<GameRecord, String> {
    let game = get_game(id)?;
    let Some(session_id) = game.active_session_id else {
        return Ok(game);
    };

    let connection = connection()?;
    ardali_storage::clear_play_session(&connection, id, session_id)
        .map_err(|error| error.to_string())?;
    get_game(id)
}

pub fn list_settings() -> Result<Vec<AppSetting>, String> {
    initialize()?;
    let connection = connection()?;
    let mut settings = ardali_storage::list_settings(&connection)
        .map(|settings| {
            settings
                .into_iter()
                .flat_map(public_settings)
                .collect::<Vec<_>>()
        })
        .map_err(|error| error.to_string())?;
    if let Some(configured) = settings
        .iter_mut()
        .find(|setting| setting.key == "steamgriddb_api_key_configured")
    {
        configured.value = get_setting_value("steamgriddb_api_key")?
            .is_some_and(|value| !value.trim().is_empty())
            .to_string();
    }
    Ok(settings)
}

pub fn set_setting(key: String, value: String) -> Result<AppSetting, String> {
    if !ALLOWED_SETTING_KEYS.contains(&key.as_str()) {
        return Err(format!("Unsupported setting key: {key}"));
    }
    initialize()?;
    let connection = connection()?;
    if SENSITIVE_SETTING_KEYS.contains(&key.as_str()) && store_keyring_secret(&key, &value)? {
        return ardali_storage::set_setting(&connection, &key, "")
            .map(|setting| public_settings(setting).next().expect("setting response"))
            .map_err(|error| error.to_string());
    }
    ardali_storage::set_setting(&connection, &key, &value)
        .map(|setting| public_settings(setting).next().expect("setting response"))
        .map_err(|error| error.to_string())
}

fn public_settings(setting: ardali_storage::StoredSetting) -> impl Iterator<Item = AppSetting> {
    let sensitive = SENSITIVE_SETTING_KEYS.contains(&setting.key.as_str());
    let configured = sensitive && !setting.value.trim().is_empty();
    let mut settings = vec![AppSetting {
        key: setting.key.clone(),
        value: if sensitive {
            String::new()
        } else {
            setting.value
        },
    }];
    if sensitive {
        settings.push(AppSetting {
            key: format!("{}_configured", setting.key),
            value: configured.to_string(),
        });
    }
    settings.into_iter()
}

pub fn get_setting_value(key: &str) -> Result<Option<String>, String> {
    initialize()?;
    if SENSITIVE_SETTING_KEYS.contains(&key) {
        if let Some(secret) = lookup_keyring_secret(key)? {
            return Ok(Some(secret));
        }
    }
    let connection = connection()?;
    let value =
        ardali_storage::get_setting_value(&connection, key).map_err(|error| error.to_string())?;
    if SENSITIVE_SETTING_KEYS.contains(&key) {
        if let Some(secret) = value.as_deref().filter(|value| !value.trim().is_empty()) {
            if store_keyring_secret(key, secret)? {
                ardali_storage::set_setting(&connection, key, "")
                    .map_err(|error| error.to_string())?;
            }
        }
    }
    Ok(value)
}

fn secret_tool() -> Option<PathBuf> {
    env::var_os("PATH")?
        .to_string_lossy()
        .split(':')
        .map(|directory| Path::new(directory).join("secret-tool"))
        .find(|path| path.is_file())
}

fn lookup_keyring_secret(key: &str) -> Result<Option<String>, String> {
    let Some(tool) = secret_tool() else {
        return Ok(None);
    };
    let output = Command::new(tool)
        .args(["lookup", "service", "ardali-gaming", "key", key])
        .output()
        .map_err(|error| format!("Secret Service okunamadı: {error}"))?;
    if !output.status.success() {
        return Ok(None);
    }
    Ok(String::from_utf8(output.stdout)
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty()))
}

fn store_keyring_secret(key: &str, value: &str) -> Result<bool, String> {
    let Some(tool) = secret_tool() else {
        return Ok(false);
    };
    if value.is_empty() {
        let _ = Command::new(tool)
            .args(["clear", "service", "ardali-gaming", "key", key])
            .status();
        return Ok(true);
    }
    let mut child = Command::new(tool)
        .args([
            "store",
            "--label",
            "ArDali Gaming",
            "service",
            "ardali-gaming",
            "key",
            key,
        ])
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|error| format!("Secret Service başlatılamadı: {error}"))?;
    child
        .stdin
        .as_mut()
        .ok_or_else(|| "Secret Service girişi açılamadı".to_string())?
        .write_all(value.as_bytes())
        .map_err(|error| format!("Secret Service yazılamadı: {error}"))?;
    let output = child
        .wait_with_output()
        .map_err(|error| format!("Secret Service beklenemedi: {error}"))?;
    Ok(output.status.success())
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
    ardali_storage::save_metadata(
        &connection,
        id,
        cover_path.as_deref(),
        title.as_deref(),
        genre.as_deref(),
        release_year,
        description.as_deref(),
        &source,
    )
    .map_err(|error| error.to_string())?;
    get_game(id)
}

pub fn update_game_mode(id: i64, options: &GameModeOptions) -> Result<GameRecord, String> {
    let connection = connection()?;
    ardali_storage::update_game_mode(
        &connection,
        id,
        options.display_mode.as_str(),
        options.fps_overlay,
    )
    .map_err(|error| error.to_string())?;
    get_game(id)
}

pub fn update_game_settings(id: i64, settings: &GameSettingsUpdate) -> Result<GameRecord, String> {
    let connection = connection()?;
    let dxvk_enabled = settings.dxvk_enabled && !settings.ddraw_override;
    ardali_storage::update_game_settings(
        &connection,
        id,
        &ardali_storage::GameSettings {
            preferred_runner: settings.preferred_runner.trim(),
            dxvk_enabled,
            dll_override: settings.dll_override.as_deref().map(str::trim),
            display_mode: settings.display_mode.as_str(),
            virtual_desktop: settings.virtual_desktop,
            gamescope_enabled: settings.gamescope_enabled,
            resolution: settings.resolution.trim(),
            protondb_note: settings.protondb_note.as_deref().map(str::trim),
            ddraw_override: settings.ddraw_override,
            windows_version: settings.windows_version.trim(),
            game_kind: settings.game_kind.as_str(),
            gamescope_scaler: normalized_gamescope_scaler(&settings.gamescope_scaler),
            gamemode_enabled: settings.gamemode_enabled,
            steam_launch_options: settings.steam_launch_options.trim(),
        },
    )
    .map_err(|error| error.to_string())?;
    get_game(id)
}

pub fn mark_cncnet_installed(id: i64) -> Result<GameRecord, String> {
    let connection = connection()?;
    ardali_storage::mark_cncnet_installed(&connection, id).map_err(|error| error.to_string())?;
    get_game(id)
}

pub fn remove_game(id: i64, remove_files: bool) -> Result<(), String> {
    let game = get_game(id)?;
    let connection = connection()?;
    ardali_storage::delete_game(&connection, id).map_err(|error| error.to_string())?;

    if remove_files {
        let install_dir = Path::new(&game.install_dir);
        if install_dir.exists() {
            fs::remove_dir_all(install_dir)
                .map_err(|error| format!("Cannot remove game install directory: {error}"))?;
        }
    }

    Ok(())
}

fn connection() -> Result<ardali_storage::StorageConnection, String> {
    ardali_storage::open(database_path()?).map_err(|error| error.to_string())
}

fn database_path() -> Result<String, String> {
    runtime::runtime_paths().map(|paths| paths.database_path)
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
        if !ardali_storage::game_id_exists(&connection, &candidate)
            .map_err(|error| error.to_string())?
        {
            return Ok(candidate);
        }

        candidate = format!("{base}-{suffix}");
        suffix += 1;
    }
}

#[cfg(test)]
mod setting_security_tests {
    use super::public_settings;
    use ardali_storage::StoredSetting;

    #[test]
    fn redacts_sensitive_settings_and_reports_configuration_state() {
        let settings = public_settings(StoredSetting {
            key: "steamgriddb_api_key".into(),
            value: "secret-value".into(),
        })
        .collect::<Vec<_>>();

        assert_eq!(settings.len(), 2);
        assert_eq!(settings[0].key, "steamgriddb_api_key");
        assert!(settings[0].value.is_empty());
        assert_eq!(settings[1].key, "steamgriddb_api_key_configured");
        assert_eq!(settings[1].value, "true");
    }

    #[test]
    fn keeps_non_sensitive_settings_visible() {
        let settings = public_settings(StoredSetting {
            key: "fps_overlay".into(),
            value: "true".into(),
        })
        .collect::<Vec<_>>();

        assert_eq!(settings.len(), 1);
        assert_eq!(settings[0].value, "true");
    }
}
