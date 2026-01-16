// ============================================================================
// GOVERNANCE PROPOSAL SYSTEM (LAND Deed Holder Democracy)
// ============================================================================
//
// Features:
// - Submit proposals (costs 10,000 LAND, non-refundable)
// - 48-hour voting period
// - YES/NO voting (51% majority wins)
// - Only LAND deed holders + founders can vote
// - One wallet = one vote (not stake-weighted)
// - Broadcast notifications to all eligible wallets
// ============================================================================

use once_cell::sync::Lazy;
use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::time::{SystemTime, UNIX_EPOCH};

// Constants
pub const PROPOSAL_FEE_LAND: u128 = 10_000_000_000; // 10,000 LAND (with 6 decimals)
pub const VOTING_PERIOD_SECS: i64 = 48 * 60 * 60; // 48 hours

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum ProposalStatus {
    Open,     // Currently open for voting
    Approved, // Passed with YES >= 51%
    Rejected, // Failed with YES < 51%
    Expired,  // Closed with no participation
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GovernanceProposal {
    pub id: String, // UUID
    pub title: String,
    pub proposal_type: String, // "protocol", "economic", "feature", "community"
    pub body: String,          // Long-form description
    pub technical_impact: Option<String>, // Optional technical details
    pub proposer_wallet: String, // Wallet address of proposer
    pub created_at: i64,       // Unix timestamp (seconds)
    pub closes_at: i64,        // created_at + 48 hours
    pub status: ProposalStatus,
    pub yes_votes: u64, // Count of YES votes (one per wallet)
    pub no_votes: u64,  // Count of NO votes (one per wallet)
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GovernanceVote {
    pub proposal_id: String,
    pub voter_wallet: String,
    pub voted_at: i64,
    pub vote: bool, // true = YES, false = NO
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct WalletNotification {
    pub id: String,
    pub wallet: String,
    pub created_at: i64,
    pub message: String,
    pub kind: String,               // "governance_proposal", etc.
    pub related_id: Option<String>, // proposal_id
    pub read: bool,
}

// Storage
static GOV_PROPOSALS: Lazy<Mutex<BTreeMap<String, GovernanceProposal>>> =
    Lazy::new(|| Mutex::new(BTreeMap::new()));
static GOV_VOTES: Lazy<Mutex<BTreeMap<String, GovernanceVote>>> =
    Lazy::new(|| Mutex::new(BTreeMap::new()));
static WALLET_NOTIFICATIONS: Lazy<Mutex<BTreeMap<String, WalletNotification>>> =
    Lazy::new(|| Mutex::new(BTreeMap::new()));

// Helper: Get current timestamp in seconds
fn now_secs() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64
}

// Create a new proposal
pub fn create_governance_proposal(
    title: String,
    proposal_type: String,
    body: String,
    technical_impact: Option<String>,
    proposer_wallet: String,
) -> Result<GovernanceProposal, String> {
    let now = now_secs();
    let proposal_id = uuid::Uuid::new_v4().to_string();

    let proposal = GovernanceProposal {
        id: proposal_id.clone(),
        title,
        proposal_type,
        body,
        technical_impact,
        proposer_wallet,
        created_at: now,
        closes_at: now + VOTING_PERIOD_SECS,
        status: ProposalStatus::Open,
        yes_votes: 0,
        no_votes: 0,
    };

    GOV_PROPOSALS
        .lock()
        .insert(proposal_id.clone(), proposal.clone());

    tracing::info!(
        "Created governance proposal {} by {}",
        proposal_id,
        proposal.proposer_wallet
    );

    Ok(proposal)
}

// Record a vote
pub fn record_governance_vote(
    proposal_id: String,
    voter_wallet: String,
    vote: bool, // true = YES, false = NO
) -> Result<(), String> {
    // Check proposal exists and is open
    let mut proposals = GOV_PROPOSALS.lock();
    let proposal = proposals
        .get_mut(&proposal_id)
        .ok_or_else(|| "Proposal not found".to_string())?;

    if proposal.status != ProposalStatus::Open {
        return Err("Proposal is not open for voting".to_string());
    }

    let now = now_secs();
    if now >= proposal.closes_at {
        return Err("Voting window has closed".to_string());
    }

    // Check if already voted (one wallet = one vote)
    let vote_key = format!("{}:{}", proposal_id, voter_wallet);
    let mut votes = GOV_VOTES.lock();

    if votes.contains_key(&vote_key) {
        return Err("Already voted on this proposal".to_string());
    }

    // Record vote
    let gov_vote = GovernanceVote {
        proposal_id: proposal_id.clone(),
        voter_wallet: voter_wallet.clone(),
        voted_at: now,
        vote,
    };

    votes.insert(vote_key, gov_vote);

    // Update vote counts
    if vote {
        proposal.yes_votes += 1;
    } else {
        proposal.no_votes += 1;
    }

    drop(votes);
    drop(proposals);

    tracing::info!(
        "Recorded vote on proposal {}: {} voted {}",
        proposal_id,
        voter_wallet,
        if vote { "YES" } else { "NO" }
    );

    Ok(())
}

// Get proposal by ID
pub fn get_governance_proposal(proposal_id: &str) -> Option<GovernanceProposal> {
    GOV_PROPOSALS.lock().get(proposal_id).cloned()
}

// List active proposals (status == Open)
pub fn list_active_proposals() -> Vec<GovernanceProposal> {
    GOV_PROPOSALS
        .lock()
        .values()
        .filter(|p| p.status == ProposalStatus::Open)
        .cloned()
        .collect()
}

// List proposal history (closed proposals)
pub fn list_proposals_history(limit: usize, offset: usize) -> Vec<GovernanceProposal> {
    let mut closed: Vec<GovernanceProposal> = GOV_PROPOSALS
        .lock()
        .values()
        .filter(|p| p.status != ProposalStatus::Open)
        .cloned()
        .collect();

    // Sort by created_at descending (newest first)
    closed.sort_by(|a, b| b.created_at.cmp(&a.created_at));

    closed.into_iter().skip(offset).take(limit).collect()
}

// Check and close expired proposals
pub fn check_and_close_expired_proposals() -> usize {
    let now = now_secs();
    let mut closed_count = 0;

    let mut proposals = GOV_PROPOSALS.lock();

    for proposal in proposals.values_mut() {
        if proposal.status == ProposalStatus::Open && now >= proposal.closes_at {
            // Calculate result
            let total_votes = proposal.yes_votes + proposal.no_votes;

            if total_votes == 0 {
                // No participation
                proposal.status = ProposalStatus::Expired;
            } else {
                // Calculate yes percentage
                let yes_percentage = (proposal.yes_votes * 100) / total_votes;

                if yes_percentage >= 51 {
                    proposal.status = ProposalStatus::Approved;
                } else {
                    proposal.status = ProposalStatus::Rejected;
                }
            }

            closed_count += 1;
            tracing::info!(
                "Closed proposal {}: {:?} (YES: {}, NO: {})",
                proposal.id,
                proposal.status,
                proposal.yes_votes,
                proposal.no_votes
            );
        }
    }

    closed_count
}

// Create notification for wallet
pub fn create_wallet_notification(
    wallet: String,
    message: String,
    kind: String,
    related_id: Option<String>,
) -> String {
    let notification_id = uuid::Uuid::new_v4().to_string();
    let now = now_secs();

    let notification = WalletNotification {
        id: notification_id.clone(),
        wallet: wallet.clone(),
        created_at: now,
        message,
        kind,
        related_id,
        read: false,
    };

    WALLET_NOTIFICATIONS
        .lock()
        .insert(notification_id.clone(), notification);

    notification_id
}

// Broadcast notification to all eligible wallets
pub fn broadcast_proposal_notification(proposal_id: &str, eligible_wallets: Vec<String>) {
    let message =
        "DING DONG BITCH – A new governance proposal is live. Tap to read & vote.".to_string();

    for wallet in eligible_wallets {
        create_wallet_notification(
            wallet,
            message.clone(),
            "governance_proposal".to_string(),
            Some(proposal_id.to_string()),
        );
    }

    tracing::info!(
        "Broadcasted proposal notification for {} to {} wallets",
        proposal_id,
        WALLET_NOTIFICATIONS.lock().len()
    );
}

// List notifications for a wallet
pub fn list_wallet_notifications(wallet: &str) -> Vec<WalletNotification> {
    WALLET_NOTIFICATIONS
        .lock()
        .values()
        .filter(|n| n.wallet == wallet)
        .cloned()
        .collect()
}

// Mark notification as read
pub fn mark_notification_read(notification_id: &str) -> Result<(), String> {
    let mut notifications = WALLET_NOTIFICATIONS.lock();

    if let Some(notification) = notifications.get_mut(notification_id) {
        notification.read = true;
        Ok(())
    } else {
        Err("Notification not found".to_string())
    }
}

// Get governance statistics
pub fn get_governance_stats() -> serde_json::Value {
    let proposals = GOV_PROPOSALS.lock();
    let votes = GOV_VOTES.lock();
    let notifications = WALLET_NOTIFICATIONS.lock();

    let open_count = proposals
        .values()
        .filter(|p| p.status == ProposalStatus::Open)
        .count();
    let approved_count = proposals
        .values()
        .filter(|p| p.status == ProposalStatus::Approved)
        .count();
    let rejected_count = proposals
        .values()
        .filter(|p| p.status == ProposalStatus::Rejected)
        .count();
    let expired_count = proposals
        .values()
        .filter(|p| p.status == ProposalStatus::Expired)
        .count();

    serde_json::json!({
        "proposals": {
            "total": proposals.len(),
            "open": open_count,
            "approved": approved_count,
            "rejected": rejected_count,
            "expired": expired_count,
        },
        "votes": {
            "total": votes.len(),
        },
        "notifications": {
            "total": notifications.len(),
            "unread": notifications.values().filter(|n| !n.read).count(),
        },
        "config": {
            "proposal_fee_land": PROPOSAL_FEE_LAND,
            "voting_period_hours": VOTING_PERIOD_SECS / 3600,
            "pass_threshold_percent": 51,
            "vote_type": "one_wallet_one_vote",
        }
    })
}
