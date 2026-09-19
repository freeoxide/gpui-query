use serde::{Deserialize, Serialize};

/// Trigger configuration for automatic refetching.
///
/// The triggers are parsed and stored, but the event system integration
/// (window focus, reconnect) is not implemented yet.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum RefetchTrigger {
    /// Always refetch when the trigger fires.
    #[default]
    Always,
    /// Refetch only if the data is stale (past TTL).
    IfStale,
    /// Never refetch on this trigger.
    Never,
}

impl RefetchTrigger {
    /// Human-readable label.
    pub fn label(self) -> &'static str {
        match self {
            Self::Always => "Always",
            Self::IfStale => "If stale",
            Self::Never => "Never",
        }
    }
}
