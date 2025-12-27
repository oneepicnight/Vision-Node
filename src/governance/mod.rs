//! Governance System - On-chain Proposals and Voting
//!
//! "DING DONG BITCH – New governance proposal is live."
//!
//! This module implements Vision's governance system where deed holders and founders
//! can propose and vote on changes to the network. Proposals cost 10k LAND and require
//! 51% majority to pass within 48 hours.
#![allow(dead_code)]

pub mod dev_payouts;

use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use sled::Db;
use tracing::{error, info};

const GOVERNANCE_PROPOSALS_TREE: &str = "governance_proposals";
const GOVERNANCE_VOTES_TREE: &str = "governance_votes";
const GOVERNANCE_CONFIG_TREE: &str = "governance_config";
const GOVERNANCE_NEXT_ID_KEY: &str = "next_proposal_id";

/// Status of a governance proposal
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum GovernanceStatus {
    /// Voting is open
    Open,
    /// Proposal passed (51%+ yes votes)
    Passed,
    /// Proposal rejected (failed to reach majority)
    Rejected,
    /// Voting period expired with no clear majority or no votes
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

/// Actions that a governance proposal can execute
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum GovernanceAction {
    /// Pure signaling - no automatic chain changes
    TextOnly,

    /// Add a new board member with payout allocation
    AddBoardMember {
        address: String,
        payout_bps: u32, // basis points (e.g., 1000 = 10%)
    },

    /// Remove a board member
    RemoveBoardMember { address: String },

    /// Update payout allocation for existing dev/board member
    UpdateDevPayout { address: String, payout_bps: u32 },
    // Future actions:
    // UpdateParam { key: String, value: String },
    // AddFounder { address: String },
    // RemoveFounder { address: String },
}

/// A governance proposal
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GovernanceProposal {
    /// Unique proposal ID
    pub id: u64,

    /// Address of the proposer
    pub proposer: String,

    /// Proposal title
    pub title: String,

    /// Proposal body/description
    pub body: String,

    /// Action to execute if passed
    pub action: GovernanceAction,

    /// When the proposal was created (unix timestamp)
    pub created_at: u64,

    /// When voting ends (unix timestamp)
    pub voting_ends_at: u64,

    /// Current status
    pub status: GovernanceStatus,

    /// Total yes votes (weighted)
    pub yes_votes: u64,

    /// Total no votes (weighted)
    pub no_votes: u64,

    /// Required majority in basis points (e.g., 5100 = 51%)
    pub required_majority_bps: u32,
}

impl GovernanceProposal {
    /// Check if voting period has expired
    pub fn is_expired(&self, now: u64) -> bool {
        now > self.voting_ends_at
    }

    /// Check if proposal has reached majority
    /// Returns Some(true) if passed, Some(false) if rejected, None if no votes
    pub fn majority_result(&self) -> Option<bool> {
        let total = self.yes_votes + self.no_votes;
        if total == 0 {
            return None; // no votes
        }

        let yes_bps = (self.yes_votes * 10_000) / total;
        Some(yes_bps >= self.required_majority_bps as u64)
    }

    /// Get time remaining in seconds (0 if expired)
    pub fn time_remaining(&self, now: u64) -> u64 {
        self.voting_ends_at.saturating_sub(now)
    }

    /// Get yes percentage (0-10000 basis points)
    pub fn yes_percentage_bps(&self) -> u32 {
        let total = self.yes_votes + self.no_votes;
        if total == 0 {
            return 0;
        }
        ((self.yes_votes * 10_000) / total) as u32
    }
}

/// A vote on a proposal
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GovernanceVote {
    /// Proposal ID being voted on
    pub proposal_id: u64,

    /// Address of the voter
    pub voter: String,

    /// Vote choice (true = yes, false = no)
    pub support: bool,

    /// Vote weight (usually 1 deed = 1 vote)
    pub weight: u64,

    /// When the vote was cast
    pub timestamp: u64,
}

/// Governance configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GovernanceConfig {
    /// LAND fee to create a proposal (in base units)
    pub proposal_fee_land: u128,

    /// Voting duration in seconds (48 hours default)
    pub voting_duration_secs: u64,

    /// Required majority in basis points (5100 = 51%)
    pub majority_ratio_bps: u32,
}

