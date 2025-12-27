//! Stub implementation of guardian module when staging feature is disabled
//! Non-custodial: provides safe stubs with no key access

use serde::{Deserialize, Serialize};

// Inline stubs for guardian submodules
mod consciousness {
    use serde::{Deserialize, Serialize};
    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct Consciousness {
        pub id: String,
    }
}

mod creator_config {
    use serde::{Deserialize, Serialize};
    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct CreatorConfig {
        pub address: String,
    }
    pub fn is_creator_address(_addr: &str) -> bool {
        false
    }
    pub fn load_creator_config() -> Result<Option<CreatorConfig>, String> {
        Ok(None)
    }
}

mod events {
    use serde::{Deserialize, Serialize};
    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct GuardianEvent {
        pub name: String,
    }
    pub fn send_guardian_event(_event: &GuardianEvent) -> Result<(), String> {
        Ok(())
    }
}

mod integrity {
    #[derive(Debug, Clone)]
    pub struct Integrity {
        pub valid: bool,
    }
}

mod role {
    use serde::{Deserialize, Serialize};
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
    pub enum GuardianRole {
        Dormant,
        Active,
        Rotating,
    }
    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct GuardianRoleConfig {
        pub role: GuardianRole,
    }
}

mod rotation {
    pub fn is_local_guardian() -> bool {
        false
    }
    pub fn spawn_guardian_rotation_loop() {
        // No-op stub
    }
}

pub use creator_config::{is_creator_address, load_creator_config};
pub use role::{GuardianRole, GuardianRoleConfig};
pub use rotation::{is_local_guardian, spawn_guardian_rotation_loop};
pub use events::GuardianEvent;

/// Stub guardian function - no-op
pub fn guardian() -> Option<String> {
    None
}

/// Stub init_guardian - no-op
pub fn init_guardian() {
    // No-op stub
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GuardianState {
    pub active: bool,
}

impl Default for GuardianState {
    fn default() -> Self {
        Self { active: false }
    }
}
