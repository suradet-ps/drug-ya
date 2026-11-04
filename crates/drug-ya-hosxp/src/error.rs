//! Error type for the HOSxP repository layer.

use thiserror::Error;

/// Errors produced by the HOSxP repository.
#[derive(Debug, Error)]
pub enum Error {
    /// Failed to establish the connection pool.
    #[error("failed to connect to HOSxP at {host}:{port}/{database}: {source}")]
    Connect {
        /// Host from the config.
        host: String,
        /// Port from the config.
        port: u16,
        /// Database name.
        database: String,
        /// Underlying driver error.
        source: sqlx::Error,
    },

    /// A query was rejected by the read-only guard.
    #[error("read-only guard rejected statement: {0}")]
    ReadOnlyViolation(String),

    /// A row from the database did not have the expected shape.
    #[error("unexpected HOSxP row shape: {0}")]
    RowShape(String),

    /// A required row was missing.
    #[error("{0}")]
    NotFound(String),

    /// A schema variant this feature cannot support (e.g. no queue column).
    #[error("unsupported HOSxP schema: {0}")]
    UnsupportedSchema(String),

    /// Any other database/driver failure.
    #[error("HOSxP database error: {0}")]
    Database(#[from] sqlx::Error),
}

/// Convenience result alias.
pub type Result<T> = std::result::Result<T, Error>;
