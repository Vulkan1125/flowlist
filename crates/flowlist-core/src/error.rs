//! Errors shared by the domain model, store and command engine.

/// Result alias used throughout Flowlist's core.
pub type Result<T> = std::result::Result<T, Error>;

/// Every failure the core can report. The CLI and MCP layers turn these into
/// exit codes and JSON-RPC tool errors respectively.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("i/o error: {0}")]
    Io(#[from] std::io::Error),

    #[error("invalid JSON: {0}")]
    Json(#[from] serde_json::Error),

    #[error("{kind} not found: {id}")]
    NotFound { kind: &'static str, id: String },

    #[error("invalid operation: {0}")]
    Invalid(String),

    #[error("id prefix '{prefix}' is ambiguous ({matches} matches)")]
    Ambiguous { prefix: String, matches: usize },

    #[error("storage is locked by another process (timed out)")]
    LockTimeout,

    #[error(
        "data was written by a newer Flowlist version (schema {found}, this build supports {supported})"
    )]
    SchemaTooNew { found: u32, supported: u32 },

    #[error("data file is corrupt and no readable backup exists: {0}")]
    Corrupt(String),

    #[error("invalid date or time: {0}")]
    Date(String),

    #[error("import failed: {0}")]
    Import(String),
}

impl Error {
    pub fn not_found(kind: &'static str, id: impl Into<String>) -> Self {
        Self::NotFound {
            kind,
            id: id.into(),
        }
    }

    pub fn invalid(message: impl Into<String>) -> Self {
        Self::Invalid(message.into())
    }
}
