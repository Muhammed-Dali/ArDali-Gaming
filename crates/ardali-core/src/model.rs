use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RunnerKind {
    Wine,
    Steam,
    Proton,
    Dxvk,
    Vkd3d,
    Openra,
    DosboxX,
    Cncnet,
}

impl RunnerKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Wine => "wine",
            Self::Steam => "steam",
            Self::Proton => "proton",
            Self::Dxvk => "dxvk",
            Self::Vkd3d => "vkd3d",
            Self::Openra => "openra",
            Self::DosboxX => "dosbox-x",
            Self::Cncnet => "cncnet",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum PrefixMode {
    Isolated,
    SharedWindowsApps,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
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

#[cfg(test)]
mod tests {
    use super::{DisplayMode, GameKind, LibraryType, PrefixMode, RunnerKind};

    #[test]
    fn request_models_keep_existing_json_names() {
        assert_eq!(
            serde_json::to_string(&GameKind::WindowsExe).unwrap(),
            "\"windows-exe\""
        );
        assert_eq!(
            serde_json::to_string(&LibraryType::WindowsApp).unwrap(),
            "\"windows-app\""
        );
        assert_eq!(
            serde_json::to_string(&PrefixMode::SharedWindowsApps).unwrap(),
            "\"shared-windows-apps\""
        );
        assert_eq!(
            serde_json::to_string(&DisplayMode::Fullscreen).unwrap(),
            "\"fullscreen\""
        );
        assert_eq!(
            serde_json::to_string(&RunnerKind::Wine).unwrap(),
            "\"wine\""
        );
    }

    #[test]
    fn string_values_keep_existing_storage_contracts() {
        assert_eq!(GameKind::OpenRaRedAlert.as_str(), "open-ra-red-alert");
        assert_eq!(LibraryType::Installer.as_str(), "installer");
        assert_eq!(DisplayMode::Windowed.as_str(), "windowed");
        assert_eq!(RunnerKind::DosboxX.as_str(), "dosbox-x");
        assert_eq!(RunnerKind::Steam.as_str(), "steam");
    }

    #[test]
    fn request_models_round_trip() {
        let value = serde_json::from_str::<GameKind>("\"steam\"").unwrap();
        assert_eq!(value, GameKind::Steam);
    }
}
