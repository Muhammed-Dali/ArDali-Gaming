use crate::runtime::{self, RunnerKind, RuntimeEventSink};
use serde::{Deserialize, Serialize};
use std::{fs, path::Path, process::Command};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ComponentUpdate {
    pub component: String,
    pub current_version: Option<String>,
    pub available_version: Option<String>,
    pub url: Option<String>,
    pub installed: bool,
    pub update_available: bool,
    #[serde(default)]
    pub source: String,
    #[serde(default)]
    pub removable: bool,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ComponentUpdateRequest {
    pub component: String,
    pub version: Option<String>,
    pub url: String,
}

pub fn check_components() -> Result<Vec<ComponentUpdate>, String> {
    let paths = runtime::runtime_paths()?;
    let manifest = update_manifest_path(&paths.data_dir);
    let configured = read_manifest(&manifest)?;
    let components = [
        "wine", "proton", "dxvk", "vkd3d", "openra", "dosbox-x", "cncnet",
    ];

    Ok(components
        .into_iter()
        .map(|component| {
            let portable_installed = component_portable_installed(&paths, component);
            let system_version = (!portable_installed)
                .then(|| system_component_version(component))
                .flatten();
            let installed = portable_installed || system_version.is_some();
            let source = if portable_installed {
                "portable"
            } else if system_version.is_some() {
                "system"
            } else {
                "missing"
            };
            let configured_item = configured
                .iter()
                .find(|item| item.component == component)
                .cloned();
            let current_version = if portable_installed {
                read_version(&paths.data_dir, component).ok()
            } else {
                system_version
            };
            let available_version = configured_item
                .as_ref()
                .and_then(|item| item.available_version.clone());
            let update_available =
                installed && available_version.is_some() && available_version != current_version;
            ComponentUpdate {
                component: component.into(),
                current_version,
                available_version,
                url: configured_item.and_then(|item| item.url),
                installed,
                update_available,
                source: source.into(),
                removable: portable_installed,
            }
        })
        .collect())
}

pub fn update_component(
    app: &dyn RuntimeEventSink,
    request: ComponentUpdateRequest,
) -> Result<ComponentUpdate, String> {
    let kind = runner_kind(&request.component)?;
    let result = runtime::download_runner(app, kind, request.url.clone(), None)?;
    let paths = runtime::runtime_paths()?;
    let version = request
        .version
        .clone()
        .unwrap_or_else(|| "manual".to_string());
    write_version(&paths.data_dir, &request.component, &version)?;

    Ok(ComponentUpdate {
        component: request.component,
        current_version: Some(version.clone()),
        available_version: Some(version),
        url: Some(request.url),
        installed: result.extracted,
        update_available: false,
        source: "portable".into(),
        removable: true,
    })
}

pub fn remove_component(
    app: &dyn RuntimeEventSink,
    component: String,
) -> Result<ComponentUpdate, String> {
    let kind = runner_kind(&component)?;
    let paths = runtime::runtime_paths()?;
    let target = component_install_path(&paths, &kind);
    if target.exists() {
        remove_path(&target)?;
    }
    let version = version_path(&paths.data_dir, &component);
    if version.exists() {
        fs::remove_file(&version)
            .map_err(|error| format!("Cannot remove component version file: {error}"))?;
    }

    runtime::emit_backend_log(app, "info", &format!("{component} kaldırıldı"))?;
    let system_version = system_component_version(&component);
    Ok(ComponentUpdate {
        component,
        current_version: system_version.clone(),
        available_version: None,
        url: None,
        installed: system_version.is_some(),
        update_available: false,
        source: if system_version.is_some() {
            "system"
        } else {
            "missing"
        }
        .into(),
        removable: false,
    })
}

fn runner_kind(component: &str) -> Result<RunnerKind, String> {
    match component {
        "wine" => Ok(RunnerKind::Wine),
        "proton" => Ok(RunnerKind::Proton),
        "dxvk" => Ok(RunnerKind::Dxvk),
        "vkd3d" => Ok(RunnerKind::Vkd3d),
        "openra" => Ok(RunnerKind::Openra),
        "dosbox-x" => Ok(RunnerKind::DosboxX),
        "cncnet" => Ok(RunnerKind::Cncnet),
        _ => Err(format!("Unknown update component: {component}")),
    }
}

fn component_install_path(paths: &runtime::RuntimePaths, kind: &RunnerKind) -> std::path::PathBuf {
    match kind {
        RunnerKind::Wine => Path::new(&paths.wine_dir).join("current"),
        RunnerKind::Steam => Path::new(&paths.components_dir).join("steam-system"),
        RunnerKind::Proton => Path::new(&paths.proton_dir).join("current"),
        RunnerKind::Dxvk => Path::new(&paths.components_dir).join("dxvk/current"),
        RunnerKind::Vkd3d => Path::new(&paths.components_dir).join("vkd3d/current"),
        RunnerKind::Openra => Path::new(&paths.openra_dir).to_path_buf(),
        RunnerKind::DosboxX => Path::new(&paths.dosbox_x_dir).join("current"),
        RunnerKind::Cncnet => Path::new(&paths.cncnet_dir).join("current"),
    }
}

fn remove_path(path: &Path) -> Result<(), String> {
    let metadata =
        fs::symlink_metadata(path).map_err(|error| format!("Cannot inspect component: {error}"))?;
    if metadata.is_dir() {
        fs::remove_dir_all(path).map_err(|error| format!("Cannot remove component: {error}"))
    } else {
        fs::remove_file(path).map_err(|error| format!("Cannot remove component: {error}"))
    }
}

fn component_portable_installed(paths: &runtime::RuntimePaths, component: &str) -> bool {
    match component {
        "wine" => Path::new(&paths.wine_dir).join("current").exists(),
        "proton" => Path::new(&paths.proton_dir).join("current").exists(),
        "dxvk" => Path::new(&paths.components_dir)
            .join("dxvk/current")
            .exists(),
        "vkd3d" => Path::new(&paths.components_dir)
            .join("vkd3d/current")
            .exists(),
        "openra" => {
            Path::new(&paths.openra_dir)
                .join("OpenRA-Red-Alert-x86_64.AppImage")
                .exists()
                || Path::new(&paths.openra_dir)
                    .join("OpenRA-Tiberian-Dawn-x86_64.AppImage")
                    .exists()
                || Path::new(&paths.openra_dir)
                    .join("OpenRA-Dune-2000-x86_64.AppImage")
                    .exists()
        }
        "dosbox-x" => Path::new(&paths.dosbox_x_dir).join("current").exists(),
        "cncnet" => Path::new(&paths.cncnet_dir).join("current/cncnet").exists(),
        _ => false,
    }
}

fn system_component_version(component: &str) -> Option<String> {
    match component {
        "wine" => command_first_line("wine", &["--version"]),
        "proton" => command_first_line("proton", &["--version"]),
        "openra" => command_first_line("openra", &["--version"]),
        "dosbox-x" => command_first_line("dosbox-x", &["--version"]),
        "cncnet" => command_first_line("cncnet", &["--version"]),
        _ => None,
    }
}

fn command_first_line(command: &str, args: &[&str]) -> Option<String> {
    let output = Command::new(command).args(args).output().ok()?;
    if !output.status.success() {
        return None;
    }

    String::from_utf8_lossy(&output.stdout)
        .lines()
        .map(str::trim)
        .find(|line| !line.is_empty())
        .map(str::to_string)
}

fn read_manifest(path: &Path) -> Result<Vec<ComponentUpdate>, String> {
    if !path.exists() {
        return Ok(Vec::new());
    }

    let bytes = fs::read(path).map_err(|error| format!("Cannot read update manifest: {error}"))?;
    serde_json::from_slice(&bytes).map_err(|error| format!("Cannot parse update manifest: {error}"))
}

fn update_manifest_path(data_dir: &str) -> std::path::PathBuf {
    Path::new(data_dir).join("updates").join("components.json")
}

fn version_path(data_dir: &str, component: &str) -> std::path::PathBuf {
    Path::new(data_dir)
        .join("updates")
        .join(format!("{component}.version"))
}

fn read_version(data_dir: &str, component: &str) -> Result<String, String> {
    fs::read_to_string(version_path(data_dir, component))
        .map(|value| value.trim().to_string())
        .map_err(|error| format!("Cannot read component version: {error}"))
}

fn write_version(data_dir: &str, component: &str, version: &str) -> Result<(), String> {
    let path = version_path(data_dir, component);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|error| format!("Cannot create update directory: {error}"))?;
    }
    fs::write(path, format!("{version}\n"))
        .map_err(|error| format!("Cannot write component version: {error}"))
}
