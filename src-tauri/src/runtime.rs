use serde::{Deserialize, Serialize};
use std::{
    env,
    fs::{self, OpenOptions},
    io::Write,
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
    process::Command,
    sync::atomic::{AtomicBool, Ordering},
    thread,
    time::{Duration, SystemTime, UNIX_EPOCH},
};
use tauri::{AppHandle, Emitter};

const APP_DIR_NAME: &str = "ardali-gaming";
static DOWNLOAD_CANCELLED: AtomicBool = AtomicBool::new(false);

#[derive(Debug, Clone, Serialize)]
pub struct RuntimePaths {
    pub data_dir: String,
    pub database_path: String,
    pub wine_dir: String,
    pub proton_dir: String,
    pub emulators_dir: String,
    pub openra_dir: String,
    pub dosbox_x_dir: String,
    pub cncnet_dir: String,
    pub prefixes_dir: String,
    pub downloads_dir: String,
    pub logs_dir: String,
    pub components_dir: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct RuntimeStatus {
    pub paths: RuntimePaths,
    pub portable_wine_ready: bool,
    pub portable_proton_ready: bool,
    pub system_wine_version: Option<String>,
    pub system_proton_version: Option<String>,
    pub system_wine_compatible: bool,
    pub openra_ready: bool,
    pub dosbox_x_ready: bool,
    pub cncnet_ready: bool,
    pub dxvk_ready: bool,
    pub vkd3d_ready: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct BackendLog {
    pub level: String,
    pub message: String,
    pub timestamp: u64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DownloadProgress {
    pub kind: String,
    pub percent: u8,
    pub downloaded_bytes: u64,
    pub total_bytes: Option<u64>,
    pub status: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct PrefixInfo {
    pub id: String,
    pub name: String,
    pub path: String,
    pub wineprefix: String,
    pub wineboot_ran: bool,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RunnerKind {
    Wine,
    Proton,
    Dxvk,
    Vkd3d,
    Openra,
    DosboxX,
    Cncnet,
}

impl RunnerKind {
    fn as_str(&self) -> &'static str {
        match self {
            Self::Wine => "wine",
            Self::Proton => "proton",
            Self::Dxvk => "dxvk",
            Self::Vkd3d => "vkd3d",
            Self::Openra => "openra",
            Self::DosboxX => "dosbox-x",
            Self::Cncnet => "cncnet",
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum GameKind {
    WindowsExe,
    WindowsMsi,
    Steam,
    OpenRaRedAlert,
    OpenRaTiberianDawn,
    OpenRaDune2000,
    Dos,
    Cncnet,
}

impl GameKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::WindowsExe => "windows-exe",
            Self::WindowsMsi => "windows-msi",
            Self::Steam => "steam",
            Self::OpenRaRedAlert => "open-ra-red-alert",
            Self::OpenRaTiberianDawn => "open-ra-tiberian-dawn",
            Self::OpenRaDune2000 => "open-ra-dune2000",
            Self::Dos => "dos",
            Self::Cncnet => "cncnet",
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct RunnerSelection {
    pub runner: String,
    pub executable: Option<String>,
    pub arguments: Vec<String>,
    pub notes: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct EmulatorStatus {
    pub emulators_dir: String,
    pub openra_dir: String,
    pub dosbox_x_dir: String,
    pub cncnet_dir: String,
    pub openra_red_alert_ready: bool,
    pub openra_tiberian_dawn_ready: bool,
    pub openra_dune_2000_ready: bool,
    pub dosbox_x_ready: bool,
    pub cncnet_ready: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct DownloadResult {
    pub kind: String,
    pub archive_path: String,
    pub install_dir: String,
    pub extracted: bool,
}

pub fn initialize(app: &AppHandle) -> Result<RuntimeStatus, String> {
    let paths = paths()?;
    for dir in [
        &paths.data_dir,
        &paths.wine_dir,
        &paths.proton_dir,
        &paths.emulators_dir,
        &paths.openra_dir,
        &paths.dosbox_x_dir,
        &paths.prefixes_dir,
        &paths.downloads_dir,
        &paths.logs_dir,
        &paths.components_dir,
    ] {
        fs::create_dir_all(dir).map_err(|error| format!("Cannot create {dir}: {error}"))?;
    }

    emit_log(app, "info", "Portable runtime directories are ready.")?;
    Ok(status_from_paths(paths))
}

pub fn download_runner(
    app: &AppHandle,
    kind: RunnerKind,
    url: String,
    file_name: Option<String>,
) -> Result<DownloadResult, String> {
    if url.trim().is_empty() {
        return Err("Download URL cannot be empty.".into());
    }
    DOWNLOAD_CANCELLED.store(false, Ordering::SeqCst);

    let paths = paths()?;
    fs::create_dir_all(&paths.downloads_dir)
        .map_err(|error| format!("Cannot create downloads directory: {error}"))?;

    let archive_name = file_name.unwrap_or_else(|| default_archive_name(&kind, &url));
    let archive_path = PathBuf::from(&paths.downloads_dir).join(archive_name);
    emit_log(
        app,
        "info",
        &format!("Downloading {} portable archive.", kind.as_str()),
    )?;

    let total_bytes = remote_content_length(&url);
    emit_download_progress(app, &kind, 1, 0, total_bytes, "İndirme başladı")?;

    let mut child = Command::new("curl")
        .args(["-L", "--fail", "--silent", "--show-error", "-o"])
        .arg(&archive_path)
        .arg(url)
        .spawn()
        .map_err(|error| format!("Failed to start curl: {error}"))?;

    let mut last_percent = 1;
    loop {
        if DOWNLOAD_CANCELLED.load(Ordering::SeqCst) {
            let _ = child.kill();
            let _ = child.wait();
            let downloaded = archive_path
                .metadata()
                .map(|metadata| metadata.len())
                .unwrap_or(0);
            let _ = fs::remove_file(&archive_path);
            emit_download_progress(
                app,
                &kind,
                last_percent,
                downloaded,
                total_bytes,
                "İptal edildi",
            )?;
            return Err("İndirme iptal edildi".into());
        }

        match child
            .try_wait()
            .map_err(|error| format!("Cannot read curl status: {error}"))?
        {
            Some(status) => {
                if !status.success() {
                    return Err(format!("curl exited with status {status}."));
                }
                break;
            }
            None => {
                let downloaded = archive_path
                    .metadata()
                    .map(|metadata| metadata.len())
                    .unwrap_or(0);
                let percent = download_percent(downloaded, total_bytes).max(last_percent);
                if percent > last_percent {
                    last_percent = percent;
                    emit_download_progress(
                        app,
                        &kind,
                        percent,
                        downloaded,
                        total_bytes,
                        "İndiriliyor",
                    )?;
                }
                thread::sleep(Duration::from_millis(400));
            }
        }
    }

    let downloaded = archive_path
        .metadata()
        .map(|metadata| metadata.len())
        .unwrap_or(0);
    emit_download_progress(
        app,
        &kind,
        100,
        downloaded,
        total_bytes,
        "İndirme tamamlandı",
    )?;

    let install_dir = install_dir(&paths, &kind);
    fs::create_dir_all(&install_dir)
        .map_err(|error| format!("Cannot create install directory: {error}"))?;

    emit_download_progress(
        app,
        &kind,
        100,
        downloaded,
        total_bytes,
        "Kurulum hazırlanıyor",
    )?;
    let extracted = install_downloaded_file(app, &archive_path, &install_dir)?;
    emit_download_progress(
        app,
        &kind,
        100,
        downloaded,
        total_bytes,
        "Kurulum tamamlandı",
    )?;
    emit_log(
        app,
        "info",
        &format!("{} archive stored under isolated app data.", kind.as_str()),
    )?;

    Ok(DownloadResult {
        kind: kind.as_str().to_string(),
        archive_path: archive_path.to_string_lossy().into_owned(),
        install_dir: install_dir.to_string_lossy().into_owned(),
        extracted,
    })
}

pub fn cancel_download() {
    DOWNLOAD_CANCELLED.store(true, Ordering::SeqCst);
}

pub fn initialize_emulators(app: &AppHandle) -> Result<EmulatorStatus, String> {
    let paths = paths()?;
    for dir in [
        &paths.emulators_dir,
        &paths.openra_dir,
        &paths.dosbox_x_dir,
        &paths.cncnet_dir,
    ] {
        fs::create_dir_all(dir).map_err(|error| format!("Cannot create {dir}: {error}"))?;
    }

    write_openra_catalog(&paths)?;
    write_dosbox_config(&paths)?;
    emit_log(app, "info", "Portable emulator directories are ready.")?;
    Ok(emulator_status(&paths))
}

pub fn download_emulator(
    app: &AppHandle,
    kind: RunnerKind,
    url: String,
    file_name: Option<String>,
) -> Result<DownloadResult, String> {
    match kind {
        RunnerKind::Openra | RunnerKind::DosboxX | RunnerKind::Cncnet => {
            download_runner(app, kind, url, file_name)
        }
        _ => Err("Only openra, cncnet and dosbox-x are valid emulator downloads.".into()),
    }
}

pub fn select_runner(
    game_kind: GameKind,
    target_path: Option<String>,
) -> Result<RunnerSelection, String> {
    let paths = paths()?;
    let target = target_path.unwrap_or_default();
    let selection = match game_kind {
        GameKind::WindowsExe => RunnerSelection {
            runner: "wine".into(),
            executable: Some(preferred_wine_executable(&paths)),
            arguments: vec![target],
            notes: "Windows EXE installers and games use the isolated Wine runner".into(),
        },
        GameKind::WindowsMsi => RunnerSelection {
            runner: "wine".into(),
            executable: Some(preferred_wine_executable(&paths)),
            arguments: vec!["msiexec".into(), "/i".into(), target],
            notes: "Windows MSI packages use msiexec through the isolated Wine runner".into(),
        },
        GameKind::Steam => RunnerSelection {
            runner: "proton".into(),
            executable: Some(preferred_proton_executable(&paths)),
            arguments: vec![target],
            notes: "Steam games will prefer the detected Steam Proton runtime in a later step."
                .into(),
        },
        GameKind::OpenRaRedAlert => openra_selection(&paths, "red-alert", target),
        GameKind::OpenRaTiberianDawn => openra_selection(&paths, "tiberian-dawn", target),
        GameKind::OpenRaDune2000 => openra_selection(&paths, "dune-2000", target),
        GameKind::Dos => RunnerSelection {
            runner: "dosbox-x".into(),
            executable: Some(
                Path::new(&paths.dosbox_x_dir)
                    .join("current/dosbox-x")
                    .to_string_lossy()
                    .into_owned(),
            ),
            arguments: if target.is_empty() {
                vec![]
            } else {
                vec![target]
            },
            notes: "DOS games are routed to the portable DOSBox-X runner.".into(),
        },
        GameKind::Cncnet => RunnerSelection {
            runner: "cncnet".into(),
            executable: Some(
                Path::new(&paths.cncnet_dir)
                    .join("current/cncnet")
                    .to_string_lossy()
                    .into_owned(),
            ),
            arguments: if target.is_empty() {
                vec![]
            } else {
                vec![target]
            },
            notes: "CnCNet games are routed to the portable CnCNet client.".into(),
        },
    };

    Ok(selection)
}

fn preferred_wine_executable(paths: &RuntimePaths) -> String {
    let portable = Path::new(&paths.wine_dir).join("current/bin/wine");
    if portable.exists() {
        return path_string(portable);
    }

    command_path("wine").unwrap_or_else(|| path_string(portable))
}

fn preferred_proton_executable(paths: &RuntimePaths) -> String {
    let portable = Path::new(&paths.proton_dir).join("current/proton");
    if portable.exists() {
        return path_string(portable);
    }

    command_path("proton").unwrap_or_else(|| path_string(portable))
}

pub fn create_prefix(app: &AppHandle, game_id: String, name: String) -> Result<PrefixInfo, String> {
    let paths = paths()?;
    let prefix_id = sanitize_id(&game_id);
    if prefix_id.is_empty() {
        return Err("Game id must include at least one letter or number.".into());
    }

    let root = PathBuf::from(&paths.prefixes_dir).join(&prefix_id);
    let wineprefix = root.join("pfx");
    fs::create_dir_all(&wineprefix)
        .map_err(|error| format!("Cannot create Wine prefix directory: {error}"))?;

    let wineboot_ran = run_wineboot_if_available(app, &paths, &wineprefix)?;
    write_prefix_manifest(&root, &prefix_id, &name, &wineprefix, wineboot_ran)?;

    emit_log(
        app,
        "info",
        &format!("Created isolated Wine prefix for {name}"),
    )?;

    Ok(PrefixInfo {
        id: prefix_id,
        name,
        path: root.to_string_lossy().into_owned(),
        wineprefix: wineprefix.to_string_lossy().into_owned(),
        wineboot_ran,
    })
}

pub fn install_graphics_components(
    app: &AppHandle,
    prefix_id: String,
) -> Result<PrefixInfo, String> {
    let paths = paths()?;
    let prefix_id = sanitize_id(&prefix_id);
    let root = PathBuf::from(&paths.prefixes_dir).join(&prefix_id);
    let wineprefix = root.join("pfx");
    if !wineprefix.exists() {
        return Err(format!("Prefix does not exist: {prefix_id}"));
    }

    let marker_dir = root.join("components");
    fs::create_dir_all(&marker_dir)
        .map_err(|error| format!("Cannot create component marker directory: {error}"))?;
    fs::write(
        marker_dir.join("dxvk.requested"),
        "pending portable DXVK bundle\n",
    )
    .map_err(|error| format!("Cannot write DXVK marker: {error}"))?;
    fs::write(
        marker_dir.join("vkd3d.requested"),
        "pending portable VKD3D bundle\n",
    )
    .map_err(|error| format!("Cannot write VKD3D marker: {error}"))?;

    let dxvk_installed = run_component_setup(
        app,
        &Path::new(&paths.components_dir).join("dxvk/current"),
        "setup_dxvk.sh",
        &wineprefix,
    )?;
    let vkd3d_installed = run_component_setup(
        app,
        &Path::new(&paths.components_dir).join("vkd3d/current"),
        "setup_vkd3d_proton.sh",
        &wineprefix,
    )?;

    if dxvk_installed {
        fs::write(marker_dir.join("dxvk.installed"), "installed\n")
            .map_err(|error| format!("Cannot write DXVK install marker: {error}"))?;
    }
    if vkd3d_installed {
        fs::write(marker_dir.join("vkd3d.installed"), "installed\n")
            .map_err(|error| format!("Cannot write VKD3D install marker: {error}"))?;
    }

    let status = match (dxvk_installed, vkd3d_installed) {
        (true, true) => "DXVK and VKD3D were injected into this isolated prefix",
        (true, false) => "DXVK was injected VKD3D is waiting for its portable bundle",
        (false, true) => "VKD3D was injected DXVK is waiting for its portable bundle",
        (false, false) => "DXVK and VKD3D installation was queued for this isolated prefix",
    };
    emit_log(app, "info", status)?;

    Ok(PrefixInfo {
        id: prefix_id,
        name: "Unknown".into(),
        path: root.to_string_lossy().into_owned(),
        wineprefix: wineprefix.to_string_lossy().into_owned(),
        wineboot_ran: false,
    })
}

fn run_component_setup(
    app: &AppHandle,
    component_dir: &Path,
    script_name: &str,
    wineprefix: &Path,
) -> Result<bool, String> {
    let Some(script) = find_file(component_dir, script_name)? else {
        return Ok(false);
    };

    let wine_bin_dir = Path::new(&paths()?.wine_dir).join("current/bin");
    let current_path = env::var("PATH").unwrap_or_default();
    let setup_path = if wine_bin_dir.exists() {
        format!("{}:{current_path}", wine_bin_dir.to_string_lossy())
    } else {
        current_path
    };

    let status = Command::new("sh")
        .arg(script)
        .arg("install")
        .env("WINEPREFIX", wineprefix)
        .env("WINEDEBUG", "-all")
        .env("PATH", setup_path)
        .status()
        .map_err(|error| format!("Failed to run {script_name}: {error}"))?;

    if !status.success() {
        emit_log(
            app,
            "warn",
            &format!("{script_name} exited with status {status}."),
        )?;
        return Ok(false);
    }

    Ok(true)
}

fn find_file(root: &Path, file_name: &str) -> Result<Option<PathBuf>, String> {
    if !root.exists() {
        return Ok(None);
    }

    for entry in
        fs::read_dir(root).map_err(|error| format!("Cannot read component dir: {error}"))?
    {
        let entry = entry.map_err(|error| format!("Cannot read component entry: {error}"))?;
        let path = entry.path();
        if path.file_name().and_then(|name| name.to_str()) == Some(file_name) {
            return Ok(Some(path));
        }
        if path.is_dir() {
            if let Some(found) = find_file(&path, file_name)? {
                return Ok(Some(found));
            }
        }
    }

    Ok(None)
}

fn paths() -> Result<RuntimePaths, String> {
    let home = env::var("HOME").map_err(|_| "HOME environment variable is not set.".to_string())?;
    let data_dir = PathBuf::from(home).join(".local/share").join(APP_DIR_NAME);
    let wine_dir = data_dir.join("wine");
    let proton_dir = wine_dir.join("proton");
    let emulators_dir = data_dir.join("emulators");
    let openra_dir = emulators_dir.join("openra");
    let dosbox_x_dir = emulators_dir.join("dosbox-x");
    let cncnet_dir = emulators_dir.join("cncnet");
    let prefixes_dir = data_dir.join("prefixes");
    let downloads_dir = data_dir.join("downloads");
    let logs_dir = data_dir.join("logs");
    let components_dir = wine_dir.join("components");

    Ok(RuntimePaths {
        database_path: path_string(data_dir.join("games.db")),
        data_dir: path_string(data_dir),
        wine_dir: path_string(wine_dir),
        proton_dir: path_string(proton_dir),
        emulators_dir: path_string(emulators_dir),
        openra_dir: path_string(openra_dir),
        dosbox_x_dir: path_string(dosbox_x_dir),
        cncnet_dir: path_string(cncnet_dir),
        prefixes_dir: path_string(prefixes_dir),
        downloads_dir: path_string(downloads_dir),
        logs_dir: path_string(logs_dir),
        components_dir: path_string(components_dir),
    })
}

fn status_from_paths(paths: RuntimePaths) -> RuntimeStatus {
    let system_wine_version = command_first_line("wine", &["--version"]);
    let system_proton_version = command_first_line("proton", &["--version"]);
    let system_wine_compatible = system_wine_version
        .as_deref()
        .and_then(parse_wine_major_version)
        .map(|major| major >= 7)
        .unwrap_or(false);

    RuntimeStatus {
        portable_wine_ready: Path::new(&paths.wine_dir).join("current/bin/wine").exists(),
        portable_proton_ready: Path::new(&paths.proton_dir).join("current/proton").exists(),
        system_wine_version,
        system_proton_version,
        system_wine_compatible,
        openra_ready: Path::new(&paths.openra_dir)
            .join("OpenRA-Red-Alert-x86_64.AppImage")
            .exists()
            || Path::new(&paths.openra_dir)
                .join("OpenRA-Tiberian-Dawn-x86_64.AppImage")
                .exists()
            || Path::new(&paths.openra_dir)
                .join("OpenRA-Dune-2000-x86_64.AppImage")
                .exists(),
        dosbox_x_ready: Path::new(&paths.dosbox_x_dir)
            .join("current/dosbox-x")
            .exists(),
        cncnet_ready: Path::new(&paths.cncnet_dir).join("current/cncnet").exists(),
        dxvk_ready: Path::new(&paths.components_dir)
            .join("dxvk/current")
            .exists(),
        vkd3d_ready: Path::new(&paths.components_dir)
            .join("vkd3d/current")
            .exists(),
        paths,
    }
}

fn command_first_line(command: &str, args: &[&str]) -> Option<String> {
    let output = Command::new(command).args(args).output().ok()?;
    if !output.status.success() {
        return None;
    }

    String::from_utf8_lossy(&output.stdout)
        .lines()
        .next()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(ToString::to_string)
}

fn command_path(command: &str) -> Option<String> {
    let output = Command::new("sh")
        .arg("-c")
        .arg(format!("command -v {} 2>/dev/null", shell_quote(command)))
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }

    String::from_utf8_lossy(&output.stdout)
        .lines()
        .next()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(ToString::to_string)
}

fn shell_quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', "'\\''"))
}

fn parse_wine_major_version(value: &str) -> Option<u32> {
    value
        .split(|character: char| !character.is_ascii_digit())
        .find(|part| !part.is_empty())
        .and_then(|part| part.parse().ok())
}

fn run_wineboot_if_available(
    app: &AppHandle,
    paths: &RuntimePaths,
    wineprefix: &Path,
) -> Result<bool, String> {
    let wine = Path::new(&paths.wine_dir).join("current/bin/wine");
    if !wine.exists() {
        emit_log(
            app,
            "warn",
            "Portable Wine is not installed yet prefix folder was created without wineboot",
        )?;
        return Ok(false);
    }

    let status = Command::new(wine)
        .env("WINEPREFIX", wineprefix)
        .arg("wineboot")
        .arg("-u")
        .status()
        .map_err(|error| format!("Failed to run portable wineboot: {error}"))?;

    if !status.success() {
        return Err(format!("portable wineboot exited with status {status}."));
    }

    Ok(true)
}

fn write_prefix_manifest(
    root: &Path,
    id: &str,
    name: &str,
    wineprefix: &Path,
    wineboot_ran: bool,
) -> Result<(), String> {
    let manifest = serde_json::json!({
      "id": id,
      "name": name,
      "runner": "wine",
      "wineprefix": wineprefix.to_string_lossy(),
      "winebootRan": wineboot_ran
    });
    let bytes = serde_json::to_vec_pretty(&manifest)
        .map_err(|error| format!("Cannot serialize prefix manifest: {error}"))?;
    fs::write(root.join("prefix.json"), bytes)
        .map_err(|error| format!("Cannot write prefix manifest: {error}"))
}

fn extract_archive(
    app: &AppHandle,
    archive_path: &Path,
    install_dir: &Path,
) -> Result<bool, String> {
    let archive = archive_path.to_string_lossy();
    let can_extract = archive.ends_with(".tar.gz")
        || archive.ends_with(".tgz")
        || archive.ends_with(".tar.xz")
        || archive.ends_with(".tar.zst");
    if !can_extract {
        emit_log(
            app,
            "warn",
            "Archive was downloaded but automatic extraction is not supported yet.",
        )?;
        return Ok(false);
    }

    let status = Command::new("tar")
        .arg("-xf")
        .arg(archive_path)
        .arg("-C")
        .arg(install_dir)
        .status()
        .map_err(|error| format!("Failed to start tar: {error}"))?;

    if !status.success() {
        return Err(format!("tar exited with status {status}."));
    }

    Ok(true)
}

fn install_downloaded_file(
    app: &AppHandle,
    archive_path: &Path,
    install_dir: &Path,
) -> Result<bool, String> {
    let archive = archive_path.to_string_lossy();
    if archive.ends_with(".AppImage") {
        let file_name = archive_path
            .file_name()
            .ok_or_else(|| "Downloaded AppImage has no file name.".to_string())?;
        let installed_path = if install_dir
            .parent()
            .and_then(Path::file_name)
            .and_then(|name| name.to_str())
            == Some("dosbox-x")
        {
            install_dir.join("dosbox-x")
        } else if install_dir
            .parent()
            .and_then(Path::file_name)
            .and_then(|name| name.to_str())
            == Some("cncnet")
        {
            install_dir.join("cncnet")
        } else {
            install_dir.join(file_name)
        };
        fs::copy(archive_path, &installed_path)
            .map_err(|error| format!("Cannot install AppImage: {error}"))?;
        let mut permissions = fs::metadata(&installed_path)
            .map_err(|error| format!("Cannot read AppImage permissions: {error}"))?
            .permissions();
        permissions.set_mode(0o755);
        fs::set_permissions(&installed_path, permissions)
            .map_err(|error| format!("Cannot mark AppImage executable: {error}"))?;
        return Ok(true);
    }

    let extracted = extract_archive(app, archive_path, install_dir)?;
    if extracted {
        normalize_extracted_runner(install_dir)?;
    }
    Ok(extracted)
}

fn normalize_extracted_runner(install_dir: &Path) -> Result<(), String> {
    if install_dir.join("bin/wine").exists()
        || install_dir.join("proton").exists()
        || install_dir.join("dosbox-x").exists()
        || install_dir.join("cncnet").exists()
    {
        return Ok(());
    }

    let entries = fs::read_dir(install_dir)
        .map_err(|error| format!("Cannot read extracted install directory: {error}"))?
        .filter_map(Result::ok)
        .collect::<Vec<_>>();

    let Some(nested_dir) = entries.iter().map(|entry| entry.path()).find(|path| {
        path.is_dir()
            && (path.join("bin/wine").exists()
                || path.join("proton").exists()
                || path.join("dosbox-x").exists()
                || path.join("cncnet").exists()
                || path.join("cncnet-client").exists())
    }) else {
        return Ok(());
    };

    for entry in fs::read_dir(&nested_dir)
        .map_err(|error| format!("Cannot read nested Wine directory: {error}"))?
    {
        let entry = entry.map_err(|error| format!("Cannot read nested Wine entry: {error}"))?;
        let target = install_dir.join(entry.file_name());
        if target.exists() {
            continue;
        }
        fs::rename(entry.path(), target)
            .map_err(|error| format!("Cannot normalize extracted Wine directory: {error}"))?;
    }

    let _ = fs::remove_dir(&nested_dir);
    Ok(())
}

fn emulator_status(paths: &RuntimePaths) -> EmulatorStatus {
    EmulatorStatus {
        emulators_dir: paths.emulators_dir.clone(),
        openra_dir: paths.openra_dir.clone(),
        dosbox_x_dir: paths.dosbox_x_dir.clone(),
        openra_red_alert_ready: Path::new(&paths.openra_dir)
            .join("OpenRA-Red-Alert-x86_64.AppImage")
            .exists(),
        openra_tiberian_dawn_ready: Path::new(&paths.openra_dir)
            .join("OpenRA-Tiberian-Dawn-x86_64.AppImage")
            .exists(),
        openra_dune_2000_ready: Path::new(&paths.openra_dir)
            .join("OpenRA-Dune-2000-x86_64.AppImage")
            .exists(),
        dosbox_x_ready: Path::new(&paths.dosbox_x_dir)
            .join("current/dosbox-x")
            .exists(),
        cncnet_dir: paths.cncnet_dir.clone(),
        cncnet_ready: Path::new(&paths.cncnet_dir).join("current/cncnet").exists(),
    }
}

fn openra_selection(paths: &RuntimePaths, mod_id: &str, target: String) -> RunnerSelection {
    let appimage = match mod_id {
        "red-alert" => "OpenRA-Red-Alert-x86_64.AppImage",
        "tiberian-dawn" => "OpenRA-Tiberian-Dawn-x86_64.AppImage",
        "dune-2000" => "OpenRA-Dune-2000-x86_64.AppImage",
        _ => "OpenRA-Red-Alert-x86_64.AppImage",
    };
    let mut arguments = Vec::new();
    if !target.is_empty() {
        arguments.push(target);
    }

    RunnerSelection {
        runner: "openra".into(),
        executable: Some(
            Path::new(&paths.openra_dir)
                .join(appimage)
                .to_string_lossy()
                .into_owned(),
        ),
        arguments,
        notes: format!("OpenRA {mod_id} games are routed to their portable AppImage."),
    }
}

fn write_openra_catalog(paths: &RuntimePaths) -> Result<(), String> {
    let catalog = serde_json::json!({
        "release": "release-20250330",
        "source": "https://www.openra.net/download/",
        "mods": [
            {
                "id": "red-alert",
                "name": "Red Alert",
                "fileName": "OpenRA-Red-Alert-x86_64.AppImage",
                "url": "https://github.com/OpenRA/OpenRA/releases/download/release-20250330/OpenRA-Red-Alert-x86_64.AppImage"
            },
            {
                "id": "tiberian-dawn",
                "name": "Tiberian Dawn",
                "fileName": "OpenRA-Tiberian-Dawn-x86_64.AppImage",
                "url": "https://github.com/OpenRA/OpenRA/releases/download/release-20250330/OpenRA-Tiberian-Dawn-x86_64.AppImage"
            },
            {
                "id": "dune-2000",
                "name": "Dune 2000",
                "fileName": "OpenRA-Dune-2000-x86_64.AppImage",
                "url": "https://github.com/OpenRA/OpenRA/releases/download/release-20250330/OpenRA-Dune-2000-x86_64.AppImage"
            }
        ]
    });
    let bytes = serde_json::to_vec_pretty(&catalog)
        .map_err(|error| format!("Cannot serialize OpenRA catalog: {error}"))?;
    fs::write(Path::new(&paths.openra_dir).join("catalog.json"), bytes)
        .map_err(|error| format!("Cannot write OpenRA catalog: {error}"))
}

fn write_dosbox_config(paths: &RuntimePaths) -> Result<(), String> {
    let config_path = Path::new(&paths.dosbox_x_dir).join("dosbox-x.conf");
    if config_path.exists() {
        return Ok(());
    }

    fs::write(
        config_path,
        "[sdl]\nfullscreen=false\n\n[render]\naspect=true\n\n[autoexec]\n",
    )
    .map_err(|error| format!("Cannot write DOSBox-X config: {error}"))
}

fn emit_log(app: &AppHandle, level: &str, message: &str) -> Result<(), String> {
    let log = BackendLog {
        level: level.into(),
        message: message.into(),
        timestamp: unix_timestamp(),
    };
    persist_log(&log)?;
    app.emit("backend-log", &log)
        .map_err(|error| format!("Cannot emit backend log: {error}"))
}

fn emit_download_progress(
    app: &AppHandle,
    kind: &RunnerKind,
    percent: u8,
    downloaded_bytes: u64,
    total_bytes: Option<u64>,
    status: &str,
) -> Result<(), String> {
    let progress = DownloadProgress {
        kind: kind.as_str().to_string(),
        percent: percent.clamp(1, 100),
        downloaded_bytes,
        total_bytes,
        status: status.to_string(),
    };
    app.emit("download-progress", &progress)
        .map_err(|error| format!("Cannot emit download progress: {error}"))
}

fn remote_content_length(url: &str) -> Option<u64> {
    let output = Command::new("curl")
        .args([
            "-L",
            "--silent",
            "--show-error",
            "--head",
            "--connect-timeout",
            "8",
            "--max-time",
            "12",
        ])
        .arg(url)
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }

    String::from_utf8_lossy(&output.stdout)
        .lines()
        .rev()
        .find_map(|line| {
            let (name, value) = line.split_once(':')?;
            if !name.eq_ignore_ascii_case("content-length") {
                return None;
            }
            value.trim().parse::<u64>().ok().filter(|size| *size > 0)
        })
}

fn download_percent(downloaded_bytes: u64, total_bytes: Option<u64>) -> u8 {
    let Some(total) = total_bytes.filter(|total| *total > 0) else {
        return 1;
    };
    let percent = downloaded_bytes.saturating_mul(100) / total;
    percent.clamp(1, 99) as u8
}

pub fn emit_backend_log(app: &AppHandle, level: &str, message: &str) -> Result<(), String> {
    emit_log(app, level, message)
}

fn persist_log(log: &BackendLog) -> Result<(), String> {
    let paths = paths()?;
    fs::create_dir_all(&paths.logs_dir)
        .map_err(|error| format!("Cannot create logs directory: {error}"))?;
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(Path::new(&paths.logs_dir).join("backend.log"))
        .map_err(|error| format!("Cannot open backend log file: {error}"))?;
    writeln!(file, "{} [{}] {}", log.timestamp, log.level, log.message)
        .map_err(|error| format!("Cannot write backend log: {error}"))
}

fn default_archive_name(kind: &RunnerKind, url: &str) -> String {
    url.rsplit('/')
        .next()
        .filter(|name| !name.trim().is_empty())
        .map(ToString::to_string)
        .unwrap_or_else(|| format!("{}.archive", kind.as_str()))
}

fn install_dir(paths: &RuntimePaths, kind: &RunnerKind) -> PathBuf {
    match kind {
        RunnerKind::Wine => Path::new(&paths.wine_dir).join("current"),
        RunnerKind::Proton => Path::new(&paths.proton_dir).join("current"),
        RunnerKind::Dxvk => Path::new(&paths.components_dir).join("dxvk/current"),
        RunnerKind::Vkd3d => Path::new(&paths.components_dir).join("vkd3d/current"),
        RunnerKind::Openra => Path::new(&paths.openra_dir).to_path_buf(),
        RunnerKind::DosboxX => Path::new(&paths.dosbox_x_dir).join("current"),
        RunnerKind::Cncnet => Path::new(&paths.cncnet_dir).join("current"),
    }
}

fn sanitize_id(value: &str) -> String {
    value
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() {
                character.to_ascii_lowercase()
            } else {
                '-'
            }
        })
        .collect::<String>()
        .split('-')
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>()
        .join("-")
}

fn unix_timestamp() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or_default()
}

fn path_string(path: PathBuf) -> String {
    path.to_string_lossy().into_owned()
}

pub fn runtime_paths() -> Result<RuntimePaths, String> {
    paths()
}

pub fn game_install_dir(game_id: &str) -> Result<PathBuf, String> {
    let paths = paths()?;
    let id = sanitize_id(game_id);
    if id.is_empty() {
        return Err("Game id must include at least one letter or number.".into());
    }

    Ok(Path::new(&paths.data_dir).join("games").join(id))
}

pub fn sanitize_game_id(game_id: &str) -> String {
    sanitize_id(game_id)
}