impl Default for GovernanceConfig {
    fn default() -> Self {
        Self {
            proposal_fee_land: 10_000_000_000_000, // 10k LAND (with 9 decimals)
            voting_duration_secs: 48 * 3600,       // 48 hours
            majority_ratio_bps: 5100,              // 51%
        }
    }
}

/// Governance manager
#[derive(Debug)]
pub struct GovernanceManager {
    pub db: Db,
    proposals_tree: sled::Tree,
    votes_tree: sled::Tree,
    config_tree: sled::Tree,
    config: GovernanceConfig,
}

impl GovernanceManager {
    /// Create a new governance manager
    pub fn new(db: &Db) -> Result<Self> {
        let proposals_tree = db.open_tree(GOVERNANCE_PROPOSALS_TREE)?;
        let votes_tree = db.open_tree(GOVERNANCE_VOTES_TREE)?;
        let config_tree = db.open_tree(GOVERNANCE_CONFIG_TREE)?;

        // Load or initialize config
        let config = if let Some(bytes) = config_tree.get(b"config")? {
            serde_json::from_slice(&bytes)?
        } else {
            let default_config = GovernanceConfig::default();
            let bytes = serde_json::to_vec(&default_config)?;
            config_tree.insert(b"config", bytes)?;
            config_tree.flush()?;
            default_config
        };

        // Initialize next_proposal_id if not exists
        if proposals_tree
            .get(GOVERNANCE_NEXT_ID_KEY.as_bytes())?
            .is_none()
        {
            proposals_tree.insert(GOVERNANCE_NEXT_ID_KEY.as_bytes(), &1u64.to_be_bytes())?;
            proposals_tree.flush()?;
        }

        info!("[GOVERNANCE] System initialized");
        info!(
            "[GOVERNANCE] Proposal fee: {} LAND",
            config.proposal_fee_land as f64 / 1_000_000_000.0
        );
        info!(
            "[GOVERNANCE] Voting duration: {} hours",
            config.voting_duration_secs / 3600
        );
        info!(
            "[GOVERNANCE] Required majority: {}%",
            config.majority_ratio_bps as f64 / 100.0
        );

        Ok(Self {
            db: db.clone(),
            proposals_tree,
            votes_tree,
            config_tree,
            config,
        })
    }

    /// Get governance config
    pub fn config(&self) -> &GovernanceConfig {
        &self.config
    }

    /// Get next proposal ID and increment
    fn next_proposal_id(&self) -> Result<u64> {
        let current = self
            .proposals_tree
            .get(GOVERNANCE_NEXT_ID_KEY.as_bytes())?
            .map(|v| u64::from_be_bytes(v.as_ref().try_into().unwrap()))
            .unwrap_or(1);

        let next = current + 1;
        self.proposals_tree
            .insert(GOVERNANCE_NEXT_ID_KEY.as_bytes(), &next.to_be_bytes())?;
        self.proposals_tree.flush()?;

        Ok(current)
    }

    /// Create a new proposal
    pub fn create_proposal(
        &self,
        proposer: String,
        title: String,
        body: String,
        action: GovernanceAction,
    ) -> Result<GovernanceProposal> {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let id = self.next_proposal_id()?;

        let proposal = GovernanceProposal {
            id,
            proposer: proposer.clone(),
            title: title.clone(),
            body,
            action,
            created_at: now,
            voting_ends_at: now + self.config.voting_duration_secs,
            status: GovernanceStatus::Open,
            yes_votes: 0,
            no_votes: 0,
            required_majority_bps: self.config.majority_ratio_bps,
        };

        self.save_proposal(&proposal)?;

        info!(
            "[GOVERNANCE] Proposal #{} created by {}: \"{}\"",
            id, proposer, title
        );

        Ok(proposal)
    }

