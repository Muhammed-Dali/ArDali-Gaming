use crate::{database::GameRecord, runtime};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{
    fs::{self, OpenOptions},
    io::Write,
    path::Path,
    process::Command,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CompatibilitySettings {
    pub wine_version: Option<String>,
    pub windows_version: Option<String>,
    pub dll_overrides: Vec<DllOverride>,
    pub launch_env: Vec<EnvOverride>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DllOverride {
    pub name: String,
    pub mode: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EnvOverride {
    pub key: String,
    pub value: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TroubleshootingReport {
    pub game_id: String,
    pub settings_path: String,
    pub log_path: String,
    pub recent_logs: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProtonDbSummary {
    pub app_id: String,
    pub tier: Option<String>,
    pub score: Option<f64>,
    pub confidence: Option<String>,
    pub source: String,
}

pub fn save_settings(
    game: &GameRecord,
    settings: CompatibilitySettings,
) -> Result<TroubleshootingReport, String> {
    let dir = compatibility_dir(game)?;
    fs::create_dir_all(&dir)
        .map_err(|error| format!("Cannot create compatibility directory: {error}"))?;
    let settings_path = dir.join("settings.json");
    let bytes = serde_json::to_vec_pretty(&settings)
        .map_err(|error| format!("Cannot serialize compatibility settings: {error}"))?;
    fs::write(&settings_path, bytes)
        .map_err(|error| format!("Cannot write compatibility settings: {error}"))?;

    write_wine_overrides(game, &settings)?;
    report(game)
}

pub fn report(game: &GameRecord) -> Result<TroubleshootingReport, String> {
    let dir = compatibility_dir(game)?;
    fs::create_dir_all(&dir)
        .map_err(|error| format!("Cannot create compatibility directory: {error}"))?;
    let log_path = dir.join("errors.log");
    if !log_path.exists() {
        fs::write(&log_path, "")
            .map_err(|error| format!("Cannot create compatibility log: {error}"))?;
    }

    let recent_logs = fs::read_to_string(&log_path)
        .unwrap_or_default()
        .lines()
        .rev()
        .take(20)
        .map(ToString::to_string)
        .collect();

    Ok(TroubleshootingReport {
        game_id: game.game_id.clone(),
        settings_path: dir.join("settings.json").to_string_lossy().into_owned(),
        log_path: log_path.to_string_lossy().into_owned(),
        recent_logs,
    })
}

pub fn append_error(game: &GameRecord, message: String) -> Result<TroubleshootingReport, String> {
    let dir = compatibility_dir(game)?;
    fs::create_dir_all(&dir)
        .map_err(|error| format!("Cannot create compatibility directory: {error}"))?;
    let log_path = dir.join("errors.log");
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&log_path)
        .map_err(|error| format!("Cannot open compatibility log: {error}"))?;
    writeln!(file, "{message}")
        .map_err(|error| format!("Cannot write compatibility log: {error}"))?;
    report(game)
}

pub fn protondb_summary(app_id: String) -> Result<ProtonDbSummary, String> {
    if app_id.trim().is_empty() {
        return Err("Steam app id cannot be empty.".into());
    }

    let url = format!("https://www.protondb.com/api/v1/reports/summaries/{app_id}.json");
    let output = Command::new("curl")
        .args(["-L", "--fail", "--silent", "--show-error", &url])
        .output()
        .map_err(|error| format!("Failed to start curl: {error}"))?;

    if !output.status.success() {
        return Ok(ProtonDbSummary {
            app_id,
            tier: None,
            score: None,
            confidence: None,
            source: url,
        });
    }

    let value: Value = serde_json::from_slice(&output.stdout)
        .map_err(|error| format!("Cannot parse ProtonDB summary: {error}"))?;

    Ok(ProtonDbSummary {
        app_id,
        tier: value
            .get("tier")
            .and_then(Value::as_str)
            .map(ToString::to_string),
        score: value.get("score").and_then(Value::as_f64),
        confidence: value
            .get("confidence")
            .and_then(Value::as_str)
            .map(ToString::to_string),
        source: url,
    })
}

fn write_wine_overrides(game: &GameRecord, settings: &CompatibilitySettings) -> Result<(), String> {
    let Some(prefix) = &game.prefix_path else {
        return Ok(());
    };

    let overrides = settings
        .dll_overrides
        .iter()
        .filter(|override_item| !override_item.name.trim().is_empty())
        .map(|override_item| {
            format!(
                "{}={}",
                override_item.name.trim(),
                override_item.mode.trim()
            )
        })
        .collect::<Vec<_>>()
        .join(";");

    let dir = Path::new(prefix).join("ardali");
    fs::create_dir_all(&dir)
        .map_err(|error| format!("Cannot create prefix compatibility directory: {error}"))?;
    fs::write(
        dir.join("dll-overrides.env"),
        format!("WINEDLLOVERRIDES={overrides}\n"),
    )
    .map_err(|error| format!("Cannot write DLL override env file: {error}"))
}

fn compatibility_dir(game: &GameRecord) -> Result<std::path::PathBuf, String> {
    let paths = runtime::runtime_paths()?;
    Ok(Path::new(&paths.data_dir)
        .join("compatibility")
        .join(&game.game_id))
}
