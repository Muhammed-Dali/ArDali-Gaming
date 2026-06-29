mod compatibility;
mod database;
mod metadata;
mod runtime;
mod steam;
mod updates;

use compatibility::{CompatibilitySettings, ProtonDbSummary, TroubleshootingReport};
use database::{
    AppSetting, DisplayMode, GameInstallRequest, GameModeOptions, GameRecord, GameSettingsUpdate,
    LibraryType, PrefixMode,
};
use metadata::MetadataResult;
use runtime::{
    DownloadResult, EmulatorStatus, GameKind, PrefixInfo, RunnerKind, RunnerSelection,
    RuntimeStatus,
};
use serde_json::Value;
use std::{
    collections::hash_map::DefaultHasher,
    env, fs,
    hash::{Hash, Hasher},
    io::{BufRead, BufReader},
    path::{Path, PathBuf},
    process::{Child, Command, Stdio},
    thread,
    time::{Duration, SystemTime},
};
use steam::SteamScan;
use tauri::{AppHandle, Emitter, Manager, WebviewUrl, WebviewWindowBuilder, WindowEvent};
use updates::{ComponentUpdate, ComponentUpdateRequest};

#[derive(Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct GameProcessEvent {
    id: i64,
    name: String,
    status: String,
}

#[derive(Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct WindowsFilePreview {
    name: String,
    path: String,
    kind: String,
    icon_path: Option<String>,
}

#[derive(Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct CncNetInstallProgress {
    id: i64,
    percent: u8,
    status: String,
}

#[derive(Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct FullscreenToolStatus {
    kdotool: bool,
    wmctrl: bool,
    xdotool: bool,
    gamescope: bool,
    has_any_tool: bool,
    has_recommended_tool: bool,
    session_type: String,
    desktop_environment: String,
    recommended_tool: String,
    install_label: String,
    warning: String,
}

struct LaunchCommand {
    executable: String,
    args: Vec<String>,
}

struct StagedInstaller {
    host_path: PathBuf,
    launch_path: String,
    working_dir: PathBuf,
}

#[tauri::command]
fn initialize_runtime(app: AppHandle) -> Result<RuntimeStatus, String> {
    database::initialize()?;
    runtime::initialize(&app)
}

#[tauri::command]
fn download_portable_runner(
    app: AppHandle,
    kind: RunnerKind,
    url: String,
    file_name: Option<String>,
) -> Result<DownloadResult, String> {
    runtime::download_runner(&app, kind, url, file_name)
}

#[tauri::command]
fn initialize_emulators(app: AppHandle) -> Result<EmulatorStatus, String> {
    runtime::initialize_emulators(&app)
}

#[tauri::command]
fn download_portable_emulator(
    app: AppHandle,
    kind: RunnerKind,
    url: String,
    file_name: Option<String>,
) -> Result<DownloadResult, String> {
    runtime::download_emulator(&app, kind, url, file_name)
}

#[tauri::command]
fn select_game_runner(
    game_kind: GameKind,
    target_path: Option<String>,
) -> Result<RunnerSelection, String> {
    runtime::select_runner(game_kind, target_path)
}

#[tauri::command]
fn create_wine_prefix(app: AppHandle, game_id: String, name: String) -> Result<PrefixInfo, String> {
    runtime::create_prefix(&app, game_id, name)
}

#[tauri::command]
fn install_dxvk_vkd3d(app: AppHandle, prefix_id: String) -> Result<PrefixInfo, String> {
    runtime::install_graphics_components(&app, prefix_id)
}

#[tauri::command]
fn add_game_installation(
    app: AppHandle,
    request: GameInstallRequest,
) -> Result<GameRecord, String> {
    database::initialize()?;
    validate_install_request(&request, false)?;
    let selection = runtime::select_runner(
        request.game_kind.clone(),
        Some(request.installer_path.clone()),
    )?;

    let prefix = match request.game_kind {
        GameKind::WindowsExe | GameKind::WindowsMsi => {
            let (prefix_id, prefix_name) = prefix_identity(&request);
            let prefix = runtime::create_prefix(&app, prefix_id, prefix_name)?;
            let _ = runtime::install_graphics_components(&app, prefix.id.clone());
            Some(prefix.wineprefix)
        }
        _ => None,
    };

    let record = database::add_game(request, &selection, prefix)?;
    apply_windows_version(&app, &record)?;
    runtime::emit_backend_log(
        &app,
        "info",
        &format!("Game added to SQLite library: {}", record.name),
    )?;
    Ok(record)
}

#[tauri::command]
fn pick_file() -> Result<Option<String>, String> {
    match pick_file_with_portal() {
        Ok(value) => Ok(value),
        Err(_) => pick_path(false),
    }
}

#[tauri::command]
fn pick_folder() -> Result<Option<String>, String> {
    pick_path(true)
}

#[tauri::command]
fn windows_file_preview(path: String) -> Result<WindowsFilePreview, String> {
    let file_path = PathBuf::from(&path);
    if !file_path.is_file() {
        return Err("Seçilen yol dosya değil".into());
    }
    if !has_extension(&file_path, "exe") && !has_extension(&file_path, "msi") {
        return Err("Windows EXE veya MSI dosyası seçmelisin".into());
    }

    let name = file_path
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or("Windows dosyası")
        .to_string();
    let icon_path = if has_extension(&file_path, "exe") {
        extract_windows_icon_preview(&file_path).ok()
    } else {
        None
    };

    Ok(WindowsFilePreview {
        name,
        path,
        kind: if has_extension(&file_path, "msi") {
            "MSI".into()
        } else {
            "EXE".into()
        },
        icon_path,
    })
}

#[tauri::command]
fn fullscreen_tool_status() -> FullscreenToolStatus {
    let kdotool = command_exists("kdotool");
    let wmctrl = command_exists("wmctrl");
    let xdotool = command_exists("xdotool");
    let gamescope = command_exists("gamescope");
    let session_type = std::env::var("XDG_SESSION_TYPE").unwrap_or_else(|_| "unknown".into());
    let desktop_environment = std::env::var("XDG_CURRENT_DESKTOP")
        .or_else(|_| std::env::var("DESKTOP_SESSION"))
        .unwrap_or_else(|_| "unknown".into());
    let recommended_tool = recommended_fullscreen_tool(&session_type, &desktop_environment);
    let has_recommended_tool = match recommended_tool.as_str() {
        "kdotool" => kdotool,
        "wmctrl/xdotool" => wmctrl || xdotool,
        _ => kdotool || wmctrl || xdotool,
    };
    let install_label = match recommended_tool.as_str() {
        "kdotool" => "kdotool Kur",
        "wmctrl/xdotool" => "X11 Tam Ekran Aracı Kur",
        _ => "Tam Ekran Aracı Kur",
    }
    .to_string();
    let warning = fullscreen_tool_warning(&session_type, &desktop_environment, &recommended_tool);
    FullscreenToolStatus {
        kdotool,
        wmctrl,
        xdotool,
        gamescope,
        has_any_tool: kdotool || wmctrl || xdotool,
        has_recommended_tool,
        session_type,
        desktop_environment,
        recommended_tool,
        install_label,
        warning,
    }
}

#[tauri::command]
async fn install_kdotool() -> Result<FullscreenToolStatus, String> {
    tauri::async_runtime::spawn_blocking(install_kdotool_blocking)
        .await
        .map_err(|error| format!("kdotool kurulum görevi tamamlanamadı: {error}"))?
}

#[tauri::command]
async fn install_gamescope() -> Result<FullscreenToolStatus, String> {
    tauri::async_runtime::spawn_blocking(install_gamescope_blocking)
        .await
        .map_err(|error| format!("Gamescope kurulum görevi tamamlanamadı: {error}"))?
}

#[tauri::command]
async fn remove_gamescope() -> Result<FullscreenToolStatus, String> {
    tauri::async_runtime::spawn_blocking(remove_gamescope_blocking)
        .await
        .map_err(|error| format!("Gamescope kaldırma görevi tamamlanamadı: {error}"))?
}

#[tauri::command]
async fn remove_fullscreen_tool() -> Result<FullscreenToolStatus, String> {
    tauri::async_runtime::spawn_blocking(remove_fullscreen_tool_blocking)
        .await
        .map_err(|error| format!("Tam ekran aracı kaldırma görevi tamamlanamadı: {error}"))?
}

fn install_gamescope_blocking() -> Result<FullscreenToolStatus, String> {
    if command_exists("gamescope") {
        return Ok(fullscreen_tool_status());
    }

    run_script_in_system_terminal("ArDali Gaming - Gamescope kur", gamescope_install_script())?;

    let refreshed = fullscreen_tool_status();
    if !refreshed.gamescope {
        return Err("Gamescope kurulumu tamamlandı ama gamescope komutu bulunamadı".into());
    }

    Ok(refreshed)
}

fn remove_gamescope_blocking() -> Result<FullscreenToolStatus, String> {
    if !command_exists("gamescope") {
        return Ok(fullscreen_tool_status());
    }

    run_script_in_system_terminal(
        "ArDali Gaming - Gamescope kaldır",
        gamescope_uninstall_script(),
    )?;

    Ok(fullscreen_tool_status())
}

fn install_kdotool_blocking() -> Result<FullscreenToolStatus, String> {
    let status = fullscreen_tool_status();
    if status.has_recommended_tool {
        return Ok(fullscreen_tool_status());
    }

    match status.recommended_tool.as_str() {
        "kdotool" => install_kdotool_for_wayland()?,
        "wmctrl/xdotool" => install_x11_fullscreen_tools()?,
        _ => {
            return Err(format!(
                "{} masaüstünde otomatik pencere aracı desteklenmiyor",
                status.desktop_environment
            ))
        }
    }

    let refreshed = fullscreen_tool_status();
    if !refreshed.has_recommended_tool {
        return Err(format!(
            "{} kurulumu tamamlandı ama araç bulunamadı",
            status.recommended_tool
        ));
    }

    Ok(refreshed)
}

fn remove_fullscreen_tool_blocking() -> Result<FullscreenToolStatus, String> {
    let status = fullscreen_tool_status();
    match status.recommended_tool.as_str() {
        "kdotool" => run_script_in_system_terminal(
            "ArDali Gaming - kdotool kaldır",
            kdotool_uninstall_script(),
        )?,
        "wmctrl/xdotool" => run_script_in_system_terminal(
            "ArDali Gaming - X11 araçlarını kaldır",
            x11_fullscreen_tool_uninstall_script(),
        )?,
        _ => return Ok(status),
    }

    Ok(fullscreen_tool_status())
}