    /// Save a proposal
    pub fn save_proposal(&self, proposal: &GovernanceProposal) -> Result<()> {
        let key = proposal.id.to_be_bytes();
        let value = serde_json::to_vec(proposal)?;
        self.proposals_tree.insert(key, value)?;
        self.proposals_tree.flush()?;
        Ok(())
    }

    /// Get a proposal by ID
    pub fn get_proposal(&self, id: u64) -> Result<Option<GovernanceProposal>> {
        let key = id.to_be_bytes();
        if let Some(bytes) = self.proposals_tree.get(key)? {
            let proposal: GovernanceProposal = serde_json::from_slice(&bytes)?;
            Ok(Some(proposal))
        } else {
            Ok(None)
        }
    }

    /// List proposals with optional status filter
    pub fn list_proposals(
        &self,
        status_filter: Option<GovernanceStatus>,
        limit: usize,
    ) -> Result<Vec<GovernanceProposal>> {
        let mut proposals = Vec::new();

        for item in self.proposals_tree.iter() {
            let (key, value) = item?;

            // Skip the next_id key
            if key.as_ref() == GOVERNANCE_NEXT_ID_KEY.as_bytes() {
                continue;
            }

            let proposal: GovernanceProposal = serde_json::from_slice(&value)?;

            if let Some(filter) = status_filter {
                if proposal.status != filter {
                    continue;
                }
            }

            proposals.push(proposal);

            if proposals.len() >= limit {
                break;
            }
        }

        // Sort by ID descending (newest first)
        proposals.sort_by(|a, b| b.id.cmp(&a.id));

        Ok(proposals)
    }

    /// Cast a vote on a proposal
    pub fn cast_vote(
        &self,
        proposal_id: u64,
        voter: String,
        support: bool,
        weight: u64,
    ) -> Result<GovernanceVote> {
        // Load proposal
        let mut proposal = self
            .get_proposal(proposal_id)?
            .ok_or_else(|| anyhow!("Proposal not found"))?;

        // Check if still open
        if proposal.status != GovernanceStatus::Open {
            return Err(anyhow!("Proposal is not open for voting"));
        }

        // Check if expired
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        if proposal.is_expired(now) {
            return Err(anyhow!("Voting period has expired"));
        }

        // Check if already voted
        if self.has_voted(proposal_id, &voter)? {
            return Err(anyhow!("Already voted on this proposal"));
        }

        // Create vote
        let vote = GovernanceVote {
            proposal_id,
            voter: voter.clone(),
            support,
            weight,
            timestamp: now,
        };

        // Save vote
        self.save_vote(&vote)?;

        // Update proposal tallies
        if support {
            proposal.yes_votes += weight;
        } else {
            proposal.no_votes += weight;
        }

        self.save_proposal(&proposal)?;

        info!(
            "[GOVERNANCE] Vote cast on proposal #{}: {} votes {} (weight: {})",
            proposal_id,
            voter,
            if support { "YES" } else { "NO" },
            weight
        );

        Ok(vote)
    }

    /// Save a vote
    fn save_vote(&self, vote: &GovernanceVote) -> Result<()> {
        let key = format!("{}:{}", vote.proposal_id, vote.voter);
        let value = serde_json::to_vec(vote)?;
        self.votes_tree.insert(key.as_bytes(), value)?;
        self.votes_tree.flush()?;
        Ok(())
    }

    /// Check if an address has already voted on a proposal
    pub fn has_voted(&self, proposal_id: u64, voter: &str) -> Result<bool> {
        let key = format!("{}:{}", proposal_id, voter);
        Ok(self.votes_tree.contains_key(key.as_bytes())?)
    }

    /// Get a user's vote on a proposal
    pub fn get_vote(&self, proposal_id: u64, voter: &str) -> Result<Option<GovernanceVote>> {
        let key = format!("{}:{}", proposal_id, voter);
        if let Some(bytes) = self.votes_tree.get(key.as_bytes())? {
            let vote: GovernanceVote = serde_json::from_slice(&bytes)?;
            Ok(Some(vote))
        } else {
            Ok(None)
        }
    }

