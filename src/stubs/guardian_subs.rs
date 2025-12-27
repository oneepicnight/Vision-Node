//! Stub modules for guardian sub-components
//! Non-custodial: no key access, safe defaults

use serde::{Deserialize, Serialize};

// --- Consciousness ---
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Consciousness {
    pub id: String,
}

// --- Creator Config ---
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

// --- Events ---
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GuardianEvent {
    pub name: String,
}

pub fn send_guardian_event(_event: &GuardianEvent) -> Result<(), String> {
    Ok(())
}

// --- Integrity ---
#[derive(Debug, Clone)]
pub struct Integrity {
    pub valid: bool,
}

// --- Role ---
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

// --- Rotation ---
pub fn is_local_guardian() -> bool {
    false
}

pub fn spawn_guardian_rotation_loop() {
    // No-op stub
}