#[tauri::command]
fn run_game_installer(app: AppHandle, request: GameInstallRequest) -> Result<(), String> {
    validate_install_request(&request, false)?;
    let wine = wine_executable()?;

    let install_dir = installer_working_dir(&request);
    std::fs::create_dir_all(&install_dir)
        .map_err(|error| format!("Kurulum klasörü oluşturulamadı: {error}"))?;
    let (prefix_id, prefix_name) = prefix_identity(&request);
    let prefix = runtime::create_prefix(&app, prefix_id, prefix_name)?;
    stop_wine_processes(&app, &wine, &prefix.wineprefix);
    let staged_installer =
        stage_installer_for_wine(&app, &request.installer_path, &prefix.wineprefix)?;
    let scan_started_at = SystemTime::now();
    let mut command = Command::new(&wine);
    match request.game_kind {
        GameKind::WindowsMsi => {
            command.args(["msiexec", "/i"]);
            command.arg(&staged_installer.launch_path);
        }
        _ => {
            command.arg(&staged_installer.launch_path);
        }
    }
    command.env("WINEPREFIX", &prefix.wineprefix);
    command.env("WINEARCH", "win64");
    command.current_dir(&staged_installer.working_dir);
    let mut child = match command.spawn() {
        Ok(child) => child,
        Err(error) => {
            cleanup_staged_installer(&app, &staged_installer.host_path);
            return Err(format!("Installer başlatılamadı: {error}"));
        }
    };

    runtime::emit_backend_log(
        &app,
        "info",
        &format!("Installer başlatıldı: {}", request.installer_path),
    )?;

    let handle = app.clone();
    thread::spawn(move || {
        let status = child.wait();
        let prefix_path = prefix.wineprefix;
        match status {
            Ok(exit_status) if exit_status.success() => {
                let _ = runtime::emit_backend_log(
                    &handle,
                    "info",
                    "Installer tamamlandı kurulan uygulama aranıyor",
                );
            }
            Ok(exit_status) => {
                let _ = runtime::emit_backend_log(
                    &handle,
                    "warn",
                    &format!(
                        "Installer çıkış kodu başarılı değil {exit_status} yine de uygulama aranıyor"
                    ),
                );
            }
            Err(error) => {
                let _ = runtime::emit_backend_log(
                    &handle,
                    "warn",
                    &format!("Installer bitiş durumu okunamadı: {error}"),
                );
            }
        }
        wait_for_wine_processes(&handle, &wine, &prefix_path);
        if let Err(error) = add_installed_app_after_installer(
            &handle,
            request,
            prefix_path,
            install_dir,
            scan_started_at,
        ) {
            let _ = runtime::emit_backend_log(
                &handle,
                "warn",
                &format!("Kurulum sonrası kütüphane kaydı eklenemedi: {error}"),
            );
        }
        cleanup_staged_installer(&handle, &staged_installer.host_path);
    });
    Ok(())
}

fn stage_installer_for_wine(
    app: &AppHandle,
    installer_path: &str,
    prefix_path: &str,
) -> Result<StagedInstaller, String> {
    let source = Path::new(installer_path);
    let extension = source
        .extension()
        .and_then(|value| value.to_str())
        .filter(|value| !value.trim().is_empty())
        .unwrap_or("exe");
    let stem = source
        .file_stem()
        .and_then(|value| value.to_str())
        .map(ascii_path_token)
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| "installer".into());
    let timestamp = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .map(|duration| duration.as_millis())
        .unwrap_or_default();
    let cache_dir = Path::new(prefix_path).join("drive_c/ardali/cache");
    fs::create_dir_all(&cache_dir)
        .map_err(|error| format!("Installer geçici klasörü oluşturulamadı: {error}"))?;
    let target = cache_dir.join(format!("{timestamp}-{stem}.{extension}"));
    fs::copy(source, &target)
        .map_err(|error| format!("Installer güvenli yola kopyalanamadı: {error}"))?;
    runtime::emit_backend_log(
        app,
        "info",
        &format!(
            "Installer Wine içine kopyalandı: {}",
            windows_cache_path(&target)
        ),
    )?;
    Ok(StagedInstaller {
        launch_path: windows_cache_path(&target),
        host_path: target,
        working_dir: cache_dir,
    })
}

fn windows_cache_path(path: &Path) -> String {
    let file_name = path
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or("installer.exe");
    format!("C:\\ardali\\cache\\{file_name}")
}

fn cleanup_staged_installer(app: &AppHandle, staged_installer: &Path) {
    if !staged_installer.exists() {
        return;
    }
    match fs::remove_file(staged_installer) {
        Ok(()) => {
            let _ = runtime::emit_backend_log(
                app,
                "info",
                &format!(
                    "Geçici installer silindi: {}",
                    staged_installer.to_string_lossy()
                ),
            );
        }
        Err(error) => {
            let _ = runtime::emit_backend_log(
                app,
                "warn",
                &format!("Geçici installer silinemedi: {error}"),
            );
        }
    }
}

fn ascii_path_token(value: &str) -> String {
    value
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() {
                character
            } else {
                '-'
            }
        })
        .collect::<String>()
        .split('-')
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>()
        .join("-")
        .to_ascii_lowercase()
}

fn wait_for_wine_processes(app: &AppHandle, wine: &str, prefix_path: &str) {
    let wineserver = Path::new(wine)
        .parent()
        .map(|path| path.join("wineserver"))
        .filter(|path| path.exists())
        .map(|path| path.to_string_lossy().into_owned())
        .or_else(|| command_path("wineserver"));
    let Some(wineserver) = wineserver else {
        thread::sleep(Duration::from_secs(3));
        return;
    };

    let status = Command::new(wineserver)
        .arg("-w")
        .env("WINEPREFIX", prefix_path)
        .env("WINEARCH", "win64")
        .env("WINEDEBUG", "-all")
        .status();
    if let Err(error) = status {
        let _ = runtime::emit_backend_log(
            app,
            "warn",
            &format!("Wine işlem bekleme tamamlanamadı: {error}"),
        );
    }
    thread::sleep(Duration::from_secs(2));
}

fn stop_wine_processes(app: &AppHandle, wine: &str, prefix_path: &str) {
    let wineserver = Path::new(wine)
        .parent()
        .map(|path| path.join("wineserver"))
        .filter(|path| path.exists())
        .map(|path| path.to_string_lossy().into_owned())
        .or_else(|| command_path("wineserver"));
    let Some(wineserver) = wineserver else {
        return;
    };

    let status = Command::new(wineserver)
        .arg("-k")
        .env("WINEPREFIX", prefix_path)
        .env("WINEARCH", "win64")
        .env("WINEDEBUG", "-all")
        .status();
    if let Err(error) = status {
        let _ = runtime::emit_backend_log(
            app,
            "warn",
            &format!("Eski Wine işlemleri kapatılamadı: {error}"),
        );
    }
}

fn installer_working_dir(request: &GameInstallRequest) -> String {
    request
        .install_dir
        .clone()
        .filter(|path| !path.trim().is_empty())
        .or_else(|| {
            Path::new(&request.installer_path)
                .parent()
                .map(|path| path.to_string_lossy().into_owned())
        })
        .unwrap_or_else(|| ".".into())
}

fn add_installed_app_after_installer(
    app: &AppHandle,
    mut request: GameInstallRequest,
    prefix_path: String,
    install_dir: String,
    scan_started_at: SystemTime,
) -> Result<(), String> {
    let installed_executable = find_installed_windows_executable(
        &prefix_path,
        &install_dir,
        &request.installer_path,
        scan_started_at,
    )
    .ok_or_else(|| {
        "Kurulumdan sonra yeni uygulama exe bulunamadı Kurulum tamamlanmamış olabilir".to_string()
    })?;
    let installed_dir = Path::new(&installed_executable)
        .parent()
        .map(|path| path.to_string_lossy().into_owned())
        .unwrap_or(install_dir);

    request.game_kind = GameKind::WindowsExe;
    request.library_type = Some(LibraryType::WindowsApp);
    request.installer_path = installed_executable;
    request.install_dir = Some(installed_dir);

    let selection = runtime::select_runner(
        request.game_kind.clone(),
        Some(request.installer_path.clone()),
    )?;
    let record = database::add_game(request, &selection, Some(prefix_path))?;
    apply_windows_version(app, &record)?;
    extract_windows_icon_for_record(app, &record);
    runtime::emit_backend_log(
        app,
        "info",
        &format!("Kurulum tamamlandı ve kütüphaneye eklendi: {}", record.name),
    )?;
    let _ = app.emit("library-changed", &record);
    Ok(())
}

fn find_installed_windows_executable(
    prefix_path: &str,
    install_dir: &str,
    installer_path: &str,
    _scan_started_at: SystemTime,
) -> Option<String> {
    let drive_c = Path::new(prefix_path).join("drive_c");
    let roots = [
        PathBuf::from(install_dir),
        drive_c.join("Program Files"),
        drive_c.join("Program Files (x86)"),
    ];
    let installer = Path::new(installer_path);
    let mut candidates = Vec::new();
    for root in roots {
        collect_executable_candidates(&root, installer, &mut candidates);
    }
    candidates
        .into_iter()
        .max_by_key(|candidate| executable_score(&candidate.0, candidate.1))
        .map(|candidate| candidate.0.to_string_lossy().into_owned())
}

fn collect_executable_candidates(
    root: &Path,
    installer: &Path,
    candidates: &mut Vec<(PathBuf, SystemTime)>,
) {
    let Ok(entries) = fs::read_dir(root) else {
        return;
    };
    for entry in entries.filter_map(Result::ok) {
        let path = entry.path();
        if path.is_dir() {
            collect_executable_candidates(&path, installer, candidates);
            continue;
        }
        let modified = entry
            .metadata()
            .and_then(|metadata| metadata.modified())
            .unwrap_or(SystemTime::UNIX_EPOCH);
        if !has_extension(&path, "exe")
            || same_path(&path, installer)
            || is_helper_executable(&path)
            || is_wine_system_executable(&path)
        {
            continue;
        }
        candidates.push((path, modified));
    }
}