    /// Get all votes for a proposal
    pub fn get_proposal_votes(&self, proposal_id: u64) -> Result<Vec<GovernanceVote>> {
        let prefix = format!("{}:", proposal_id);
        let mut votes = Vec::new();

        for item in self.votes_tree.scan_prefix(prefix.as_bytes()) {
            let (_key, value) = item?;
            let vote: GovernanceVote = serde_json::from_slice(&value)?;
            votes.push(vote);
        }

        Ok(votes)
    }

    /// Get all open proposals that have expired and need closing
    pub fn get_expired_open_proposals(&self) -> Result<Vec<GovernanceProposal>> {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let mut expired = Vec::new();

        for item in self.proposals_tree.iter() {
            let (key, value) = item?;

            // Skip the next_id key
            if key.as_ref() == GOVERNANCE_NEXT_ID_KEY.as_bytes() {
                continue;
            }

            let proposal: GovernanceProposal = serde_json::from_slice(&value)?;

            if proposal.status == GovernanceStatus::Open && proposal.is_expired(now) {
                expired.push(proposal);
            }
        }

        Ok(expired)
    }

    /// Close an expired proposal and update its status
    pub fn close_proposal(&self, proposal_id: u64) -> Result<(GovernanceProposal, bool)> {
        let mut proposal = self
            .get_proposal(proposal_id)?
            .ok_or_else(|| anyhow!("Proposal not found"))?;

        if proposal.status != GovernanceStatus::Open {
            return Err(anyhow!("Proposal is not open"));
        }

        // Determine outcome
        let passed = match proposal.majority_result() {
            Some(true) => {
                proposal.status = GovernanceStatus::Passed;
                true
            }
            Some(false) => {
                proposal.status = GovernanceStatus::Rejected;
                false
            }
            None => {
                proposal.status = GovernanceStatus::Expired;
                false
            }
        };

        self.save_proposal(&proposal)?;

        info!(
            "[GOVERNANCE] Proposal #{} closed: {} ({}% yes)",
            proposal_id,
            proposal.status.as_str(),
            proposal.yes_percentage_bps() as f64 / 100.0
        );

        Ok((proposal, passed))
    }
}

/// Helper to check if an address can vote
/// Checks if address is founder or owns a land deed
pub fn can_vote(db: &sled::Db, address: &str) -> bool {
    // Check if founder (hardcoded founder address)
    let is_founder =
        address.to_lowercase() == crate::vision_constants::FOUNDER_ADDRESS.to_lowercase();

    // Check if address owns a land deed
    let has_deed = crate::land_deeds::wallet_has_deed(db, address);

    is_founder || has_deed
}

/// Calculate vote weight for an address
/// For now: 1 vote per deed holder or founder
pub fn calculate_vote_weight(db: &sled::Db, address: &str) -> u64 {
    if can_vote(db, address) {
        1
    } else {
        0
    }
}

/// Get all eligible voters (founder + deed holders)
pub fn get_all_eligible_voters(db: &sled::Db) -> Vec<String> {
    let mut voters = Vec::new();

    // Add founder
    voters.push(crate::vision_constants::FOUNDER_ADDRESS.to_string());

    // Add all deed holders using the land_deeds module function
    let deed_owners = crate::land_deeds::all_deed_owners(db);
    for owner in deed_owners {
        if !voters.contains(&owner) {
            voters.push(owner);
        }
    }

    voters
}

