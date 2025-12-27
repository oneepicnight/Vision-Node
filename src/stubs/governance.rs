//! Stub for governance module when staging is disabled
//! Non-custodial: provides safe stubs with default values

use serde::{Deserialize, Serialize};

mod dev_payouts {
    use serde::{Deserialize, Serialize};
    
    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct DevPayout {
        pub address: String,
        pub amount: u128,
    }
    
    pub fn get_dev_payouts() -> Result<Vec<DevPayout>, String> {
        Ok(vec![])
    }
}

pub use dev_payouts::{DevPayout, get_dev_payouts};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum GovernanceStatus {
    Open,
    Passed,
    Rejected,
    Expired,
}

impl GovernanceStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            GovernanceStatus::Open => "open",
            GovernanceStatus::Passed => "passed",
            GovernanceStatus::Rejected => "rejected",
            GovernanceStatus::Expired => "expired",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum GovernanceAction {
    TextOnly,
    AddBoardMember { address: String, payout_bps: u32 },
    RemoveBoardMember { address: String },
    UpdateDevPayout { address: String, payout_bps: u32 },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GovernanceProposal {
    pub id: u64,
    pub proposer: String,
    pub title: String,
    pub body: String,
    pub action: GovernanceAction,
    pub created_at: u64,
    pub voting_ends_at: u64,
    pub status: GovernanceStatus,
    pub yes_votes: u64,
    pub no_votes: u64,
}

impl Default for GovernanceProposal {
    fn default() -> Self {
        Self {
            id: 0,
            proposer: String::new(),
            title: String::new(),
            body: String::new(),
            action: GovernanceAction::TextOnly,
            created_at: 0,
            voting_ends_at: 0,
            status: GovernanceStatus::Open,
            yes_votes: 0,
            no_votes: 0,
        }
    }
}

// Additional stubs for governance functions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GovernanceVote {
    pub voter: String,
    pub proposal_id: u64,
    pub yes: bool,
}

pub struct GovernanceManager;

impl GovernanceManager {
    pub fn new(_db: &sled::Db) -> Result<Self, String> {
        Ok(GovernanceManager)
    }

    pub fn create_proposal(_proposal: &GovernanceProposal) -> Result<u64, String> {
        Ok(0)
    }

    pub fn get_proposal(_id: u64) -> Result<Option<GovernanceProposal>, String> {
        Ok(None)
    }

    pub fn cast_vote(_vote: &GovernanceVote) -> Result<(), String> {
        Ok(())
    }
}

pub fn calculate_vote_weight(_address: &str) -> Result<u64, String> {
    Ok(1)
}

pub fn can_vote(_address: &str) -> bool {
    false
}

pub fn get_all_eligible_voters() -> Result<Vec<String>, String> {
    Ok(vec![])
}
