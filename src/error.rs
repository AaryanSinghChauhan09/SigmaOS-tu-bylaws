//! Error types for the tu-bylaws component.

/// Errors that can occur parsing bylaws or running SVP.
#[derive(Debug, thiserror::Error)]
pub enum BylawsError {
    /// Document could not be read.
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),

    /// SVP rule violation (vote outside voting period, etc.).
    #[error("svp error: {0}")]
    Svp(String),
}