/// Background task to close expired proposals
pub async fn spawn_governance_closer_loop(
    manager: Arc<GovernanceManager>,
    dev_payout_manager: Arc<parking_lot::Mutex<dev_payouts::DevPayoutManager>>,
    check_interval_secs: u64,
) {
    use tokio::time::{interval, Duration};

    let mut ticker = interval(Duration::from_secs(check_interval_secs));

    info!(
        "[GOVERNANCE] Closer loop started (check interval: {}s)",
        check_interval_secs
    );

    loop {
        ticker.tick().await;

        // Find expired open proposals
        match manager.get_expired_open_proposals() {
            Ok(expired) => {
                for proposal in expired {
                    info!("[GOVERNANCE] Closing expired proposal #{}...", proposal.id);

                    match manager.close_proposal(proposal.id) {
                        Ok((closed_proposal, passed)) => {
                            if passed {
                                info!(
                                    "[GOVERNANCE] Proposal #{} PASSED - applying action...",
                                    proposal.id
                                );

                                // Apply the governance action
                                let mut dev_payout = dev_payout_manager.lock();
                                if let Err(e) =
                                    dev_payout.apply_governance_action(&closed_proposal.action)
                                {
                                    error!(
                                        "[GOVERNANCE] Failed to apply action for proposal #{}: {}",
                                        proposal.id, e
                                    );
                                } else {
                                    info!(
                                        "[GOVERNANCE] Action applied successfully for proposal #{}",
                                        proposal.id
                                    );
                                }
                            } else {
                                info!(
                                    "[GOVERNANCE] Proposal #{} REJECTED/EXPIRED ({}% yes)",
                                    proposal.id,
                                    closed_proposal.yes_percentage_bps() as f64 / 100.0
                                );
                            }

                            // Notify all eligible voters about the result
                            let eligible_voters = get_all_eligible_voters(&manager.db);
                            for voter in eligible_voters {
                                crate::ws_notifications::WsNotificationManager::notify_governance_proposal_closed(
                                    &voter,
                                    closed_proposal.id,
                                    closed_proposal.status.as_str(),
                                    closed_proposal.yes_votes,
                                    closed_proposal.no_votes,
                                );
                            }
                        }
                        Err(e) => {
                            error!(
                                "[GOVERNANCE] Failed to close proposal #{}: {}",
                                proposal.id, e
                            );
                        }
                    }
                }
            }
            Err(e) => {
                error!("[GOVERNANCE] Failed to get expired proposals: {}", e);
            }
        }
    }
}

use std::sync::Arc;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_governance_config_defaults() {
        let config = GovernanceConfig::default();
        assert_eq!(config.proposal_fee_land, 10_000_000_000_000);
        assert_eq!(config.voting_duration_secs, 48 * 3600);
        assert_eq!(config.majority_ratio_bps, 5100);
    }

    #[test]
    fn test_proposal_lifecycle() {
        let db = sled::Config::new().temporary(true).open().unwrap();
        let manager = GovernanceManager::new(&db).unwrap();

        // Create proposal
        let proposal = manager
            .create_proposal(
                "proposer123".to_string(),
                "Test Proposal".to_string(),
                "This is a test".to_string(),
                GovernanceAction::TextOnly,
            )
            .unwrap();

        assert_eq!(proposal.id, 1);
        assert_eq!(proposal.status, GovernanceStatus::Open);
        assert_eq!(proposal.yes_votes, 0);
        assert_eq!(proposal.no_votes, 0);

        // Vote yes
        manager
            .cast_vote(proposal.id, "voter1".to_string(), true, 1)
            .unwrap();

        // Check vote recorded
        assert!(manager.has_voted(proposal.id, "voter1").unwrap());

        // Try to vote again (should fail)
        assert!(manager
            .cast_vote(proposal.id, "voter1".to_string(), false, 1)
            .is_err());

        // Load updated proposal
        let updated = manager.get_proposal(proposal.id).unwrap().unwrap();
        assert_eq!(updated.yes_votes, 1);
    }

    #[test]
    fn test_majority_calculation() {
        let mut proposal = GovernanceProposal {
            id: 1,
            proposer: "test".to_string(),
            title: "Test".to_string(),
            body: "Test".to_string(),
            action: GovernanceAction::TextOnly,
            created_at: 0,
            voting_ends_at: 1000,
            status: GovernanceStatus::Open,
            yes_votes: 51,
            no_votes: 49,
            required_majority_bps: 5100,
        };

        // 51% yes - should pass
        assert_eq!(proposal.majority_result(), Some(true));

        // 49% yes - should fail
        proposal.yes_votes = 49;
        proposal.no_votes = 51;
        assert_eq!(proposal.majority_result(), Some(false));

        // No votes - should be None
        proposal.yes_votes = 0;
        proposal.no_votes = 0;
        assert_eq!(proposal.majority_result(), None);
    }
}
