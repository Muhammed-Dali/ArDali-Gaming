//! UI-independent domain contracts for ArDali Gaming.
//!
//! This crate must not depend on Tauri, GTK, Slint, or another UI toolkit.

mod error;
mod event;
mod model;

pub use error::ArdaliError;
pub use event::{
    ArdaliEvent, CncNetInstallProgressEvent, DownloadProgressEvent, EventSink, GameProcessEvent,
    LogEvent,
};
pub use model::{DisplayMode, GameKind, LibraryType, PrefixMode, RunnerKind};
