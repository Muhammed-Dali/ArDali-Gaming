use crate::runtime;
use serde::Serialize;
use std::{
    collections::BTreeSet,
    env, fs,
    path::{Path, PathBuf},
};

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SteamGame {
    pub app_id: String,
    pub name: String,
    pub install_dir: String,
    pub library_dir: String,
    pub manifest_path: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SteamScan {
    pub steam_roots: Vec<String>,
    pub library_dirs: Vec<String>,
    pub proton_versions: Vec<String>,
    pub games: Vec<SteamGame>,
}

pub fn scan() -> Result<SteamScan, String> {
    let steam_roots = steam_roots()?;
    let library_dirs = library_dirs(&steam_roots);
    let proton_versions = proton_versions(&steam_roots, &library_dirs);
    let games = steam_games(&library_dirs);

    Ok(SteamScan {
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
    })
}

pub fn preferred_proton(scan: &SteamScan) -> Option<String> {
    scan.proton_versions
        .iter()
        .find(|path| path.contains("Proton"))
        .cloned()
        .or_else(|| scan.proton_versions.first().cloned())
}

fn steam_roots() -> Result<Vec<PathBuf>, String> {
    let home = env::var("HOME").map_err(|_| "HOME environment variable is not set.".to_string())?;
    let candidates = [
        PathBuf::from(&home).join(".local/share/Steam"),
        PathBuf::from(&home).join(".steam/steam"),
        PathBuf::from(&home).join(".var/app/com.valvesoftware.Steam/.local/share/Steam"),
    ];

    Ok(candidates
        .into_iter()
        .filter(|path| path.exists())
        .collect())
}

fn library_dirs(steam_roots: &[PathBuf]) -> Vec<PathBuf> {
    let mut dirs = BTreeSet::new();
    for root in steam_roots {
        dirs.insert(root.clone());
        let library_file = root.join("steamapps/libraryfolders.vdf");
        if let Ok(contents) = fs::read_to_string(library_file) {
            for path in parse_vdf_values(&contents, "path") {
                let candidate = PathBuf::from(path);
                if candidate.join("steamapps").exists() {
                    dirs.insert(candidate);
                }
            }
        }
    }
    dirs.into_iter().collect()
}

fn steam_games(library_dirs: &[PathBuf]) -> Vec<SteamGame> {
    let mut games = Vec::new();
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
                games.push(game);
            }
        }
    }
    games.sort_by(|left, right| left.name.to_lowercase().cmp(&right.name.to_lowercase()));
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
        if path.is_dir() && name.to_lowercase().contains("proton") {
            versions.insert(path);
        }
    }
}

fn parse_manifest(library: &Path, manifest_path: &Path) -> Option<SteamGame> {
    let contents = fs::read_to_string(manifest_path).ok()?;
    let app_id = parse_vdf_values(&contents, "appid").into_iter().next()?;
    let name = parse_vdf_values(&contents, "name").into_iter().next()?;
    let install_folder = parse_vdf_values(&contents, "installdir")
        .into_iter()
        .next()
        .unwrap_or_else(|| name.clone());
    let install_dir = library.join("steamapps/common").join(install_folder);

    Some(SteamGame {
        app_id,
        name,
        install_dir: install_dir.to_string_lossy().into_owned(),
        library_dir: library.to_string_lossy().into_owned(),
        manifest_path: manifest_path.to_string_lossy().into_owned(),
    })
}

fn parse_vdf_values(contents: &str, key: &str) -> Vec<String> {
    let mut values = Vec::new();
    for line in contents.lines() {
        let mut parts = line.split('"').filter(|part| !part.trim().is_empty());
        let Some(found_key) = parts.next() else {
            continue;
        };
        let Some(value) = parts.next() else {
            continue;
        };
        if found_key == key {
            values.push(value.replace("\\\\", "\\"));
        }
    }
    values
}

pub fn steam_game_id(app_id: &str) -> String {
    runtime::sanitize_game_id(&format!("steam-{app_id}"))
}
