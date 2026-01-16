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
    pub required_majority_bps: u32,
}

impl GovernanceProposal {
    pub fn is_expired(&self, now: u64) -> bool {
        now > self.voting_ends_at
    }

    pub fn majority_result(&self) -> Option<bool> {
        let total = self.yes_votes + self.no_votes;
        if total == 0 {
            return None;
        }
        let yes_bps = (self.yes_votes * 10_000) / total;
        Some(yes_bps >= self.required_majority_bps as u64)
    }

    pub fn time_remaining(&self, now: u64) -> u64 {
        self.voting_ends_at.saturating_sub(now)
    }

    pub fn yes_percentage_bps(&self) -> u32 {
        let total = self.yes_votes + self.no_votes;
        if total == 0 {
            return 0;
        }
        ((self.yes_votes * 10_000) / total) as u32
    }
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
            required_majority_bps: 5100,
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GovernanceConfig {
    pub proposal_fee_land: u128,
    pub voting_duration_secs: u64,
    pub majority_ratio_bps: u32,
}

impl Default for GovernanceConfig {
    fn default() -> Self {
        Self {
            proposal_fee_land: 10_000_000_000_000,
            voting_duration_secs: 48 * 3600,
            majority_ratio_bps: 5100,
        }
    }
}

pub struct GovernanceManager {
    pub db: sled::Db,
}

impl GovernanceManager {
    pub fn new(db: &sled::Db) -> Result<Self, String> {
        Ok(GovernanceManager { db: db.clone() })
    }

    pub fn config(&self) -> GovernanceConfig {
        GovernanceConfig::default()
    }

    pub fn create_proposal(
        &self,
        proposer: String,
        title: String,
        body: String,
        action: GovernanceAction,
    ) -> Result<GovernanceProposal, String> {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        Ok(GovernanceProposal {
            id: 1,
            proposer,
            title,
            body,
            action,
            created_at: now,
            voting_ends_at: now + 48 * 3600,
            status: GovernanceStatus::Open,
            yes_votes: 0,
            no_votes: 0,
            required_majority_bps: 5100,
        })
    }

    pub fn get_proposal(&self, _id: u64) -> Result<Option<GovernanceProposal>, String> {
        Ok(None)
    }

    pub fn list_proposals(
        &self,
        _status_filter: Option<GovernanceStatus>,
        _limit: usize,
    ) -> Result<Vec<GovernanceProposal>, String> {
        Ok(vec![])
    }

    pub fn get_proposal_votes(&self, _proposal_id: u64) -> Result<Vec<GovernanceVote>, String> {
        Ok(vec![])
    }

    pub fn has_voted(&self, _proposal_id: u64, _voter: &str) -> Result<bool, String> {
        Ok(false)
    }

    pub fn get_vote(&self, _proposal_id: u64, _voter: &str) -> Result<Option<GovernanceVote>, String> {
        Ok(None)
    }

    pub fn cast_vote(
        &self,
        _proposal_id: u64,
        _voter: String,
        _support: bool,
        _weight: u64,
    ) -> Result<GovernanceVote, String> {
        Ok(GovernanceVote {
            voter: _voter,
            proposal_id: _proposal_id,
            yes: _support,
        })
    }
}

pub fn calculate_vote_weight(_db: &sled::Db, _address: &str) -> u64 {
    1
}

pub fn can_vote(_db: &sled::Db, _address: &str) -> bool {
    false
}

pub fn get_all_eligible_voters(_db: &sled::Db) -> Vec<String> {
    vec![]
}
