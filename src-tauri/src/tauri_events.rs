use crate::runtime::RuntimeEventSink;
use ardali_core::{ArdaliError, ArdaliEvent, EventSink};
use tauri::{AppHandle, Emitter};

#[derive(Clone)]
pub struct TauriEventSink {
    app: AppHandle,
}

impl TauriEventSink {
    pub fn new(app: &AppHandle) -> Self {
        Self { app: app.clone() }
    }
}

impl EventSink for TauriEventSink {
    fn publish(&self, event: ArdaliEvent) -> Result<(), ArdaliError> {
        let result = match event {
            ArdaliEvent::Log(payload) => self.app.emit("backend-log", payload),
            ArdaliEvent::DownloadProgress(payload) => self.app.emit("download-progress", payload),
            ArdaliEvent::LibraryChanged { id } => self.app.emit("library-changed", id),
            ArdaliEvent::InstallProgress(payload) => self.app.emit("install-progress", payload),
            ArdaliEvent::CncNetInstallProgress(payload) => {
                self.app.emit("cncnet-install-progress", payload)
            }
            ArdaliEvent::GameStarted(payload) => self.app.emit("game-started", payload),
            ArdaliEvent::GameEnded(payload) => self.app.emit("game-ended", payload),
        };

        result.map_err(|error| ArdaliError::Other(format!("Cannot emit UI event: {error}")))
    }
}

impl RuntimeEventSink for TauriEventSink {
    fn publish_event(&self, event: ArdaliEvent) -> Result<(), String> {
        self.publish(event).map_err(|error| error.to_string())
    }
}

impl RuntimeEventSink for AppHandle {
    fn publish_event(&self, event: ArdaliEvent) -> Result<(), String> {
        TauriEventSink::new(self)
            .publish(event)
            .map_err(|error| error.to_string())
    }
}