fn executable_score(path: &Path, modified: SystemTime) -> (u8, SystemTime) {
    let name = path
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase();
    let parent = path
        .parent()
        .and_then(|value| value.file_name())
        .and_then(|value| value.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase();
    let name_without_ext = name.strip_suffix(".exe").unwrap_or(&name);
    let score = if name_without_ext == parent {
        5
    } else if parent.contains(name_without_ext) || name_without_ext.contains(&parent) {
        4
    } else if name.contains("launcher") || name.contains("player") {
        3
    } else {
        1
    };
    (score, modified)
}

fn same_path(left: &Path, right: &Path) -> bool {
    match (left.canonicalize(), right.canonicalize()) {
        (Ok(left), Ok(right)) => left == right,
        _ => left == right,
    }
}

fn is_helper_executable(path: &Path) -> bool {
    let name = path
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase();
    [
        "unins",
        "uninstall",
        "setup",
        "install",
        "installer",
        "update",
    ]
    .iter()
    .any(|needle| name.contains(needle))
}

fn is_wine_system_executable(path: &Path) -> bool {
    let normalized = path
        .to_string_lossy()
        .replace('\\', "/")
        .to_ascii_lowercase();
    [
        "/drive_c/windows/",
        "/drive_c/program files/internet explorer/",
        "/drive_c/program files (x86)/internet explorer/",
        "/drive_c/program files/windows media player/",
        "/drive_c/program files (x86)/windows media player/",
        "/drive_c/program files/windows nt/",
        "/drive_c/program files (x86)/windows nt/",
    ]
    .iter()
    .any(|needle| normalized.contains(needle))
}

fn validate_install_request(
    request: &GameInstallRequest,
    require_install_dir: bool,
) -> Result<(), String> {
    if request.name.trim().is_empty() {
        return Err("Kütüphaneye eklemek için ad girmelisin".into());
    }
    if request.installer_path.trim().is_empty() {
        return Err("Windows dosyası veya hedef dosya seçmelisin".into());
    }
    if require_install_dir
        && request
            .install_dir
            .as_deref()
            .map(str::trim)
            .filter(|path| !path.is_empty())
            .is_none()
    {
        return Err("Installer çalıştırmak için kurulum klasörü seçmelisin".into());
    }

    let installer_path = Path::new(&request.installer_path);
    match request.game_kind {
        GameKind::WindowsExe if !has_extension(installer_path, "exe") => {
            Err("Windows EXE türü için .exe dosyası seçmelisin".into())
        }
        GameKind::WindowsMsi if !has_extension(installer_path, "msi") => {
            Err("Windows MSI türü için .msi dosyası seçmelisin".into())
        }
        _ => Ok(()),
    }
}

fn has_extension(path: &Path, extension: &str) -> bool {
    path.extension()
        .and_then(|value| value.to_str())
        .is_some_and(|value| value.eq_ignore_ascii_case(extension))
}

fn prefix_identity(request: &GameInstallRequest) -> (String, String) {
    match request.prefix_mode.as_ref() {
        Some(PrefixMode::SharedWindowsApps) => {
            ("windows-apps".into(), "Windows Uygulamaları Ortamı".into())
        }
        _ => (
            runtime::sanitize_game_id(&request.name),
            request.name.clone(),
        ),
    }
}

fn pick_file_with_portal() -> Result<Option<String>, String> {
    if !command_exists("gdbus") {
        return Err("gdbus yok".into());
    }

    let options = "{'multiple': <false>, 'modal': <true>, 'filters': <[('Windows dosyaları', [(0, '*.exe'), (0, '*.msi')])]>}";
    let output = Command::new("gdbus")
        .args([
            "call",
            "--session",
            "--dest",
            "org.freedesktop.portal.Desktop",
            "--object-path",
            "/org/freedesktop/portal/desktop",
            "--method",
            "org.freedesktop.portal.FileChooser.OpenFile",
            "",
            "ArDali Gaming",
            options,
        ])
        .output()
        .map_err(|error| format!("Portal dosya seçici açılamadı: {error}"))?;

    if !output.status.success() {
        return Err(String::from_utf8_lossy(&output.stderr).trim().to_string());
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let handle = parse_portal_handle(&stdout).ok_or_else(|| "Portal cevabı okunamadı".to_string())?;
    wait_for_portal_file_response(&handle)
}

fn parse_portal_handle(output: &str) -> Option<String> {
    let start = output.find("objectpath '")? + "objectpath '".len();
    let rest = output.get(start..)?;
    let end = rest.find('\'')?;
    Some(rest[..end].to_string())
}

fn wait_for_portal_file_response(handle: &str) -> Result<Option<String>, String> {
    let mut monitor = Command::new("gdbus")
        .args([
            "monitor",
            "--session",
            "--dest",
            "org.freedesktop.portal.Desktop",
            "--object-path",
            handle,
        ])
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|error| format!("Portal cevabı izlenemedi: {error}"))?;

    let stdout = monitor
        .stdout
        .take()
        .ok_or_else(|| "Portal izleme çıktısı alınamadı".to_string())?;
    let reader = BufReader::new(stdout);
    for line in reader.lines() {
        let line = line.map_err(|error| format!("Portal cevabı okunamadı: {error}"))?;
        if !line.contains("Response") {
            continue;
        }
        let result = if line.contains("uint32 0") {
            parse_first_file_uri(&line).map(|uri| file_uri_to_path(&uri)).transpose()
        } else {
            Ok(None)
        };
        let _ = monitor.kill();
        let _ = monitor.wait();
        return result;
    }

    let _ = monitor.kill();
    let _ = monitor.wait();
    Err("Portal dosya seçici cevapsız kaldı".into())
}

fn parse_first_file_uri(line: &str) -> Option<String> {
    let start = line.find("file://")?;
    let rest = line.get(start..)?;
    let end = rest.find('\'').or_else(|| rest.find('"')).unwrap_or(rest.len());
    Some(rest[..end].to_string())
}

fn file_uri_to_path(uri: &str) -> Result<String, String> {
    let path = uri
        .strip_prefix("file://")
        .ok_or_else(|| "Portal file URI döndürmedi".to_string())?;
    percent_decode(path)
}

fn percent_decode(value: &str) -> Result<String, String> {
    let bytes = value.as_bytes();
    let mut decoded = Vec::with_capacity(bytes.len());
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] == b'%' && index + 2 < bytes.len() {
            let hex = std::str::from_utf8(&bytes[index + 1..index + 3])
                .map_err(|error| format!("URI kodu okunamadı: {error}"))?;
            let byte = u8::from_str_radix(hex, 16)
                .map_err(|error| format!("URI kodu çözülemedi: {error}"))?;
            decoded.push(byte);
            index += 3;
        } else {
            decoded.push(bytes[index]);
            index += 1;
        }
    }
    String::from_utf8(decoded).map_err(|error| format!("URI UTF-8 değil: {error}"))
}

fn pick_path(folder: bool) -> Result<Option<String>, String> {
    let icon_path = linux_app_icon_path();
    let candidates: Vec<(&str, Vec<String>)> = if folder {
        vec![
            (
                "kdialog",
                vec![
                    "--title".into(),
                    "ArDali Gaming".into(),
                    "--icon".into(),
                    icon_path.clone(),
                    "--getexistingdirectory".into(),
                    "~".into(),
                ],
            ),
            (
                "zenity",
                vec![
                    "--title=ArDali Gaming".into(),
                    format!("--window-icon={icon_path}"),
                    "--file-selection".into(),
                    "--directory".into(),
                ],
            ),
        ]
    } else {
        vec![
            (
                "kdialog",
                vec![
                    "--title".into(),
                    "ArDali Gaming".into(),
                    "--icon".into(),
                    icon_path.clone(),
                    "--getopenfilename".into(),
                    "~".into(),
                    "*.exe *.msi|Windows kurulum dosyaları (*.exe *.msi)".into(),
                ],
            ),
            (
                "zenity",
                vec![
                    "--title=ArDali Gaming".into(),
                    format!("--window-icon={icon_path}"),
                    "--file-selection".into(),
                ],
            ),
        ]
    };

    for (program, args) in candidates {
        let output = Command::new(program).args(args).output();
        let Ok(output) = output else {
            continue;
        };
        if output.status.success() {
            let value = String::from_utf8_lossy(&output.stdout).trim().to_string();
            return Ok((!value.is_empty()).then_some(value));
        }
        if output.status.code() == Some(1) {
            return Ok(None);
        }
    }

    Err("Dosya seçici açılamadı kdialog veya zenity kurulu olmalı".into())
}

fn linux_app_icon_path() -> String {
    env::var_os("HOME")
        .map(PathBuf::from)
        .map(|home| {
            home.join(".local/share/icons/hicolor/512x512/apps/ardali-gaming.png")
                .to_string_lossy()
                .into_owned()
        })
        .unwrap_or_else(|| "ardali-gaming".into())
}

#[tauri::command]
fn list_games() -> Result<Vec<GameRecord>, String> {
    database::list_games()
}

#[tauri::command]
fn launch_game(app: AppHandle, id: i64, options: GameModeOptions) -> Result<GameRecord, String> {
    let source = database::get_game_by_id(id)?;
    apply_windows_version(&app, &source)?;
    let effective_options = GameModeOptions {
        display_mode: if source.display_mode == "fullscreen" {
            DisplayMode::Fullscreen
        } else {
            DisplayMode::Windowed
        },
        fps_overlay: options.fps_overlay,
    };
    if let Some(executable) = &source.executable {
        let runner = effective_runner(&source);
        let cncnet_client = if runner == "cncnet" {
            let path = database::cncnet_client_path(&source.install_dir);
            if !path.exists() {
                return Err("CnCNet kurulu değil kurmak ister misiniz".into());
            }
            Some(path)
        } else {
            None
        };
        let base_executable = effective_executable(runner, executable)?;
        let base_launch_args = if runner == "cncnet" {
            cncnet_client
                .as_ref()
                .map(|path| vec![path.to_string_lossy().into_owned()])
                .unwrap_or_default()
        } else if runner == "wine" && source.virtual_desktop {
            wine_virtual_desktop_arguments(&source)
        } else {
            source.arguments.clone()
        };
        let launch_command =
            launch_command(&app, &source, runner, &base_executable, base_launch_args)?;
        eprintln!(
            "[ardali-launch] id={} name='{}' runner={} executable='{}' args={:?} prefix={:?} display={} virtual_desktop={} resolution={} windows={} overrides='{}'",
            source.id,
            source.name,
            runner,
            launch_command.executable,
            launch_command.args,
            source.prefix_path,
            source.display_mode,
        source.virtual_desktop,
        selected_resolution(&source),
            source.windows_version,
            if runner == "wine" { wine_dll_overrides(&source) } else { String::new() },
        );
        let mut command = Command::new(&launch_command.executable);
        command.args(&launch_command.args);
        command.env("ARDALI_DISPLAY_MODE", source.display_mode.as_str());
        command.env("MANGOHUD", if options.fps_overlay { "1" } else { "0" });
        if runner == "wine" {
            let overrides = wine_dll_overrides(&source);
            if !overrides.is_empty() {
                command.env("WINEDLLOVERRIDES", overrides);
            }
            command.env("WINEDEBUG", "-all");
            if source.dxvk_enabled {
                command.env("DXVK_LOG_LEVEL", "none");
            } else {
                command.env("DXVK_DISABLE", "1");
            }
        }
        if let Some(prefix_path) = &source.prefix_path {
            if runner == "wine" && source.virtual_desktop {
                hide_wine_desktop_shortcuts(prefix_path);
            }
            command.env("WINEPREFIX", prefix_path);
        }
        command.current_dir(&source.install_dir);

        let mut child = command
            .spawn()
            .map_err(|error| format!("Cannot launch game runner: {error}"))?;
        let desktop_title_for_monitor = should_auto_fullscreen_wine_desktop(&source, runner)
            .then(|| format!("{} - Wine Desktop", wine_desktop_name(&source.name)));
        if should_auto_fullscreen_wine_desktop(&source, runner) {
            let handle = app.clone();
            let desktop_title = desktop_title_for_monitor.clone().unwrap_or_default();
            thread::spawn(move || {
                auto_fullscreen_wine_desktop(&handle, &desktop_title);
            });
        }
        let handle = app.clone();
        let name = source.name.clone();
        thread::spawn(move || {
            let status = monitor_game_process(&mut child, desktop_title_for_monitor.as_deref());
            eprintln!("[ardali-launch-ended] id={id} name='{name}' status={status}");
            let _ = database::finish_play_session(id);
            let _ = handle.emit("game-ended", GameProcessEvent { id, name, status });
        });
    }

    let game = database::mark_game_launched(id, &effective_options)?;
    runtime::emit_backend_log(&app, "info", &format!("Launch requested: {}", game.name))?;
    Ok(game)
}

