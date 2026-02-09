use dioxus::prelude::ServerFnError;
use joydb::JoydbError;
use crate::cqrs::framework::{CommandError};
use crate::import::ImportError;

#[derive(Debug, thiserror::Error)]
pub enum RustyError {
    #[error("Database Error: {0}")]
    JoydbError(JoydbError),
    #[error("DefaultBudgetNotFound")]
    DefaultBudgetNotFound,
    #[error("ImportError: {0}")]
    ImportError(ImportError),
    #[error("ItemNotFound: {0}")]
    ItemNotFound(String, String),
    #[error("Command error: {0}")]
    CommandError(CommandError),
    #[error("Serde error: {0}")]
    SerdeError(serde_json::Error),
    #[error("Parse error: {0}")]
    ParseError(chrono::ParseError),
}

impl From<chrono::ParseError> for RustyError {
    fn from(value: chrono::ParseError) -> Self {
        RustyError::ParseError(value)
    }
}

impl From<serde_json::Error> for RustyError {
    fn from(value: serde_json::Error) -> Self {
        RustyError::SerdeError(value)
    }
}

impl From<CommandError> for RustyError {
    fn from(value: CommandError) -> Self {
        RustyError::CommandError(value)
    }
}

impl From<JoydbError> for RustyError {
    fn from(value: JoydbError) -> Self {
        match value {
            JoydbError::NotFound { id, model } => RustyError::ItemNotFound(id, model),
            _ => RustyError::JoydbError(value)
        }
    }
}

impl From<ImportError> for RustyError {
    fn from(value: ImportError) -> Self {
        RustyError::ImportError(value)
    }
}

impl From<RustyError> for ServerFnError {
    fn from(value: RustyError) -> Self {
        ServerFnError::new(value)
    }
}