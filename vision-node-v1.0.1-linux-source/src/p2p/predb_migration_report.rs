use once_cell::sync::Lazy;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};

/// Why a peer was dropped during the guarded migration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DropReason {
    WrongPort,
    InvalidIP,
    SelfIP,
    Banned,
    BlockedCIDR,
    Duplicate,
}

/// Example of a dropped peer for debugging (capped)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DropSample {
    pub peer_id: String,
    pub last_ip: String,
    pub last_port: u16,
    pub reason: DropReason,
}

/// Report for the pre-DB → DB migration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PredbMigrationReport {
    pub ran: bool,
    pub skipped: bool,
    pub captured: usize,
    pub kept: usize,
    pub dropped: usize,
    pub cap: usize,
    pub when_ms: u128,
    pub samples: Vec<DropSample>,
    pub counts_wrong_port: usize,
    pub counts_invalid_ip: usize,
    pub counts_self_ip: usize,
    pub counts_banned: usize,
    pub counts_blocked_cidr: usize,
    pub counts_duplicate: usize,
    pub dropped_by_cap: u64,
}

/// Marker stored in sled meta tree after successful migration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PredbMigrationMarker {
    pub when_ms: u128,
    pub captured: usize,
    pub kept: usize,
    pub dropped: usize,
    pub cap: usize,
}

static REPORT: Lazy<RwLock<Option<PredbMigrationReport>>> = Lazy::new(|| RwLock::new(None));

pub fn set_report(report: PredbMigrationReport) {
    let mut lock = REPORT.write();
    *lock = Some(report);
}

pub fn get_report() -> Option<PredbMigrationReport> {
    REPORT.read().clone()
}
