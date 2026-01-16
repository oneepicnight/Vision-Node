//! Stub implementations for sub-modules of guardian
//! Used when staging feature is disabled

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GuardianConsciousness {
    pub id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GuardianIntegrity {
    pub status: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GuardianEvent {
    pub name: String,
}
