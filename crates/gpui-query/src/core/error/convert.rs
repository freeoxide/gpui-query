//! Standard trait implementations for [`QueryError`](super::QueryError).

use super::types::QueryError;

impl std::fmt::Display for QueryError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.kind(), self.message())
    }
}

impl std::error::Error for QueryError {}

impl AsRef<str> for QueryError {
    fn as_ref(&self) -> &str {
        self.message()
    }
}

impl AsRef<std::sync::Arc<str>> for QueryError {
    fn as_ref(&self) -> &std::sync::Arc<str> {
        &self.message
    }
}

impl From<String> for QueryError {
    /// Creates a [`QueryError`] with kind [`QueryErrorKind::Unknown`](super::QueryErrorKind::Unknown).
    ///
    /// `From<String>` and `From<&str>` always map to `Unknown` because the
    /// original error category cannot be recovered from a plain string. Use
    /// [`QueryError::transport`], [`QueryError::response`], or
    /// [`QueryError::cancelled`] for typed errors.
    fn from(value: String) -> Self {
        Self::unknown(value)
    }
}

impl From<&str> for QueryError {
    /// Creates a [`QueryError`] with kind [`QueryErrorKind::Unknown`](super::QueryErrorKind::Unknown).
    ///
    /// See [`From<String>`] for rationale on the `Unknown` mapping.
    fn from(value: &str) -> Self {
        Self::unknown(value)
    }
}
