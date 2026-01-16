//! Stub implementation of guardian module when staging feature is disabled
//! Non-custodial: provides safe stubs with no key access

use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use sled;

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
        pub creator_address: String,
        pub name: String,
        pub title: String,
    }

    impl Default for CreatorConfig {
        fn default() -> Self {
            Self {
                creator_address: String::new(),
                name: "".to_string(),
                title: "".to_string(),
            }
        }
    }

    pub fn is_creator_address(_db: &sled::Db, _addr: &str) -> bool {
        false
    }
    pub fn load_creator_config(_db: &sled::Db) -> Result<CreatorConfig, String> {
        Ok(CreatorConfig::default())
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

/// Minimal stub guardian struct providing welcome/farewell hooks
#[derive(Default)]
pub struct GuardianStub;

impl GuardianStub {
    pub async fn welcome_star(&self, _peer_id: &str, _addr: Option<&str>, _region: Option<&str>) {}
    pub async fn farewell_star(&self, _peer_id: &str, _addr: Option<&str>) {}
}

static GUARDIAN: Lazy<Arc<GuardianStub>> = Lazy::new(|| Arc::new(GuardianStub::default()));

/// Stub guardian accessor
pub fn guardian() -> &'static Arc<GuardianStub> {
    &GUARDIAN
}

/// Stub init_guardian - no-op
pub fn init_guardian() {
    let _ = GUARDIAN.as_ref();
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
