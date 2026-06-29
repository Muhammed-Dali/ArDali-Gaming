use crate::{database, runtime};
use serde::Serialize;
use serde_json::Value;
use std::{fs, path::Path, process::Command};

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MetadataResult {
    pub game_id: i64,
    pub name: String,
    pub cover_path: Option<String>,
    pub genre: Option<String>,
    pub release_year: Option<i64>,
    pub description: Option<String>,
    pub source: String,
}

pub fn fetch_from_steamgriddb(id: i64) -> Result<MetadataResult, String> {
    let game = database::get_game_by_id(id)?;
    let api_key = database::get_setting_value("steamgriddb_api_key")?
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| "SteamGridDB API key is not configured.".to_string())?;

    let search_url = format!(
        "https://www.steamgriddb.com/api/v2/search/autocomplete/{}",
        url_segment(&game.name)
    );
    let search = request_json(&search_url, &api_key)?;
    let Some(remote_id) = search
        .get("data")
        .and_then(Value::as_array)
        .and_then(|items| items.first())
        .and_then(|item| item.get("id"))
        .and_then(Value::as_i64)
    else {
        return Err("SteamGridDB game match was not found.".into());
    };

    let grid_url = format!(
        "https://www.steamgriddb.com/api/v2/grids/game/{remote_id}?dimensions=600x900&types=static"
    );
    let grids = request_json(&grid_url, &api_key)?;
    let cover_url = grids
        .get("data")
        .and_then(Value::as_array)
        .and_then(|items| items.first())
        .and_then(|item| item.get("url"))
        .and_then(Value::as_str)
        .map(ToString::to_string);

    let cover_path = match cover_url {
        Some(url) => Some(download_cover(id, &url)?),
        None => None,
    };
    let release_year = search
        .get("data")
        .and_then(Value::as_array)
        .and_then(|items| items.first())
        .and_then(|item| item.get("release_date"))
        .and_then(Value::as_i64)
        .map(|timestamp| 1970 + timestamp / 31_536_000);

    let description = Some(format!("SteamGridDB match id: {remote_id}"));
    database::save_metadata(
        id,
        cover_path.clone(),
        Some(game.name.clone()),
        None,
        release_year,
        description.clone(),
        "steamgriddb".into(),
    )?;

    Ok(MetadataResult {
        game_id: id,
        name: game.name,
        cover_path,
        genre: None,
        release_year,
        description,
        source: "steamgriddb".into(),
    })
}

pub fn set_manual_cover(id: i64, cover_path: String) -> Result<MetadataResult, String> {
    let game = database::save_metadata(
        id,
        Some(cover_path.clone()),
        None,
        None,
        None,
        None,
        "manual".into(),
    )?;

    Ok(MetadataResult {
        game_id: id,
        name: game.name,
        cover_path: Some(cover_path),
        genre: game.genre,
        release_year: game.release_year,
        description: game.description,
        source: "manual".into(),
    })
}

fn request_json(url: &str, api_key: &str) -> Result<Value, String> {
    let output = Command::new("curl")
        .args(["-L", "--fail", "--silent", "--show-error"])
        .arg("-H")
        .arg(format!("Authorization: Bearer {api_key}"))
        .arg(url)
        .output()
        .map_err(|error| format!("Failed to start curl: {error}"))?;

    if !output.status.success() {
        return Err(format!("SteamGridDB request failed: {}", output.status));
    }

    serde_json::from_slice(&output.stdout)
        .map_err(|error| format!("Cannot parse SteamGridDB response: {error}"))
}

fn download_cover(id: i64, url: &str) -> Result<String, String> {
    let paths = runtime::runtime_paths()?;
    let cover_dir = Path::new(&paths.data_dir).join("covers");
    fs::create_dir_all(&cover_dir).map_err(|error| format!("Cannot create cover dir: {error}"))?;
    let extension = url
        .rsplit('.')
        .next()
        .filter(|value| value.len() <= 5)
        .unwrap_or("jpg");
    let path = cover_dir.join(format!("{id}.{extension}"));
    let status = Command::new("curl")
        .args(["-L", "--fail", "--show-error", "-o"])
        .arg(&path)
        .arg(url)
        .status()
        .map_err(|error| format!("Failed to download cover: {error}"))?;
    if !status.success() {
        return Err(format!("Cover download failed: {status}"));
    }
    Ok(path.to_string_lossy().into_owned())
}

fn url_segment(value: &str) -> String {
    value
        .bytes()
        .flat_map(|byte| match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' => {
                vec![byte as char]
            }
            b' ' => vec!['%', '2', '0'],
            _ => {
                let encoded = format!("%{byte:02X}");
                encoded.chars().collect()
            }
        })
        .collect()
}