fn monitor_game_process(child: &mut Child, desktop_title: Option<&str>) -> String {
    let mut desktop_seen = false;
    let mut checks = 0_u32;

    loop {
        match child.try_wait() {
            Ok(Some(status)) => return status.to_string(),
            Ok(None) => {}
            Err(error) => return format!("wait failed: {error}"),
        }

        if let Some(title) = desktop_title {
            let window_exists = wine_desktop_window_exists(title);
            desktop_seen = desktop_seen || window_exists;
            if desktop_seen && !window_exists && checks > 3 {
                return "Wine Desktop window closed".into();
            }
        }

        checks = checks.saturating_add(1);
        thread::sleep(std::time::Duration::from_millis(1000));
    }
}

fn effective_runner(game: &GameRecord) -> &str {
    if game.preferred_runner.trim().is_empty() {
        &game.runner
    } else {
        &game.preferred_runner
    }
}

fn effective_executable(runner: &str, fallback: &str) -> Result<String, String> {
    if runner == "cncnet" {
        return wine_executable();
    }

    if runner == "wine" && !Path::new(fallback).exists() {
        return wine_executable();
    }

    Ok(fallback.to_string())
}

fn wine_executable() -> Result<String, String> {
    let executable = Path::new(&runtime::runtime_paths()?.wine_dir).join("current/bin/wine");
    if executable.exists() {
        return Ok(executable.to_string_lossy().into_owned());
    }

    if let Some(system_wine) = command_path("wine") {
        return Ok(system_wine);
    }

    Err("Wine bulunamadı Önce Wine kur veya sistem paket yöneticisinden Wine yükle".into())
}

fn launch_command(
    app: &AppHandle,
    game: &GameRecord,
    runner: &str,
    executable: &str,
    args: Vec<String>,
) -> Result<LaunchCommand, String> {
    if runner == "wine" && game.gamescope_enabled && game.display_mode == "fullscreen" {
        return gamescope_launch_command(app, game, executable, args);
    }

    Ok(LaunchCommand {
        executable: executable.to_string(),
        args,
    })
}

fn gamescope_launch_command(
    app: &AppHandle,
    game: &GameRecord,
    executable: &str,
    args: Vec<String>,
) -> Result<LaunchCommand, String> {
    let Some(gamescope) = command_path("gamescope") else {
        runtime::emit_backend_log(
            app,
            "warn",
            "Gamescope kurulu değil oyun normal Wine başlatmasına düşürüldü",
        )?;
        return Ok(LaunchCommand {
            executable: executable.to_string(),
            args,
        });
    };

    let source_resolution = selected_gamescope_source_resolution(game);
    let target_resolution = detect_screen_resolution().unwrap_or_else(|| "1920x1080".into());
    let (source_width, source_height) = parse_resolution(&source_resolution)
        .ok_or_else(|| format!("Gamescope için çözünürlük okunamadı: {source_resolution}"))?;
    let (target_width, target_height) =
        parse_resolution(&target_resolution).unwrap_or((source_width, source_height));

    let mut gamescope_args = vec![
        "-f".into(),
        "-w".into(),
        source_width.to_string(),
        "-h".into(),
        source_height.to_string(),
        "-W".into(),
        target_width.to_string(),
        "-H".into(),
        target_height.to_string(),
        "-S".into(),
        selected_gamescope_scaler(game).into(),
    ];
    if !game.virtual_desktop {
        gamescope_args.push("--force-windows-fullscreen".into());
    }
    gamescope_args.extend(["--".into(), executable.to_string()]);
    gamescope_args.extend(args);

    Ok(LaunchCommand {
        executable: gamescope,
        args: gamescope_args,
    })
}

#[tauri::command]
async fn install_cncnet_for_game(app: AppHandle, id: i64) -> Result<GameRecord, String> {
    tauri::async_runtime::spawn_blocking(move || install_cncnet_for_game_blocking(app, id))
        .await
        .map_err(|error| format!("CnCNet kurulum görevi tamamlanamadı: {error}"))?
}

fn install_cncnet_for_game_blocking(app: AppHandle, id: i64) -> Result<GameRecord, String> {
    let game = database::get_game_by_id(id)?;
    if database::cncnet_client_path(&game.install_dir).exists() {
        emit_cncnet_install_progress(&app, id, 100, "CnCNet zaten kurulu")?;
        return database::mark_cncnet_installed(id);
    }

    let installer_dir = cncnet_temp_dir()?;
    let installer = installer_dir.join("CnCNet5_YR_Installer.exe");
    fs::create_dir_all(&installer_dir)
        .map_err(|error| format!("Geçici CnCNet dizini oluşturulamadı: {error}"))?;

    emit_cncnet_install_progress(&app, id, 5, "CnCNet release aranıyor")?;
    let fallback_url = "https://downloads.cncnet.org/CnCNet5_YR_Installer.exe";
    let url = resolve_cncnet_installer_url().unwrap_or_else(|| fallback_url.to_string());
    emit_cncnet_install_progress(&app, id, 15, "CnCNet indiriliyor")?;
    if let Err(error) = download_file(&url, &installer) {
        runtime::emit_backend_log(
            &app,
            "warn",
            &format!("GitHub CnCNet indirimi başarısız fallback deneniyor: {error}"),
        )?;
        download_file(fallback_url, &installer)?;
    }

    emit_cncnet_install_progress(&app, id, 70, "CnCNet kurulumu başlatılıyor")?;
    run_cncnet_installer(&game, &installer)?;

    if !database::cncnet_client_path(&game.install_dir).exists() {
        return Err("Kurulum tamamlandı ama Resources/clientogl.exe bulunamadı".into());
    }

    let _ = fs::remove_file(&installer);
    emit_cncnet_install_progress(&app, id, 100, "CnCNet kuruldu")?;
    database::mark_cncnet_installed(id)
}

fn cncnet_temp_dir() -> Result<std::path::PathBuf, String> {
    let home =
        std::env::var("HOME").map_err(|_| "HOME environment variable is not set.".to_string())?;
    Ok(Path::new(&home).join(".cache").join("ardali-gaming"))
}

fn resolve_cncnet_installer_url() -> Option<String> {
    let output = Command::new("curl")
        .args([
            "-L",
            "--fail",
            "--silent",
            "--show-error",
            "https://api.github.com/repos/CnCNet/cncnet-yr-client-package/releases/latest",
        ])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let value: Value = serde_json::from_slice(&output.stdout).ok()?;
    value.get("assets")?.as_array()?.iter().find_map(|asset| {
        let name = asset.get("name")?.as_str()?;
        if name.to_lowercase().ends_with(".exe") {
            asset
                .get("browser_download_url")
                .and_then(Value::as_str)
                .map(ToString::to_string)
        } else {
            None
        }
    })
}

fn download_file(url: &str, target: &Path) -> Result<(), String> {
    let status = Command::new("curl")
        .args(["-L", "--fail", "--show-error", "-o"])
        .arg(target)
        .arg(url)
        .status()
        .map_err(|error| format!("CnCNet indirilemedi: {error}"))?;
    if !status.success() {
        return Err(format!("CnCNet indirme başarısız: {status}"));
    }
    Ok(())
}

fn run_cncnet_installer(game: &GameRecord, installer: &Path) -> Result<(), String> {
    let wine = wine_executable()?;
    let Some(prefix_path) = &game.prefix_path else {
        return Err("Bu oyun için Wine prefix bulunamadı".into());
    };

    let install_dir_arg = format!("/DIR={}", game.install_dir);
    let status = Command::new(wine)
        .arg(installer)
        .arg("/VERYSILENT")
        .arg("/SUPPRESSMSGBOXES")
        .arg("/NORESTART")
        .arg(install_dir_arg)
        .env("WINEPREFIX", prefix_path)
        .env("WINEDEBUG", "-all")
        .current_dir(&game.install_dir)
        .status()
        .map_err(|error| format!("CnCNet installer başlatılamadı: {error}"))?;
    if !status.success() {
        return Err(format!("CnCNet installer çıktı: {status}"));
    }
    Ok(())
}

fn emit_cncnet_install_progress(
    app: &AppHandle,
    id: i64,
    percent: u8,
    status: &str,
) -> Result<(), String> {
    app.emit(
        "cncnet-install-progress",
        CncNetInstallProgress {
            id,
            percent,
            status: status.into(),
        },
    )
    .map_err(|error| format!("CnCNet ilerleme olayı gönderilemedi: {error}"))
}

fn wine_virtual_desktop_arguments(game: &GameRecord) -> Vec<String> {
    let desktop_name = wine_desktop_name(&game.name);
    let desktop_size = selected_wine_desktop_resolution(game);
    let mut arguments = vec![
        "explorer".into(),
        format!("/desktop={desktop_name},{desktop_size}"),
    ];
    arguments.extend(game.arguments.clone());
    arguments
}

fn hide_wine_desktop_shortcuts(prefix_path: &str) {
    let users_dir = Path::new(prefix_path).join("drive_c/users");
    let Ok(users) = fs::read_dir(users_dir) else {
        return;
    };

    for user in users.flatten() {
        let desktop_dir = user.path().join("Desktop");
        let Ok(entries) = fs::read_dir(desktop_dir) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            let Some(extension) = path.extension().and_then(|value| value.to_str()) else {
                continue;
            };
            if !matches!(extension.to_ascii_lowercase().as_str(), "lnk" | "url") {
                continue;
            }
            let Some(file_name) = path.file_name().and_then(|value| value.to_str()) else {
                continue;
            };
            let hidden = path.with_file_name(format!("{file_name}.ardali-hidden"));
            let _ = fs::rename(path, hidden);
        }
    }
}

fn should_auto_fullscreen_wine_desktop(game: &GameRecord, runner: &str) -> bool {
    runner == "wine" && game.virtual_desktop && game.display_mode == "fullscreen"
}

