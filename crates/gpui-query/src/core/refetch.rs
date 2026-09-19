use serde::{Deserialize, Serialize};

/// Parsed and stored, but focus/reconnect event integration is not implemented yet.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum RefetchTrigger {
    #[default]
    Always,
    IfStale,
    Never,
}

impl RefetchTrigger {
    pub fn label(self) -> &'static str {
        match self {
            Self::Always => "Always",
            Self::IfStale => "If stale",
            Self::Never => "Never",
        }
    }
}
