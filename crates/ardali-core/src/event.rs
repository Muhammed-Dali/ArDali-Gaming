use crate::ArdaliError;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", tag = "type", content = "payload")]
pub enum ArdaliEvent {
    Log(LogEvent),
    DownloadProgress(DownloadProgressEvent),
    LibraryChanged { id: Option<i64> },
    InstallProgress(DownloadProgressEvent),
    CncNetInstallProgress(CncNetInstallProgressEvent),
    GameStarted(GameProcessEvent),
    GameEnded(GameProcessEvent),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LogEvent {
    pub level: String,
    pub message: String,
    pub timestamp: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DownloadProgressEvent {
    pub kind: String,
    pub percent: u8,
    pub downloaded_bytes: u64,
    pub total_bytes: Option<u64>,
    pub status: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CncNetInstallProgressEvent {
    pub id: i64,
    pub percent: u8,
    pub status: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GameProcessEvent {
    pub id: i64,
    pub name: String,
    pub status: String,
}

/// UI adapters implement this contract to receive core events.
pub trait EventSink: Send + Sync {
    fn publish(&self, event: ArdaliEvent) -> Result<(), ArdaliError>;
}

#[cfg(test)]
mod tests {
    use super::{ArdaliEvent, CncNetInstallProgressEvent, DownloadProgressEvent};

    #[test]
    fn events_have_a_stable_tagged_json_shape() {
        let event = ArdaliEvent::DownloadProgress(DownloadProgressEvent {
            kind: "wine".into(),
            percent: 42,
            downloaded_bytes: 420,
            total_bytes: Some(1_000),
            status: "downloading".into(),
        });

        let value = serde_json::to_value(event).unwrap();
        assert_eq!(value["type"], "downloadProgress");
        assert_eq!(value["payload"]["downloadedBytes"], 420);
        assert_eq!(value["payload"]["totalBytes"], 1_000);
    }

    #[test]
    fn adapter_events_keep_existing_frontend_payload_fields() {
        let event = ArdaliEvent::CncNetInstallProgress(CncNetInstallProgressEvent {
            id: 42,
            percent: 75,
            status: "installing".into(),
        });
        let value = serde_json::to_value(event).unwrap();

        assert_eq!(value["type"], "cncNetInstallProgress");
        assert_eq!(value["payload"]["id"], 42);
        assert_eq!(value["payload"]["percent"], 75);
    }

    #[test]
    fn library_change_can_carry_the_affected_record_id() {
        let event = ArdaliEvent::LibraryChanged { id: Some(7) };
        let value = serde_json::to_value(event).unwrap();

        assert_eq!(value["type"], "libraryChanged");
        assert_eq!(value["payload"]["id"], 7);
    }
}