fn auto_fullscreen_wine_desktop(app: &AppHandle, title: &str) {
    thread::sleep(std::time::Duration::from_millis(1200));

    for attempt in 0..8 {
        if try_fullscreen_with_kdotool(title)
            || try_fullscreen_with_wmctrl(title)
            || try_fullscreen_with_xdotool(title)
        {
            eprintln!("[ardali-fullscreen] Wine Desktop büyütme denendi: {title}");
            let _ = runtime::emit_backend_log(
                app,
                "info",
                &format!("Wine Desktop tam ekran denendi: {title}"),
            );
            return;
        }
        thread::sleep(std::time::Duration::from_millis(450 + attempt * 80));
    }

    let _ = runtime::emit_backend_log(
        app,
        "warn",
        &format!("Wine Desktop penceresi otomatik büyütülemedi: {title}"),
    );
    eprintln!("[ardali-fullscreen] Wine Desktop penceresi bulunamadı/büyütülemedi: {title}");
}

fn wine_desktop_window_exists(title: &str) -> bool {
    if let Some(kdotool) = command_path("kdotool") {
        let script = format!(
            "{} search --title {} --limit 1 >/dev/null 2>&1",
            shell_quote(&kdotool),
            shell_quote(&regex_escape(title)),
        );
        if shell_status_success(&script) {
            return true;
        }
    }

    if let Some(wmctrl) = command_path("wmctrl") {
        let script = format!(
            "{} -l 2>/dev/null | grep -F -- {} >/dev/null",
            shell_quote(&wmctrl),
            shell_quote(title),
        );
        if shell_status_success(&script) {
            return true;
        }
    }

    if let Some(xdotool) = command_path("xdotool") {
        let script = format!(
            "{} search --name {} >/dev/null 2>&1",
            shell_quote(&xdotool),
            shell_quote(title),
        );
        if shell_status_success(&script) {
            return true;
        }
    }

    false
}

fn try_fullscreen_with_kdotool(title: &str) -> bool {
    let Some(kdotool) = command_path("kdotool") else {
        return false;
    };

    let exact_title = regex_escape(title);
    let script = kdotool_fullscreen_script(&kdotool, &exact_title)
        + " || "
        + &kdotool_fullscreen_script(&kdotool, "Wine Desktop");
    shell_status_success(&script)
}

fn kdotool_fullscreen_script(kdotool: &str, pattern: &str) -> String {
    format!(
        "{} search --title {} --limit 1 windowactivate %1 windowstate --add FULLSCREEN %1 windowsize %1 100% 100%",
        shell_quote(kdotool),
        shell_quote(pattern),
    )
}

fn try_fullscreen_with_wmctrl(title: &str) -> bool {
    let Some(wmctrl) = command_path("wmctrl") else {
        return false;
    };

    let script = format!(
        "{} -r '{}' -b add,maximized_vert,maximized_horz || {} -r '{}' -b add,fullscreen",
        shell_quote(&wmctrl),
        shell_quote(title),
        shell_quote(&wmctrl),
        shell_quote(title)
    );
    shell_status_success(&script)
}

fn try_fullscreen_with_xdotool(title: &str) -> bool {
    let Some(xdotool) = command_path("xdotool") else {
        return false;
    };

    let script = format!(
        "wid=$({} search --name '{}' 2>/dev/null | head -n1); \
         test -n \"$wid\" && {} windowactivate \"$wid\" && {} windowsize \"$wid\" 100% 100%",
        shell_quote(&xdotool),
        shell_quote(title),
        shell_quote(&xdotool),
        shell_quote(&xdotool)
    );
    shell_status_success(&script)
}

fn command_exists(command: &str) -> bool {
    command_path(command).is_some()
}

fn command_path(command: &str) -> Option<String> {
    let output = Command::new("sh")
        .arg("-c")
        .arg(format!("command -v {} 2>/dev/null", shell_quote(command)))
        .output()
        .ok()?;

    if output.status.success() {
        let path = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if !path.is_empty() {
            return Some(path);
        }
    }

    let cargo_bin = std::env::var("HOME")
        .ok()
        .map(|home| Path::new(&home).join(".cargo/bin").join(command));
    cargo_bin
        .filter(|path| path.exists())
        .map(|path| path.to_string_lossy().to_string())
}

fn shell_status_success(script: &str) -> bool {
    Command::new("sh")
        .arg("-c")
        .arg(script)
        .status()
        .map(|status| status.success())
        .unwrap_or(false)
}

fn shell_quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', "'\\''"))
}

fn regex_escape(value: &str) -> String {
    let mut escaped = String::with_capacity(value.len());
    for character in value.chars() {
        if "\\.^$|?*+()[]{}".contains(character) {
            escaped.push('\\');
        }
        escaped.push(character);
    }
    escaped
}

fn recommended_fullscreen_tool(session_type: &str, desktop_environment: &str) -> String {
    let session = session_type.to_ascii_lowercase();
    let desktop = desktop_environment.to_ascii_lowercase();
    if session == "x11" {
        "wmctrl/xdotool".into()
    } else if session == "wayland" && desktop.contains("kde") {
        "kdotool".into()
    } else if session == "wayland" {
        "unsupported".into()
    } else {
        "unsupported".into()
    }
}

fn fullscreen_tool_warning(
    session_type: &str,
    desktop_environment: &str,
    recommended_tool: &str,
) -> String {
    if recommended_tool == "wmctrl/xdotool" {
        return format!(
            "X11 oturumu algılandı ({session_type}/{desktop_environment}) Bu araç sadece Wine Desktop penceresini büyütür oyun görüntüsünü ölçeklemek için Gamescope önerilir"
        );
    }

    if recommended_tool == "unsupported" {
        return format!(
            "Bu masaüstü için otomatik pencere büyütme aracı desteklenmiyor ({session_type}/{desktop_environment}) Eski oyun ölçekleme için Gamescope önerilir"
        );
    }

    format!(
        "KDE Wayland oturumu algılandı ({session_type}/{desktop_environment}) kdotool sadece Wine Desktop penceresini büyütür oyun görüntüsünü ölçeklemek için Gamescope önerilir"
    )
}

fn install_x11_fullscreen_tools() -> Result<(), String> {
    run_script_in_system_terminal(
        "ArDali Gaming - X11 tam ekran araçları",
        x11_fullscreen_tool_install_script(),
    )
}

fn install_kdotool_for_wayland() -> Result<(), String> {
    if command_exists("kdotool") {
        return Ok(());
    }

    run_script_in_system_terminal("ArDali Gaming - kdotool kur", kdotool_install_script())
}

fn run_script_in_system_terminal(title: &str, script: &str) -> Result<(), String> {
    let wrapped = terminal_wrapper_script(title, script);
    let status = if command_exists("konsole") {
        Command::new("konsole")
            .args(["--nofork", "-p"])
            .arg(format!("tabtitle={title}"))
            .args(["-e", "sh", "-lc"])
            .arg(&wrapped)
            .status()
    } else if command_exists("gnome-terminal") {
        Command::new("gnome-terminal")
            .args(["--wait", "--title"])
            .arg(title)
            .args(["--", "sh", "-lc"])
            .arg(&wrapped)
            .status()
    } else if command_exists("kgx") {
        Command::new("kgx")
            .args(["--title"])
            .arg(title)
            .args(["--", "sh", "-lc"])
            .arg(&wrapped)
            .status()
    } else if command_exists("xterm") {
        Command::new("xterm")
            .args(["-T", title, "-e", "sh", "-lc"])
            .arg(&wrapped)
            .status()
    } else {
        Command::new("sh").arg("-lc").arg(script).status()
    }
    .map_err(|error| format!("Sistem terminali başlatılamadı: {error}"))?;

    if !status.success() {
        return Err(format!("Sistem terminalindeki işlem başarısız: {status}"));
    }
    Ok(())
}

fn terminal_wrapper_script(title: &str, script: &str) -> String {
    format!(
        "printf '\\033]0;{}\\007'; \
         set +e; \
         {}; \
         status=$?; \
         echo; \
         if [ \"$status\" -eq 0 ]; then \
           echo 'ArDali Gaming: işlem tamamlandı Pencere birazdan kapanacak'; sleep 3; \
         else \
           echo 'ArDali Gaming: işlem başarısız oldu'; \
           printf 'Kapatmak için Enter tuşuna basın'; read _; \
         fi; \
         exit \"$status\"",
        title.replace('\'', ""),
        script
    )
}

fn x11_fullscreen_tool_install_script() -> &'static str {
    "if command -v pkexec >/dev/null 2>&1; then
  AUTH='pkexec'
elif command -v sudo >/dev/null 2>&1; then
  AUTH='sudo'
else
  echo 'pkexec veya sudo bulunamadı' >&2
  exit 1
fi
if command -v pacman >/dev/null 2>&1; then
  $AUTH pacman -S --needed --noconfirm wmctrl xdotool
elif command -v dnf >/dev/null 2>&1; then
  $AUTH dnf install -y wmctrl xdotool
elif command -v zypper >/dev/null 2>&1; then
  $AUTH zypper --non-interactive install wmctrl xdotool
elif command -v apt-get >/dev/null 2>&1; then
  $AUTH sh -c 'apt-get update && apt-get install -y wmctrl xdotool'
else
  echo 'Desteklenen paket yöneticisi bulunamadı' >&2
  exit 1
fi"
}

fn kdotool_install_script() -> &'static str {
    "if command -v yay >/dev/null 2>&1; then
  yay -S --needed kdotool
elif command -v paru >/dev/null 2>&1; then
  paru -S --needed kdotool
elif command -v cargo >/dev/null 2>&1; then
  tmp=\"${XDG_CACHE_HOME:-$HOME/.cache}/ardali-gaming/kdotool-build\"
  rm -rf \"$tmp\"
  mkdir -p \"$tmp\"
  cargo install --root \"$tmp/root\" --git https://github.com/jinliu/kdotool.git kdotool
  if command -v pkexec >/dev/null 2>&1; then
    pkexec install -m 0755 \"$tmp/root/bin/kdotool\" /usr/local/bin/kdotool
  elif command -v sudo >/dev/null 2>&1; then
    sudo install -m 0755 \"$tmp/root/bin/kdotool\" /usr/local/bin/kdotool
  else
    echo 'pkexec veya sudo bulunamadı' >&2
    exit 1
  fi
  rm -rf \"$tmp\"
elif command -v pkexec >/dev/null 2>&1; then
  AUTH='pkexec'
  if command -v dnf >/dev/null 2>&1; then
    $AUTH dnf install -y kdotool
  elif command -v zypper >/dev/null 2>&1; then
    $AUTH zypper --non-interactive install kdotool
  elif command -v apt-get >/dev/null 2>&1; then
    $AUTH sh -c 'apt-get update && apt-get install -y kdotool'
  else
    echo 'kdotool için AUR helper cargo veya desteklenen paket yöneticisi bulunamadı' >&2
    exit 1
  fi
else
  echo 'kdotool için yay/paru cargo veya pkexec bulunamadı' >&2
  exit 1
