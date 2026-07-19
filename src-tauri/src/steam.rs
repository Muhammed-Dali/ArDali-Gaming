use crate::runtime;
use serde::Serialize;
use std::{
    collections::{BTreeMap, BTreeSet},
    env, fs,
    path::{Component, Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SteamGame {
    pub app_id: String,
    pub name: String,
    pub state_flags: Option<u64>,
    pub installed: bool,
    pub install_dir: String,
    pub library_dir: String,
    pub manifest_path: String,
    pub cover_path: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SteamScan {
    pub steam_roots: Vec<String>,
    pub library_dirs: Vec<String>,
    pub proton_versions: Vec<String>,
    pub games: Vec<SteamGame>,
    pub launcher_command: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SteamLauncher {
    pub executable: String,
    pub arguments: Vec<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ProtonTool {
    pub name: String,
    pub display_name: String,
    pub path: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SteamCompatibilityStatus {
    pub app_id: String,
    pub selected_tool: Option<String>,
    pub tools: Vec<ProtonTool>,
    pub config_path: Option<String>,
    pub steam_running: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SteamCompatibilityUpdate {
    pub app_id: String,
    pub selected_tool: Option<String>,
    pub config_path: String,
    pub backup_path: String,
}

pub fn scan() -> Result<SteamScan, String> {
    let steam_roots = steam_roots()?;
    let library_dirs = library_dirs(&steam_roots);
    let proton_versions = proton_versions(&steam_roots, &library_dirs);
    let mut games = steam_games(&library_dirs);
    for game in &mut games {
        game.cover_path = local_steam_cover(&steam_roots, &game.app_id)
            .map(|path| path.to_string_lossy().into_owned());
    }

    let mut scan = SteamScan {
        steam_roots: steam_roots
            .into_iter()
            .map(|path| path.to_string_lossy().into_owned())
            .collect(),
        library_dirs: library_dirs
            .into_iter()
            .map(|path| path.to_string_lossy().into_owned())
            .collect(),
        proton_versions: proton_versions
            .into_iter()
            .map(|path| path.to_string_lossy().into_owned())
            .collect(),
        games,
        launcher_command: None,
    };
    scan.launcher_command = launcher(&scan).map(|launcher| {
        std::iter::once(launcher.executable)
            .chain(launcher.arguments)
            .collect::<Vec<_>>()
            .join(" ")
    });
    Ok(scan)
}

pub fn launcher(scan: &SteamScan) -> Option<SteamLauncher> {
    if let Some(executable) = command_in_path("steam") {
        return Some(SteamLauncher {
            executable: executable.to_string_lossy().into_owned(),
            arguments: Vec::new(),
        });
    }

    for root in &scan.steam_roots {
        let script = Path::new(root).join("steam.sh");
        if script.is_file() {
            return Some(SteamLauncher {
                executable: script.to_string_lossy().into_owned(),
                arguments: Vec::new(),
            });
        }
    }

    if scan
        .steam_roots
        .iter()
        .any(|root| root.contains("/.var/app/com.valvesoftware.Steam/"))
    {
        if let Some(executable) = command_in_path("flatpak") {
            return Some(SteamLauncher {
                executable: executable.to_string_lossy().into_owned(),
                arguments: vec!["run".into(), "com.valvesoftware.Steam".into()],
            });
        }
    }

    if scan
        .steam_roots
        .iter()
        .any(|root| root.contains("/snap/steam/"))
    {
        if let Some(executable) = command_in_path("snap") {
            return Some(SteamLauncher {
                executable: executable.to_string_lossy().into_owned(),
                arguments: vec!["run".into(), "steam".into()],
            });
        }
    }

    None
}

pub fn launch_arguments(launcher: &SteamLauncher, app_id: &str) -> Vec<String> {
    let mut arguments = launcher.arguments.clone();
    arguments.extend(["-applaunch".into(), app_id.into()]);
    arguments
}

pub fn compatibility_status(app_id: &str) -> Result<SteamCompatibilityStatus, String> {
    validate_app_id(app_id)?;
    let scan = scan()?;
    let tools = proton_tools(&scan.proton_versions);
    let config = steam_config_path(&scan.steam_roots);
    let selected_tool = config
        .as_deref()
        .and_then(|path| fs::read_to_string(path).ok())
        .and_then(|contents| compat_tool_mapping(&contents, app_id));
    Ok(SteamCompatibilityStatus {
        app_id: app_id.into(),
        selected_tool,
        tools,
        config_path: config.map(|path| path.to_string_lossy().into_owned()),
        steam_running: steam_client_running(),
    })
}

pub fn set_compatibility_tool(
    app_id: &str,
    tool_name: Option<&str>,
) -> Result<SteamCompatibilityUpdate, String> {
    validate_app_id(app_id)?;
    if steam_client_running() {
        return Err("Proton tercihini değiştirmeden önce Steam istemcisini tamamen kapat".into());
    }
    let scan = scan()?;
    let tools = proton_tools(&scan.proton_versions);
    let selected = tool_name.map(str::trim).filter(|value| !value.is_empty());
    if let Some(name) = selected {
        if !tools.iter().any(|tool| tool.name == name) {
            return Err(format!("Seçilen Proton aracı kurulu değil: {name}"));
        }
    }
    let config_path = steam_config_path(&scan.steam_roots)
        .ok_or_else(|| "Steam config.vdf bulunamadı".to_string())?;
    let contents = fs::read_to_string(&config_path)
        .map_err(|error| format!("Steam yapılandırması okunamadı: {error}"))?;
    let updated = update_compat_tool_mapping(&contents, app_id, selected)?;
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|error| format!("Sistem saati okunamadı: {error}"))?
        .as_secs();
    let backup_path = config_path.with_extension(format!("vdf.ardali-backup-{timestamp}"));
    fs::copy(&config_path, &backup_path)
        .map_err(|error| format!("Steam yapılandırması yedeklenemedi: {error}"))?;
    let temporary_path = config_path.with_extension(format!("vdf.ardali-tmp-{timestamp}"));
    fs::write(&temporary_path, updated)
        .map_err(|error| format!("Steam Proton tercihi yazılamadı: {error}"))?;
    if let Ok(metadata) = fs::metadata(&config_path) {
        fs::set_permissions(&temporary_path, metadata.permissions())
            .map_err(|error| format!("Steam yapılandırma izinleri korunamadı: {error}"))?;
    }
    fs::rename(&temporary_path, &config_path)
        .map_err(|error| format!("Steam Proton tercihi atomik olarak uygulanamadı: {error}"))?;
    Ok(SteamCompatibilityUpdate {
        app_id: app_id.into(),
        selected_tool: selected.map(ToString::to_string),
        config_path: config_path.to_string_lossy().into_owned(),
        backup_path: backup_path.to_string_lossy().into_owned(),
    })
}

fn validate_app_id(app_id: &str) -> Result<(), String> {
    if app_id.is_empty() || !app_id.chars().all(|character| character.is_ascii_digit()) {
        Err("Steam AppID yalnızca rakamlardan oluşmalı".into())
    } else {
        Ok(())
    }
}

fn proton_tools(paths: &[String]) -> Vec<ProtonTool> {
    let mut tools = paths
        .iter()
        .map(|path| {
            let directory = Path::new(path);
            let fallback = directory
                .file_name()
                .and_then(|value| value.to_str())
                .unwrap_or("Proton");
            let vdf = fs::read_to_string(directory.join("compatibilitytool.vdf")).ok();
            let name = vdf
                .as_deref()
                .and_then(compat_tool_name)
                .unwrap_or(fallback);
            let display_name = vdf
                .as_deref()
                .and_then(|contents| first_vdf_value(contents, "display_name"))
                .unwrap_or_else(|| fallback.to_string());
            ProtonTool {
                name: name.into(),
                display_name,
                path: path.clone(),
            }
        })
        .collect::<Vec<_>>();
    tools.sort_by(|left, right| {
        left.display_name
            .to_lowercase()
            .cmp(&right.display_name.to_lowercase())
    });
    tools.dedup_by(|left, right| left.name == right.name);
    tools
}

fn compat_tool_name(contents: &str) -> Option<&str> {
    let fields = contents.split('"').skip(1).step_by(2).collect::<Vec<_>>();
    fields
        .windows(2)
        .find(|pair| pair[0] == "compat_tools")
        .map(|pair| pair[1])
}

fn steam_config_path(roots: &[String]) -> Option<PathBuf> {
    roots
        .iter()
        .map(|root| Path::new(root).join("config/config.vdf"))
        .find(|path| path.is_file())
}

fn steam_client_running() -> bool {
    fs::read_dir("/proc")
        .ok()
        .into_iter()
        .flatten()
        .flatten()
        .any(|entry| {
            let name = entry.file_name();
            if !name
                .to_string_lossy()
                .chars()
                .all(|character| character.is_ascii_digit())
            {
                return false;
            }
            fs::read_to_string(entry.path().join("comm"))
                .map(|comm| matches!(comm.trim(), "steam" | "steamwebhelper"))
                .unwrap_or(false)
        })
}

fn command_in_path(name: &str) -> Option<PathBuf> {
    env::var_os("PATH")?
        .to_string_lossy()
        .split(':')
        .map(|directory| Path::new(directory).join(name))
        .find(|candidate| candidate.is_file())
}

fn steam_roots() -> Result<Vec<PathBuf>, String> {
    let home = env::var_os("HOME")
        .map(PathBuf::from)
        .ok_or_else(|| "HOME environment variable is not set.".to_string())?;
    Ok(steam_roots_from_home(&home))
}

fn steam_roots_from_home(home: &Path) -> Vec<PathBuf> {
    let candidates = [
        home.join(".local/share/Steam"),
        home.join(".steam/steam"),
        home.join(".steam/root"),
        home.join(".steam/debian-installation"),
        home.join(".var/app/com.valvesoftware.Steam/.local/share/Steam"),
        home.join("snap/steam/common/.local/share/Steam"),
    ];

    unique_existing_dirs(candidates)
}

fn library_dirs(steam_roots: &[PathBuf]) -> Vec<PathBuf> {
    let mut dirs = BTreeSet::new();
    for root in steam_roots {
        if root.join("steamapps").is_dir() {
            dirs.insert(normalized_path(root));
        }
        let library_file = root.join("steamapps/libraryfolders.vdf");
        if let Ok(contents) = fs::read_to_string(library_file) {
            for path in parse_library_paths(&contents) {
                let candidate = PathBuf::from(path);
                if candidate.join("steamapps").is_dir() {
                    dirs.insert(normalized_path(&candidate));
                }
            }
        }
    }
    dirs.into_iter().collect()
}

fn steam_games(library_dirs: &[PathBuf]) -> Vec<SteamGame> {
    let mut games_by_app_id = BTreeMap::<String, SteamGame>::new();
    for library in library_dirs {
        let steamapps = library.join("steamapps");
        let Ok(entries) = fs::read_dir(&steamapps) else {
            continue;
        };

        for entry in entries.flatten() {
            let path = entry.path();
            let Some(file_name) = path.file_name().and_then(|name| name.to_str()) else {
                continue;
            };
            if !file_name.starts_with("appmanifest_") || !file_name.ends_with(".acf") {
                continue;
            }

            if let Some(game) = parse_manifest(library, &path) {
                match games_by_app_id.entry(game.app_id.clone()) {
                    std::collections::btree_map::Entry::Vacant(entry) => {
                        entry.insert(game);
                    }
                    std::collections::btree_map::Entry::Occupied(mut entry)
                        if !entry.get().installed && game.installed =>
                    {
                        entry.insert(game);
                    }
                    _ => {}
                }
            }
        }
    }
    let mut games = games_by_app_id.into_values().collect::<Vec<_>>();
    games.sort_by_key(|game| game.name.to_lowercase());
    games
}

fn proton_versions(steam_roots: &[PathBuf], library_dirs: &[PathBuf]) -> Vec<PathBuf> {
    let mut versions = BTreeSet::new();
    for root in steam_roots {
        collect_proton_dirs(&root.join("compatibilitytools.d"), &mut versions);
    }
    for library in library_dirs {
        collect_proton_dirs(&library.join("steamapps/common"), &mut versions);
    }
    versions.into_iter().collect()
}

fn collect_proton_dirs(parent: &Path, versions: &mut BTreeSet<PathBuf>) {
    let Ok(entries) = fs::read_dir(parent) else {
        return;
    };

    for entry in entries.flatten() {
        let path = entry.path();
        let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
            continue;
        };
        if path.is_dir() && name.to_lowercase().contains("proton") && path.join("proton").is_file()
        {
            versions.insert(path);
        }
    }
}

fn parse_manifest(library: &Path, manifest_path: &Path) -> Option<SteamGame> {
    let contents = fs::read_to_string(manifest_path).ok()?;
    let app_id = first_vdf_value(&contents, "appid")?;
    if app_id.is_empty() || !app_id.chars().all(|character| character.is_ascii_digit()) {
        return None;
    }
    let name = first_vdf_value(&contents, "name")?;
    if name.trim().is_empty() {
        return None;
    }
    let install_folder = PathBuf::from(first_vdf_value(&contents, "installdir")?);
    if install_folder.as_os_str().is_empty()
        || install_folder
            .components()
            .any(|component| !matches!(component, Component::Normal(_) | Component::CurDir))
    {
        return None;
    }
    let install_dir = library.join("steamapps/common").join(install_folder);
    let state_flags =
        first_vdf_value(&contents, "StateFlags").and_then(|value| value.parse::<u64>().ok());
    let installed = state_flags
        .map(|flags| flags & 4 == 4)
        .unwrap_or_else(|| install_dir.is_dir());

    Some(SteamGame {
        app_id,
        name,
        state_flags,
        installed,
        install_dir: install_dir.to_string_lossy().into_owned(),
        library_dir: library.to_string_lossy().into_owned(),
        manifest_path: manifest_path.to_string_lossy().into_owned(),
        cover_path: None,
    })
}

fn local_steam_cover(steam_roots: &[PathBuf], app_id: &str) -> Option<PathBuf> {
    const SUFFIXES: &[&str] = &[
        "library_600x900.jpg",
        "library_600x900.png",
        "header.jpg",
        "header.png",
    ];
    for root in steam_roots {
        let cache = root.join("appcache/librarycache");
        for suffix in SUFFIXES {
            for candidate in [
                cache.join(app_id).join(suffix),
                cache.join(format!("{app_id}_{suffix}")),
            ] {
                if candidate.is_file() {
                    return Some(candidate);
                }
            }
        }
    }
    None
}

fn parse_vdf_values(contents: &str, key: &str) -> Vec<String> {
    let mut values = Vec::new();
    for line in contents.lines() {
        let fields = line.split('"').skip(1).step_by(2).collect::<Vec<_>>();
        for pair in fields.windows(2) {
            if pair[0].eq_ignore_ascii_case(key) {
                values.push(pair[1].replace("\\\\", "\\"));
            }
        }
    }
    values
}

fn first_vdf_value(contents: &str, key: &str) -> Option<String> {
    parse_vdf_values(contents, key).into_iter().next()
}

fn parse_library_paths(contents: &str) -> Vec<String> {
    let mut paths = parse_vdf_values(contents, "path");

    // Steam'in eski libraryfolders.vdf biçimi kütüphaneleri
    // `"1" "/mnt/games/SteamLibrary"` şeklinde saklar.
    for line in contents.lines() {
        let parts = line
            .split('"')
            .filter(|part| !part.trim().is_empty())
            .collect::<Vec<_>>();
        if parts.len() >= 2 && parts[0].chars().all(|character| character.is_ascii_digit()) {
            paths.push(parts[1].replace("\\\\", "\\"));
        }
    }

    paths.sort();
    paths.dedup();
    paths
}

fn compat_tool_mapping(contents: &str, app_id: &str) -> Option<String> {
    let (open, close) = vdf_block(contents, "CompatToolMapping")?;
    let mapping = &contents[open + 1..close];
    let (app_open, app_close) = vdf_block(mapping, app_id)?;
    first_vdf_value(&mapping[app_open + 1..app_close], "name").filter(|value| !value.is_empty())
}

fn update_compat_tool_mapping(
    contents: &str,
    app_id: &str,
    tool_name: Option<&str>,
) -> Result<String, String> {
    let entry = tool_name.map(|name| {
        format!(
            "\n\t\t\t\t\"{app_id}\"\n\t\t\t\t{{\n\t\t\t\t\t\"name\"\t\t\"{name}\"\n\t\t\t\t\t\"config\"\t\t\"\"\n\t\t\t\t\t\"priority\"\t\t\"250\"\n\t\t\t\t}}"
        )
    });

    if let Some((mapping_open, mapping_close)) = vdf_block(contents, "CompatToolMapping") {
        let mapping = &contents[mapping_open + 1..mapping_close];
        if let Some((app_open, app_close)) = vdf_block(mapping, app_id) {
            let key_start = mapping[..app_open]
                .rfind(&format!("\"{app_id}\""))
                .ok_or_else(|| "Steam AppID eşlemesi ayrıştırılamadı".to_string())?;
            let absolute_start = mapping_open + 1 + key_start;
            let absolute_end = mapping_open + 1 + app_close + 1;
            let mut updated = contents.to_string();
            updated.replace_range(absolute_start..absolute_end, entry.as_deref().unwrap_or(""));
            return Ok(updated);
        }
        if let Some(entry) = entry {
            let mut updated = contents.to_string();
            updated.insert_str(mapping_close, &entry);
            return Ok(updated);
        }
        return Ok(contents.to_string());
    }

    let (_, steam_close) = vdf_block(contents, "Steam")
        .ok_or_else(|| "Steam config.vdf içinde Steam bölümü bulunamadı".to_string())?;
    let Some(entry) = entry else {
        return Ok(contents.to_string());
    };
    let block = format!("\n\t\t\t\"CompatToolMapping\"\n\t\t\t{{{entry}\n\t\t\t}}\n");
    let mut updated = contents.to_string();
    updated.insert_str(steam_close, &block);
    Ok(updated)
}

fn vdf_block(contents: &str, key: &str) -> Option<(usize, usize)> {
    let key_position = contents.find(&format!("\"{key}\""))?;
    let open = contents[key_position..].find('{')? + key_position;
    let mut depth = 0_u32;
    let mut quoted = false;
    let mut escaped = false;
    for (offset, character) in contents[open..].char_indices() {
        if character == '\\' && quoted && !escaped {
            escaped = true;
            continue;
        }
        if character == '"' && !escaped {
            quoted = !quoted;
        }
        if !quoted {
            if character == '{' {
                depth += 1;
            }
            if character == '}' {
                depth -= 1;
                if depth == 0 {
                    return Some((open, open + offset));
                }
            }
        }
        escaped = false;
    }
    None
}

fn unique_existing_dirs<const N: usize>(candidates: [PathBuf; N]) -> Vec<PathBuf> {
    candidates
        .into_iter()
        .filter(|path| path.is_dir())
        .map(|path| normalized_path(&path))
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}

fn normalized_path(path: &Path) -> PathBuf {
    path.canonicalize().unwrap_or_else(|_| path.to_path_buf())
}

pub fn steam_game_id(app_id: &str) -> String {
    runtime::sanitize_game_id(&format!("steam-{app_id}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn test_dir(name: &str) -> PathBuf {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        env::temp_dir().join(format!(
            "ardali-steam-{name}-{}-{nonce}",
            std::process::id()
        ))
    }

    fn write_manifest(library: &Path, app_id: &str, body: &str) -> PathBuf {
        let steamapps = library.join("steamapps");
        fs::create_dir_all(&steamapps).unwrap();
        let path = steamapps.join(format!("appmanifest_{app_id}.acf"));
        fs::write(&path, body).unwrap();
        path
    }

    #[test]
    fn finds_native_flatpak_and_snap_steam_roots() {
        let home = test_dir("roots");
        let native = home.join(".local/share/Steam");
        let flatpak = home.join(".var/app/com.valvesoftware.Steam/.local/share/Steam");
        let snap = home.join("snap/steam/common/.local/share/Steam");
        fs::create_dir_all(&native).unwrap();
        fs::create_dir_all(&flatpak).unwrap();
        fs::create_dir_all(&snap).unwrap();

        let roots = steam_roots_from_home(&home);

        assert_eq!(roots.len(), 3);
        assert!(roots.contains(&native));
        assert!(roots.contains(&flatpak));
        assert!(roots.contains(&snap));
        fs::remove_dir_all(home).unwrap();
    }

    #[test]
    fn reads_modern_and_legacy_library_folder_entries() {
        let home = test_dir("libraries");
        let root = home.join(".local/share/Steam");
        let modern = home.join("modern-library");
        let legacy = home.join("legacy-library");
        fs::create_dir_all(root.join("steamapps")).unwrap();
        fs::create_dir_all(modern.join("steamapps")).unwrap();
        fs::create_dir_all(legacy.join("steamapps")).unwrap();
        fs::write(
            root.join("steamapps/libraryfolders.vdf"),
            format!(
                "\"libraryfolders\"\n{{\n  \"1\" {{ \"path\" \"{}\" }}\n  \"2\" \"{}\"\n}}",
                modern.display(),
                legacy.display()
            ),
        )
        .unwrap();

        let libraries = library_dirs(&[root.clone()]);

        assert_eq!(libraries.len(), 3);
        assert!(libraries.contains(&root));
        assert!(libraries.contains(&modern));
        assert!(libraries.contains(&legacy));
        fs::remove_dir_all(home).unwrap();
    }

    #[cfg(unix)]
    #[test]
    fn deduplicates_symlinked_steam_roots() {
        use std::os::unix::fs::symlink;

        let home = test_dir("symlinks");
        let native = home.join(".local/share/Steam");
        fs::create_dir_all(&native).unwrap();
        fs::create_dir_all(home.join(".steam")).unwrap();
        symlink(&native, home.join(".steam/steam")).unwrap();

        assert_eq!(steam_roots_from_home(&home), vec![native]);
        fs::remove_dir_all(home).unwrap();
    }

    #[test]
    fn parses_installed_game_manifest() {
        let library = test_dir("installed-manifest");
        let install_dir = library.join("steamapps/common/Half-Life");
        fs::create_dir_all(&install_dir).unwrap();
        let manifest = write_manifest(
            &library,
            "70",
            "\"AppState\"\n{\n\"appid\" \"70\"\n\"name\" \"Half-Life\"\n\"StateFlags\" \"4\"\n\"installdir\" \"Half-Life\"\n}",
        );

        let game = parse_manifest(&library, &manifest).unwrap();

        assert_eq!(game.app_id, "70");
        assert_eq!(game.name, "Half-Life");
        assert_eq!(game.state_flags, Some(4));
        assert!(game.installed);
        assert_eq!(game.install_dir, install_dir.to_string_lossy());
        fs::remove_dir_all(library).unwrap();
    }

    #[test]
    fn rejects_manifest_install_directory_traversal() {
        let library = test_dir("unsafe-manifest");
        let manifest = write_manifest(
            &library,
            "10",
            "\"AppState\" { \"appid\" \"10\" \"name\" \"Unsafe\" \"installdir\" \"../../outside\" }",
        );

        assert!(parse_manifest(&library, &manifest).is_none());
        fs::remove_dir_all(library).unwrap();
    }

    #[test]
    fn deduplicates_app_ids_and_prefers_installed_manifest() {
        let first_library = test_dir("duplicate-first");
        let installed_library = test_dir("duplicate-installed");
        write_manifest(
            &first_library,
            "220",
            "\"AppState\" { \"appid\" \"220\" \"name\" \"Half-Life 2\" \"StateFlags\" \"2\" \"installdir\" \"Half-Life 2\" }",
        );
        let installed_manifest = write_manifest(
            &installed_library,
            "220",
            "\"AppState\" { \"appid\" \"220\" \"name\" \"Half-Life 2\" \"StateFlags\" \"4\" \"installdir\" \"Half-Life 2\" }",
        );

        let games = steam_games(&[first_library.clone(), installed_library.clone()]);

        assert_eq!(games.len(), 1);
        assert!(games[0].installed);
        assert_eq!(games[0].manifest_path, installed_manifest.to_string_lossy());
        fs::remove_dir_all(first_library).unwrap();
        fs::remove_dir_all(installed_library).unwrap();
    }

    #[test]
    fn builds_native_and_flatpak_applaunch_arguments() {
        let native = SteamLauncher {
            executable: "/usr/bin/steam".into(),
            arguments: Vec::new(),
        };
        let flatpak = SteamLauncher {
            executable: "/usr/bin/flatpak".into(),
            arguments: vec!["run".into(), "com.valvesoftware.Steam".into()],
        };

        assert_eq!(launch_arguments(&native, "220"), ["-applaunch", "220"]);
        assert_eq!(
            launch_arguments(&flatpak, "220"),
            ["run", "com.valvesoftware.Steam", "-applaunch", "220"]
        );
    }

    #[test]
    fn finds_local_steam_library_cover() {
        let root = test_dir("cover");
        let cover = root
            .join("appcache/librarycache/220")
            .join("library_600x900.jpg");
        fs::create_dir_all(cover.parent().unwrap()).unwrap();
        fs::write(&cover, b"cover").unwrap();

        assert_eq!(local_steam_cover(&[root.clone()], "220"), Some(cover));
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn updates_and_removes_steam_compat_tool_mapping() {
        let config = "\"InstallConfigStore\" { \"Software\" { \"Valve\" { \"Steam\" { \"CompatToolMapping\" { \"220\" { \"name\" \"proton_old\" \"config\" \"\" \"priority\" \"250\" } } } } } }";
        let updated = update_compat_tool_mapping(config, "220", Some("proton_new")).unwrap();
        assert_eq!(
            compat_tool_mapping(&updated, "220").as_deref(),
            Some("proton_new")
        );
        assert!(updated.contains("proton_new"));
        assert!(!updated.contains("proton_old"));

        let removed = update_compat_tool_mapping(&updated, "220", None).unwrap();
        assert_eq!(compat_tool_mapping(&removed, "220"), None);
    }

    #[test]
    fn inserts_missing_compat_tool_mapping_block() {
        let config = "\"InstallConfigStore\" { \"Software\" { \"Valve\" { \"Steam\" { \"Language\" \"english\" } } } }";
        let updated =
            update_compat_tool_mapping(config, "70", Some("proton_experimental")).unwrap();
        assert_eq!(
            compat_tool_mapping(&updated, "70").as_deref(),
            Some("proton_experimental")
        );
    }
}
