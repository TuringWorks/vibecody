//! Error types for the Fluxo core.

use thiserror::Error;

/// Errors produced while parsing, validating, or executing a workflow definition.
#[derive(Debug, Error)]
pub enum FluxoError {
    /// The workflow definition failed structural validation.
    #[error("invalid workflow definition: {0}")]
    InvalidDefinition(String),

    /// A task type is not yet supported by the decider.
    #[error("unsupported task type: {0}")]
    UnsupportedTaskType(String),

    /// An `${…}` expression could not be resolved.
    #[error("expression error: {0}")]
    Expression(String),

    /// JSON (de)serialization failed.
    #[error("serialization error: {0}")]
    Serde(#[from] serde_json::Error),

    /// A referenced entity was not found.
    #[error("not found: {0}")]
    NotFound(String),

    /// The store backend reported an error.
    #[error("store error: {0}")]
    Store(String),

    /// The run reached a state the caller did not expect.
    #[error("invalid state: {0}")]
    InvalidState(String),
}

/// Convenience alias for results in the Fluxo core.
pub type Result<T> = std::result::Result<T, FluxoError>;