fi"
}

fn gamescope_install_script() -> &'static str {
    "if command -v pkexec >/dev/null 2>&1; then
  AUTH='pkexec'
elif command -v sudo >/dev/null 2>&1; then
  AUTH='sudo'
else
  echo 'pkexec veya sudo bulunamadı' >&2
  exit 1
fi
if command -v pacman >/dev/null 2>&1; then
  $AUTH pacman -S --needed --noconfirm gamescope
elif command -v dnf >/dev/null 2>&1; then
  $AUTH dnf install -y gamescope
elif command -v zypper >/dev/null 2>&1; then
  $AUTH zypper --non-interactive install gamescope
elif command -v apt-get >/dev/null 2>&1; then
  $AUTH sh -c 'apt-get update && apt-get install -y gamescope'
else
  echo 'Gamescope için desteklenen paket yöneticisi bulunamadı' >&2
  exit 1
fi"
}

fn gamescope_uninstall_script() -> &'static str {
    "if command -v pkexec >/dev/null 2>&1; then
  AUTH='pkexec'
elif command -v sudo >/dev/null 2>&1; then
  AUTH='sudo'
else
  echo 'pkexec veya sudo bulunamadı' >&2
  exit 1
fi
if command -v pacman >/dev/null 2>&1; then
  $AUTH pacman -Rns --noconfirm gamescope
elif command -v dnf >/dev/null 2>&1; then
  $AUTH dnf remove -y gamescope
elif command -v zypper >/dev/null 2>&1; then
  $AUTH zypper --non-interactive remove gamescope
elif command -v apt-get >/dev/null 2>&1; then
  $AUTH apt-get purge -y gamescope
else
  echo 'Gamescope için desteklenen paket yöneticisi bulunamadı' >&2
  exit 1
fi"
}

fn x11_fullscreen_tool_uninstall_script() -> &'static str {
    "if command -v pkexec >/dev/null 2>&1; then
  AUTH='pkexec'
elif command -v sudo >/dev/null 2>&1; then
  AUTH='sudo'
else
  echo 'pkexec veya sudo bulunamadı' >&2
  exit 1
fi
if command -v pacman >/dev/null 2>&1; then
  $AUTH pacman -Rns --noconfirm wmctrl xdotool
elif command -v dnf >/dev/null 2>&1; then
  $AUTH dnf remove -y wmctrl xdotool
elif command -v zypper >/dev/null 2>&1; then
  $AUTH zypper --non-interactive remove wmctrl xdotool
elif command -v apt-get >/dev/null 2>&1; then
  $AUTH apt-get purge -y wmctrl xdotool
else
  echo 'Desteklenen paket yöneticisi bulunamadı' >&2
  exit 1
fi"
}

fn kdotool_uninstall_script() -> &'static str {
    "set +e
if command -v cargo >/dev/null 2>&1; then
  cargo uninstall kdotool
fi
rm -f \"$HOME/.cargo/bin/kdotool\"
if [ -f /usr/local/bin/kdotool ]; then
  if command -v pkexec >/dev/null 2>&1; then
    pkexec rm -f /usr/local/bin/kdotool
  elif command -v sudo >/dev/null 2>&1; then
    sudo rm -f /usr/local/bin/kdotool
  fi
fi
if command -v pacman >/dev/null 2>&1 && pacman -Q kdotool >/dev/null 2>&1; then
  if command -v pkexec >/dev/null 2>&1; then pkexec pacman -Rns --noconfirm kdotool; else sudo pacman -Rns --noconfirm kdotool; fi
elif command -v dnf >/dev/null 2>&1 && rpm -q kdotool >/dev/null 2>&1; then
  if command -v pkexec >/dev/null 2>&1; then pkexec dnf remove -y kdotool; else sudo dnf remove -y kdotool; fi
elif command -v zypper >/dev/null 2>&1 && rpm -q kdotool >/dev/null 2>&1; then
  if command -v pkexec >/dev/null 2>&1; then pkexec zypper --non-interactive remove kdotool; else sudo zypper --non-interactive remove kdotool; fi
elif command -v dpkg >/dev/null 2>&1 && dpkg -s kdotool >/dev/null 2>&1; then
  if command -v pkexec >/dev/null 2>&1; then pkexec apt-get purge -y kdotool; else sudo apt-get purge -y kdotool; fi
fi
exit 0"
}

fn wine_desktop_name(name: &str) -> String {
    let desktop_name = name
        .chars()
        .filter(|character| character.is_ascii_alphanumeric())
        .collect::<String>();
    if desktop_name.is_empty() {
        "ArDaliGame".into()
    } else {
        desktop_name
    }
}

fn selected_resolution(game: &GameRecord) -> String {
    let value = game.resolution.trim();
    if !value.is_empty() && value != "auto" {
        return value.to_string();
    }

    detect_screen_resolution().unwrap_or_else(|| "1024x768".into())
}

fn selected_wine_desktop_resolution(game: &GameRecord) -> String {
    if game.display_mode == "fullscreen" && game.gamescope_enabled {
        return selected_game_resolution(game);
    }

    selected_resolution(game)
}

fn selected_gamescope_source_resolution(game: &GameRecord) -> String {
    if game.virtual_desktop {
        return selected_wine_desktop_resolution(game);
    }

    selected_game_resolution(game)
}

fn selected_game_resolution(game: &GameRecord) -> String {
    let value = game.resolution.trim();
    if !value.is_empty() && value != "auto" {
        return value.to_string();
    }

    "1024x768".into()
}

fn parse_resolution(value: &str) -> Option<(u32, u32)> {
    let (width, height) = value.trim().split_once('x')?;
    Some((width.parse().ok()?, height.parse().ok()?))
}

fn selected_gamescope_scaler(game: &GameRecord) -> &'static str {
    match game.gamescope_scaler.trim() {
        "stretch" => "stretch",
        "fill" => "fill",
        "integer" => "integer",
        "auto" => "auto",
        _ => "fit",
    }
}

fn detect_screen_resolution() -> Option<String> {
    for command in [
        "xrandr --current 2>/dev/null | awk '/\\*/ {print $1; exit}'",
        "wlr-randr 2>/dev/null | awk '/current/ {print $1; exit}'",
        "kscreen-doctor -o 2>/dev/null | awk -F': ' '/Resolution:/ {print $2; exit}'",
    ] {
        let output = Command::new("sh").arg("-c").arg(command).output().ok()?;
        if !output.status.success() {
            continue;
        }
        let resolution = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if !resolution.is_empty() {
            return Some(resolution);
        }
    }

    None
}

fn wine_dll_overrides(game: &GameRecord) -> String {
    let mut overrides = Vec::new();
    if game.ddraw_override {
        push_unique_override(&mut overrides, "ddraw=n,b");
    }
    if let Some(value) = &game.dll_override {
        for item in value
            .split(';')
            .map(str::trim)
            .filter(|item| !item.is_empty())
        {
            push_unique_override(&mut overrides, item);
        }
    }
    push_unique_override(&mut overrides, "winemenubuilder.exe=d");
    overrides.join(";")
}

fn push_unique_override(overrides: &mut Vec<String>, value: &str) {
    if !overrides.iter().any(|item| item == value) {
        overrides.push(value.to_string());
    }
}

#[tauri::command]
fn remove_game(app: AppHandle, id: i64, remove_files: bool) -> Result<(), String> {
    database::remove_game(id, remove_files)?;
    let _ = app.emit("library-changed", id);
    Ok(())
}

#[tauri::command]
fn refresh_windows_app_icon(app: AppHandle, id: i64) -> Result<GameRecord, String> {
    let record = database::get_game_by_id(id)?;
    extract_windows_icon_for_record(&app, &record);
    database::get_game_by_id(id)
}

#[tauri::command]
async fn uninstall_windows_app(app: AppHandle, id: i64) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || uninstall_windows_app_blocking(app, id))
        .await
        .map_err(|error| format!("Windows kaldırma görevi tamamlanamadı: {error}"))?
}

fn uninstall_windows_app_blocking(app: AppHandle, id: i64) -> Result<(), String> {
    let game = database::get_game_by_id(id)?;
    let Some(prefix_path) = game.prefix_path.clone() else {
        return Err("Bu kayıt için Wine prefix bulunamadı".into());
    };
    let target = game
        .arguments
        .first()
        .cloned()
        .or_else(|| Some(game.installer_path.clone()))
        .unwrap_or_default();
    let wine = wine_executable()?;
    stop_wine_processes(&app, &wine, &prefix_path);

    if let Some(uninstall_string) = find_uninstall_string(&prefix_path, &game, &target) {
        runtime::emit_backend_log(
            &app,
            "info",
            &format!("Windows kaldırıcı başlatılıyor: {uninstall_string}"),
        )?;
        let status = Command::new(&wine)
            .args(["cmd", "/c", &uninstall_string])
            .env("WINEPREFIX", &prefix_path)
            .env("WINEARCH", "win64")
            .current_dir(Path::new(&prefix_path).join("drive_c"))
            .status()
            .map_err(|error| format!("Windows kaldırıcı başlatılamadı: {error}"))?;
        if !status.success() {
            runtime::emit_backend_log(
                &app,
                "warn",
                &format!("Windows kaldırıcı çıkış kodu başarılı değil: {status}"),
            )?;
        }
    } else {
        runtime::emit_backend_log(
            &app,
            "warn",
            "Kaldırıcı kaydı bulunamadı Wine Program Ekle Kaldır açılıyor",
        )?;
        Command::new(&wine)
            .arg("uninstaller")
            .env("WINEPREFIX", &prefix_path)
            .env("WINEARCH", "win64")
            .status()
            .map_err(|error| format!("Wine kaldırma penceresi açılamadı: {error}"))?;
    }

    wait_for_wine_processes(&app, &wine, &prefix_path);
    if !target.trim().is_empty() && Path::new(&target).exists() {
        return Err("Kaldırma tamamlanmadı Uygulama dosyası hala Wine içinde duruyor".into());
    }

    database::remove_game(id, false)?;
    runtime::emit_backend_log(
        &app,
        "info",
        &format!("{} Wine içinden kaldırıldı", game.name),
    )?;
    let _ = app.emit("library-changed", id);
    Ok(())
}

