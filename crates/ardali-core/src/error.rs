use std::{error::Error, fmt};

/// Stable error categories that a UI can translate without parsing messages.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ArdaliError {
    InvalidInput(String),
    NotFound(String),
    Io(String),
    Process(String),
    Storage(String),
    Network(String),
    Cancelled,
    Other(String),
}

impl ArdaliError {
    pub fn message(&self) -> &str {
        match self {
            Self::InvalidInput(message)
            | Self::NotFound(message)
            | Self::Io(message)
            | Self::Process(message)
            | Self::Storage(message)
            | Self::Network(message)
            | Self::Other(message) => message,
            Self::Cancelled => "Operation cancelled",
        }
    }
}

impl fmt::Display for ArdaliError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.message())
    }
}

impl Error for ArdaliError {}

impl From<String> for ArdaliError {
    fn from(message: String) -> Self {
        Self::Other(message)
    }
}

impl From<&str> for ArdaliError {
    fn from(message: &str) -> Self {
        Self::Other(message.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::ArdaliError;

    #[test]
    fn cancellation_has_a_stable_message() {
        assert_eq!(ArdaliError::Cancelled.to_string(), "Operation cancelled");
    }

    #[test]
    fn string_conversion_preserves_the_message() {
        let error = ArdaliError::from("runner failed");
        assert_eq!(error, ArdaliError::Other("runner failed".into()));
    }
}