fn find_uninstall_string(prefix_path: &str, game: &GameRecord, target: &str) -> Option<String> {
    let reg_files = [
        Path::new(prefix_path).join("system.reg"),
        Path::new(prefix_path).join("user.reg"),
    ];
    let target_windows = host_path_to_windows_path(prefix_path, target).unwrap_or_default();
    let install_dir_windows =
        host_path_to_windows_path(prefix_path, &game.install_dir).unwrap_or_default();
    let game_name = normalize_lookup_text(&game.name);
    let mut best: Option<(u8, String)> = None;

    for reg_file in reg_files {
        let Ok(content) = fs::read_to_string(reg_file) else {
            continue;
        };
        for block in registry_blocks(&content) {
            let Some(uninstall_string) = registry_value(&block, "UninstallString") else {
                continue;
            };
            if uninstall_string.to_ascii_lowercase().contains("wine mono") {
                continue;
            }
            let display_name = registry_value(&block, "DisplayName").unwrap_or_default();
            let install_location = registry_value(&block, "InstallLocation").unwrap_or_default();
            let display_icon = registry_value(&block, "DisplayIcon").unwrap_or_default();
            let haystack = normalize_lookup_text(&format!(
                "{display_name} {install_location} {display_icon} {uninstall_string}"
            ));
            let mut score = 0;
            if !game_name.is_empty() && haystack.contains(&game_name) {
                score += 4;
            }
            if !install_dir_windows.is_empty()
                && haystack.contains(&normalize_lookup_text(&install_dir_windows))
            {
                score += 5;
            }
            if !target_windows.is_empty()
                && haystack.contains(&normalize_lookup_text(&target_windows))
            {
                score += 3;
            }
            if score == 0 {
                continue;
            }
            if best
                .as_ref()
                .map(|(best_score, _)| score > *best_score)
                .unwrap_or(true)
            {
                best = Some((score, uninstall_string));
            }
        }
    }

    best.map(|(_, uninstall_string)| uninstall_string)
}

fn registry_blocks(content: &str) -> Vec<String> {
    let mut blocks = Vec::new();
    let mut current = String::new();
    for line in content.lines() {
        if line.starts_with('[') && !current.is_empty() {
            blocks.push(current);
            current = String::new();
        }
        current.push_str(line);
        current.push('\n');
    }
    if !current.is_empty() {
        blocks.push(current);
    }
    blocks
}

fn registry_value(block: &str, key: &str) -> Option<String> {
    let prefix = format!("\"{key}\"=");
    block.lines().find_map(|line| {
        let value = line.strip_prefix(&prefix)?;
        Some(decode_registry_value(value))
    })
}

fn decode_registry_value(value: &str) -> String {
    let value = value.strip_prefix("str(2):").unwrap_or(value).trim();
    let value = value.trim_matches('"');
    value.replace("\\\"", "\"").replace("\\\\", "\\")
}

fn host_path_to_windows_path(prefix_path: &str, host_path: &str) -> Option<String> {
    let drive_c = Path::new(prefix_path).join("drive_c");
    let relative = Path::new(host_path).strip_prefix(drive_c).ok()?;
    Some(format!(
        "C:\\{}",
        relative
            .components()
            .map(|component| component.as_os_str().to_string_lossy())
            .collect::<Vec<_>>()
            .join("\\")
    ))
}

fn normalize_lookup_text(value: &str) -> String {
    value
        .chars()
        .filter(|character| character.is_ascii_alphanumeric())
        .flat_map(char::to_lowercase)
        .collect()
}

fn extract_windows_icon_for_record(app: &AppHandle, record: &GameRecord) {
    if record.library_type != "windows-app" && record.library_type != "tool" {
        return;
    }
    let Some(target) = record
        .arguments
        .first()
        .filter(|path| has_extension(Path::new(path), "exe"))
    else {
        return;
    };
    let Ok(icon_path) = extract_windows_icon(target, record.id) else {
        return;
    };
    if let Err(error) = database::save_metadata(
        record.id,
        Some(icon_path.clone()),
        None,
        None,
        None,
        None,
        "windows-icon".into(),
    ) {
        let _ = runtime::emit_backend_log(
            app,
            "warn",
            &format!("Windows simgesi kaydedilemedi: {error}"),
        );
        return;
    }
    let _ = runtime::emit_backend_log(
        app,
        "info",
        &format!("Windows simgesi çıkarıldı: {icon_path}"),
    );
}

fn extract_windows_icon(executable: &str, id: i64) -> Result<String, String> {
    extract_windows_icon_named(executable, &format!("windows-app-{id}.ico"))
}

fn extract_windows_icon_preview(executable: &Path) -> Result<String, String> {
    let mut hasher = DefaultHasher::new();
    executable.to_string_lossy().hash(&mut hasher);
    if let Ok(metadata) = executable.metadata() {
        metadata.len().hash(&mut hasher);
        if let Ok(modified) = metadata.modified() {
            modified.hash(&mut hasher);
        }
    }
    extract_windows_icon_named(
        &executable.to_string_lossy(),
        &format!("windows-preview-{:x}.ico", hasher.finish()),
    )
}

fn extract_windows_icon_named(executable: &str, file_name: &str) -> Result<String, String> {
    let bytes = fs::read(executable).map_err(|error| format!("EXE okunamadı: {error}"))?;
    let resource_rva = pe_resource_rva(&bytes)?;
    let sections = pe_sections(&bytes)?;
    let resource_offset = rva_to_offset(resource_rva, &sections)
        .ok_or_else(|| "PE resource bölümü bulunamadı".to_string())?;
    let group = first_resource_data(&bytes, resource_offset, resource_rva, &sections, 14)?
        .ok_or_else(|| "Windows icon group bulunamadı".to_string())?;
    let icon = best_icon_from_group(&bytes[group.clone()])?;
    let image = resource_data_by_id(&bytes, resource_offset, resource_rva, &sections, 3, icon.id)?
        .ok_or_else(|| "Windows icon verisi bulunamadı".to_string())?;
    let ico = build_ico(&icon, &bytes[image]);
    let icons_dir = Path::new(&runtime::runtime_paths()?.data_dir).join("icons");
    fs::create_dir_all(&icons_dir)
        .map_err(|error| format!("Simge klasörü oluşturulamadı: {error}"))?;
    let target = icons_dir.join(file_name);
    fs::write(&target, ico).map_err(|error| format!("Simge yazılamadı: {error}"))?;
    Ok(target.to_string_lossy().into_owned())
}

#[derive(Clone)]
struct PeSection {
    virtual_address: u32,
    virtual_size: u32,
    raw_pointer: u32,
    raw_size: u32,
}

#[derive(Clone)]
struct GroupIconEntry {
    width: u8,
    height: u8,
    color_count: u8,
    planes: u16,
    bit_count: u16,
    bytes_in_res: u32,
    id: u16,
}

fn pe_resource_rva(bytes: &[u8]) -> Result<u32, String> {
    let pe_offset = read_u32(bytes, 0x3c)? as usize;
    if bytes.get(pe_offset..pe_offset + 4) != Some(b"PE\0\0") {
        return Err("PE imzası bulunamadı".into());
    }
    let optional_offset = pe_offset + 24;
    let magic = read_u16(bytes, optional_offset)?;
    let data_directory = match magic {
        0x10b => optional_offset + 96,
        0x20b => optional_offset + 112,
        _ => return Err("PE optional header okunamadı".into()),
    };
    read_u32(bytes, data_directory + 16)
}

fn pe_sections(bytes: &[u8]) -> Result<Vec<PeSection>, String> {
    let pe_offset = read_u32(bytes, 0x3c)? as usize;
    let section_count = read_u16(bytes, pe_offset + 6)? as usize;
    let optional_size = read_u16(bytes, pe_offset + 20)? as usize;
    let section_offset = pe_offset + 24 + optional_size;
    let mut sections = Vec::new();
    for index in 0..section_count {
        let offset = section_offset + index * 40;
        sections.push(PeSection {
            virtual_size: read_u32(bytes, offset + 8)?,
            virtual_address: read_u32(bytes, offset + 12)?,
            raw_size: read_u32(bytes, offset + 16)?,
            raw_pointer: read_u32(bytes, offset + 20)?,
        });
    }
    Ok(sections)
}

fn rva_to_offset(rva: u32, sections: &[PeSection]) -> Option<usize> {
    sections.iter().find_map(|section| {
        let size = section.virtual_size.max(section.raw_size);
        if rva >= section.virtual_address && rva < section.virtual_address.saturating_add(size) {
            Some((section.raw_pointer + (rva - section.virtual_address)) as usize)
        } else {
            None
        }
    })
}

fn first_resource_data(
    bytes: &[u8],
    root: usize,
    resource_rva: u32,
    sections: &[PeSection],
    type_id: u16,
) -> Result<Option<std::ops::Range<usize>>, String> {
    let Some(type_dir) = resource_child_dir(bytes, root, root, type_id)? else {
        return Ok(None);
    };
    let Some(name_dir) = first_resource_child_dir(bytes, root, type_dir)? else {
        return Ok(None);
    };
    first_resource_child_data(bytes, root, resource_rva, sections, name_dir)
}

fn resource_data_by_id(
    bytes: &[u8],
    root: usize,
    resource_rva: u32,
    sections: &[PeSection],
    type_id: u16,
    name_id: u16,
) -> Result<Option<std::ops::Range<usize>>, String> {
    let Some(type_dir) = resource_child_dir(bytes, root, root, type_id)? else {
        return Ok(None);
    };
    let Some(name_dir) = resource_child_dir(bytes, root, type_dir, name_id)? else {
        return Ok(None);
    };
    first_resource_child_data(bytes, root, resource_rva, sections, name_dir)
}

fn resource_child_dir(
    bytes: &[u8],
    root: usize,
    dir: usize,
    id: u16,
) -> Result<Option<usize>, String> {
    for entry in resource_entries(bytes, dir)? {
        if !entry.is_named && entry.id == id && entry.is_dir {
            return Ok(Some(root + entry.offset));
        }
    }
    Ok(None)
}

fn first_resource_child_dir(
    bytes: &[u8],
    root: usize,
    dir: usize,
) -> Result<Option<usize>, String> {
    for entry in resource_entries(bytes, dir)? {
        if entry.is_dir {
            return Ok(Some(root + entry.offset));
        }
    }
    Ok(None)
}

fn first_resource_child_data(
    bytes: &[u8],
    root: usize,
    resource_rva: u32,
    sections: &[PeSection],
    dir: usize,
) -> Result<Option<std::ops::Range<usize>>, String> {
    for entry in resource_entries(bytes, dir)? {
        if !entry.is_dir {
            let data_entry = root + entry.offset;
            let data_rva = read_u32(bytes, data_entry)?;
            let size = read_u32(bytes, data_entry + 4)? as usize;
            let Some(start) = rva_to_offset(data_rva, sections) else {
                continue;
            };
            return Ok(Some(start..start + size));
        }
        if let Some(range) =
            first_resource_child_data(bytes, root, resource_rva, sections, root + entry.offset)?
        {
            return Ok(Some(range));
        }
    }
    let _ = resource_rva;
    Ok(None)
}

struct ResourceEntry {
    id: u16,
    is_named: bool,
    is_dir: bool,
    offset: usize,
}

fn resource_entries(bytes: &[u8], dir: usize) -> Result<Vec<ResourceEntry>, String> {
    let named = read_u16(bytes, dir + 12)? as usize;
    let ids = read_u16(bytes, dir + 14)? as usize;
    let mut entries = Vec::new();
    for index in 0..named + ids {
        let offset = dir + 16 + index * 8;
        let name = read_u32(bytes, offset)?;
        let value = read_u32(bytes, offset + 4)?;
        entries.push(ResourceEntry {
            id: (name & 0xffff) as u16,
            is_named: name & 0x8000_0000 != 0,
            is_dir: value & 0x8000_0000 != 0,
            offset: (value & 0x7fff_ffff) as usize,
        });
    }
    Ok(entries)
}

fn best_icon_from_group(bytes: &[u8]) -> Result<GroupIconEntry, String> {
    let count = read_u16(bytes, 4)? as usize;
    let mut icons = Vec::new();
    for index in 0..count {
        let offset = 6 + index * 14;
        icons.push(GroupIconEntry {
            width: *bytes.get(offset).unwrap_or(&0),
            height: *bytes.get(offset + 1).unwrap_or(&0),
            color_count: *bytes.get(offset + 2).unwrap_or(&0),
            planes: read_u16(bytes, offset + 4)?,
            bit_count: read_u16(bytes, offset + 6)?,
            bytes_in_res: read_u32(bytes, offset + 8)?,
            id: read_u16(bytes, offset + 12)?,
        });
    }
    icons
        .into_iter()
        .max_by_key(|icon| {
            let width = if icon.width == 0 {
                256
            } else {
                icon.width as u32
            };
            let height = if icon.height == 0 {
                256
            } else {
                icon.height as u32
            };
            (width * height, icon.bit_count, icon.bytes_in_res)
        })
        .ok_or_else(|| "Icon group boş".into())
}

fn build_ico(icon: &GroupIconEntry, image: &[u8]) -> Vec<u8> {
    let mut ico = Vec::with_capacity(22 + image.len());
    ico.extend_from_slice(&0u16.to_le_bytes());
    ico.extend_from_slice(&1u16.to_le_bytes());
    ico.extend_from_slice(&1u16.to_le_bytes());
    ico.push(icon.width);
    ico.push(icon.height);
    ico.push(icon.color_count);
    ico.push(0);
    ico.extend_from_slice(&icon.planes.to_le_bytes());
    ico.extend_from_slice(&icon.bit_count.to_le_bytes());
    ico.extend_from_slice(&(image.len() as u32).to_le_bytes());
    ico.extend_from_slice(&22u32.to_le_bytes());
    ico.extend_from_slice(image);
    ico
}

fn read_u16(bytes: &[u8], offset: usize) -> Result<u16, String> {
    let data = bytes
        .get(offset..offset + 2)
        .ok_or_else(|| "Dosya beklenenden kısa".to_string())?;
    Ok(u16::from_le_bytes([data[0], data[1]]))
}

fn read_u32(bytes: &[u8], offset: usize) -> Result<u32, String> {
    let data = bytes
        .get(offset..offset + 4)
        .ok_or_else(|| "Dosya beklenenden kısa".to_string())?;
    Ok(u32::from_le_bytes([data[0], data[1], data[2], data[3]]))
}

#[tauri::command]
fn game_settings(id: i64) -> Result<GameRecord, String> {
    database::get_game_by_id(id)
}

#[tauri::command]
fn open_game_settings_window(app: AppHandle, id: i64) -> Result<(), String> {
    let game = database::get_game_by_id(id)?;
    let label = format!("game-settings-{id}");
    if let Some(window) = app.get_webview_window(&label) {
        window
            .set_focus()
            .map_err(|error| format!("Cannot focus settings window: {error}"))?;
        return Ok(());
    }

    let url = WebviewUrl::App(format!("game-settings.html?id={id}").into());
    WebviewWindowBuilder::new(&app, label, url)
        .title(format!("{} Ayarları", game.name))
        .inner_size(760.0, 720.0)
        .min_inner_size(640.0, 560.0)
        .resizable(true)
        .decorations(false)
        .build()
        .map_err(|error| format!("Cannot open settings window: {error}"))?;
    Ok(())
}

#[tauri::command]
fn update_game_mode(id: i64, options: GameModeOptions) -> Result<GameRecord, String> {
    database::update_game_mode(id, &options)
}

#[tauri::command]
fn clear_game_session(app: AppHandle, id: i64) -> Result<GameRecord, String> {
    let game = database::clear_play_session(id)?;
    runtime::emit_backend_log(
        &app,
        "info",
        &format!("Çalışıyor durumu temizlendi: {}", game.name),
    )?;
    Ok(game)
}

#[tauri::command]
fn update_game_settings(
    app: AppHandle,
    id: i64,
    settings: GameSettingsUpdate,
) -> Result<GameRecord, String> {
    let game = database::update_game_settings(id, &settings)?;
    apply_windows_version(&app, &game)?;
    let _ = app.emit("library-changed", &game);
    Ok(game)
}

fn apply_windows_version(app: &AppHandle, game: &GameRecord) -> Result<(), String> {
    if game.runner != "wine" {
        return Ok(());
    }
    let Some(prefix_path) = &game.prefix_path else {
        return Ok(());
    };

    let Ok(wine) = wine_executable() else {
        runtime::emit_backend_log(app, "warn", "Wine yok Windows sürümü prefix'e uygulanamadı")?;
        return Ok(());
    };

    let version = normalized_windows_version(&game.windows_version);
    let status = Command::new(wine)
        .env("WINEPREFIX", prefix_path)
        .env("WINEDEBUG", "-all")
        .arg("winecfg")
        .arg("/v")
        .arg(version)
        .status()
        .map_err(|error| format!("Windows sürümü uygulanamadı: {error}"))?;

    if !status.success() {
        return Err(format!("winecfg /v {version} exited with status {status}."));
    }

    runtime::emit_backend_log(
        app,
        "info",
        &format!("{} için Windows sürümü uygulandı: {version}", game.name),
    )?;
    Ok(())
}

fn normalized_windows_version(value: &str) -> &'static str {
    match value {
        "winxp" | "Windows XP" => "winxp",
        "win7" | "Windows 7" => "win7",
        "win11" | "Windows 11" => "win11",
        _ => "win10",
    }
}

#[tauri::command]
fn list_settings() -> Result<Vec<AppSetting>, String> {
    database::list_settings()
}

#[tauri::command]
fn set_setting(key: String, value: String) -> Result<AppSetting, String> {
    database::set_setting(key, value)
}

#[tauri::command]
fn fetch_game_metadata(id: i64) -> Result<MetadataResult, String> {
    metadata::fetch_from_steamgriddb(id)
}

#[tauri::command]
fn set_manual_cover(id: i64, cover_path: String) -> Result<MetadataResult, String> {
    metadata::set_manual_cover(id, cover_path)
}

#[tauri::command]
fn check_component_updates() -> Result<Vec<ComponentUpdate>, String> {
    updates::check_components()
}

#[tauri::command]
async fn update_component(
    app: AppHandle,
    request: ComponentUpdateRequest,
) -> Result<ComponentUpdate, String> {
    tauri::async_runtime::spawn_blocking(move || updates::update_component(&app, request))
        .await
        .map_err(|error| format!("Güncelleme görevi tamamlanamadı: {error}"))?
}

#[tauri::command]
async fn remove_component(app: AppHandle, component: String) -> Result<ComponentUpdate, String> {
    tauri::async_runtime::spawn_blocking(move || updates::remove_component(&app, component))
        .await
        .map_err(|error| format!("Kaldırma görevi tamamlanamadı: {error}"))?
}

#[tauri::command]
fn cancel_component_download() -> Result<(), String> {
    runtime::cancel_download();
    Ok(())
}

#[tauri::command]
fn save_compatibility_settings(
    id: i64,
    settings: CompatibilitySettings,
) -> Result<TroubleshootingReport, String> {
    let game = database::get_game_by_id(id)?;
    compatibility::save_settings(&game, settings)
}

#[tauri::command]
fn compatibility_report(id: i64) -> Result<TroubleshootingReport, String> {
    let game = database::get_game_by_id(id)?;
    compatibility::report(&game)
}

#[tauri::command]
fn append_compatibility_error(id: i64, message: String) -> Result<TroubleshootingReport, String> {
    let game = database::get_game_by_id(id)?;
    compatibility::append_error(&game, message)
}

#[tauri::command]
fn fetch_protondb_summary(app_id: String) -> Result<ProtonDbSummary, String> {
    compatibility::protondb_summary(app_id)
}

#[tauri::command]
fn scan_steam() -> Result<SteamScan, String> {
    steam::scan()
}

#[tauri::command]
fn sync_steam_library(app: AppHandle) -> Result<Vec<GameRecord>, String> {
    database::initialize()?;
    let scan = steam::scan()?;
    let proton = steam::preferred_proton(&scan);
    let mut records = Vec::new();

    for game in &scan.games {
        records.push(database::upsert_steam_game(game, proton.as_deref())?);
    }

    runtime::emit_backend_log(
        &app,
        "info",
        &format!("Steam sync completed: {} games.", records.len()),
    )?;
    Ok(records)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            app.handle()
                .plugin(tauri_plugin_updater::Builder::new().build())?;
            if let Some(window) = app.get_webview_window("main") {
                let icon = tauri::image::Image::from_bytes(include_bytes!("../icons/icon.png"))?;
                window.set_icon(icon)?;
            }
            if cfg!(debug_assertions) {
                app.handle().plugin(
                    tauri_plugin_log::Builder::default()
                        .level(log::LevelFilter::Info)
                        .build(),
                )?;
            }
            Ok(())
        })
        .on_window_event(|window, event| {
            if window.label() != "main" || !matches!(event, WindowEvent::CloseRequested { .. }) {
                return;
            }

            for (label, child) in window.app_handle().webview_windows() {
                if label.starts_with("game-settings-") {
                    let _ = child.close();
                }
            }
        })
        .invoke_handler(tauri::generate_handler![
            initialize_runtime,
            download_portable_runner,
            initialize_emulators,
            download_portable_emulator,
            select_game_runner,
            create_wine_prefix,
            install_dxvk_vkd3d,
            add_game_installation,
            pick_file,
            pick_folder,
            windows_file_preview,
            fullscreen_tool_status,
            install_kdotool,
            install_gamescope,
            remove_gamescope,
            remove_fullscreen_tool,
            run_game_installer,
            list_games,
            launch_game,
            remove_game,
            refresh_windows_app_icon,
            uninstall_windows_app,
            game_settings,
            open_game_settings_window,
            update_game_mode,
            clear_game_session,
            update_game_settings,
            install_cncnet_for_game,
            list_settings,
            set_setting,
            fetch_game_metadata,
            set_manual_cover,
            check_component_updates,
            update_component,
            remove_component,
            cancel_component_download,
            save_compatibility_settings,
            compatibility_report,
            append_compatibility_error,
            fetch_protondb_summary,
            scan_steam,
            sync_steam_library
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
