//! Governance HTTP API Routes
//!
//! Endpoints for creating proposals, casting votes, and querying governance state.

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::{IntoResponse, Json},
    routing::{get, post},
    Router,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::{error, info};

use crate::governance::{
    calculate_vote_weight, can_vote, get_all_eligible_voters, GovernanceAction, GovernanceManager,
    GovernanceProposal, GovernanceStatus, GovernanceVote,
};

/// Request to create a proposal
#[derive(Debug, Deserialize)]
pub struct CreateProposalRequest {
    pub proposer: String,
    pub title: String,
    pub body: String,
    pub action: GovernanceAction,
}

/// Request to cast a vote
#[derive(Debug, Deserialize)]
pub struct CastVoteRequest {
    pub voter: String,
    pub support: bool, // true = yes, false = no
}

/// Query parameters for listing proposals
#[derive(Debug, Deserialize)]
pub struct ListProposalsQuery {
    pub status: Option<String>, // "open", "passed", "rejected", "expired"
    #[serde(default = "default_limit")]
    pub limit: usize,
}

fn default_limit() -> usize {
    50
}

/// Query parameters for proposal detail
#[derive(Debug, Deserialize)]
pub struct ProposalDetailQuery {
    pub voter: Option<String>, // Check vote status for this address
}

/// Response for proposal creation
#[derive(Debug, Serialize)]
pub struct CreateProposalResponse {
    pub success: bool,
    pub message: String,
    pub proposal: Option<GovernanceProposal>,
    pub fee_paid: u128,
}

/// Response for vote casting
#[derive(Debug, Serialize)]
pub struct CastVoteResponse {
    pub success: bool,
    pub message: String,
    pub vote: Option<GovernanceVote>,
    pub updated_proposal: Option<GovernanceProposal>,
}

/// Response for proposal list
#[derive(Debug, Serialize)]
pub struct ListProposalsResponse {
    pub proposals: Vec<ProposalSummary>,
    pub total: usize,
}

/// Proposal summary for list view
#[derive(Debug, Serialize)]
pub struct ProposalSummary {
    pub id: u64,
    pub proposer: String,
    pub title: String,
    pub status: String,
    pub created_at: u64,
    pub voting_ends_at: u64,
    pub time_remaining_secs: u64,
    pub yes_votes: u64,
    pub no_votes: u64,
    pub yes_percentage: f64,
    pub action_type: String,
}

/// Proposal detail response
#[derive(Debug, Serialize)]
pub struct ProposalDetailResponse {
    pub proposal: GovernanceProposal,
    pub voter_status: Option<VoterStatus>,
    pub votes: Vec<GovernanceVote>,
}

/// Status of a specific voter for a proposal
#[derive(Debug, Serialize)]
pub struct VoterStatus {
    pub can_vote: bool,
    pub has_voted: bool,
    pub vote: Option<GovernanceVote>,
    pub vote_weight: u64,
}

/// POST /governance/proposals - Create a new proposal
pub async fn create_proposal(
    State(state): State<Arc<GovernanceState>>,
    Json(req): Json<CreateProposalRequest>,
) -> impl IntoResponse {
    let manager = &state.manager;
    // Note: Fee deduction happens outside governance system
    // The proposer must pay 10k LAND fee via normal transaction before calling this endpoint.
    // This design keeps governance stateless and independent from the Chain balances.
    //
    // Implementation options:
    // 1. Frontend creates a transaction sending 10k LAND to vault before proposal creation
    // 2. Add proposal creation as special transaction type that includes fee deduction
    // 3. Extend GovernanceState to include Chain reference for direct balance manipulation
    //
    // For MVP: We trust the caller has handled fee payment externally

    match manager.create_proposal(
        req.proposer.clone(),
        req.title.clone(),
        req.body,
        req.action,
    ) {
        Ok(proposal) => {
            let fee = manager.config().proposal_fee_land;

            info!(
                "[GOVERNANCE API] Proposal #{} created by {}: \"{}\"",
                proposal.id, req.proposer, req.title
            );

            // Notify all eligible voters (DING DONG BITCH!)
            let eligible_voters = get_all_eligible_voters(&manager.db);
            for voter in eligible_voters {
                crate::ws_notifications::WsNotificationManager::notify_governance_proposal_created(
                    &voter,
                    proposal.id,
                    &req.proposer,
                    &req.title,
                    proposal.voting_ends_at,
                );
            }

            (
                StatusCode::OK,
                Json(CreateProposalResponse {
                    success: true,
                    message: format!(
                        "Proposal created successfully. ID: {}. Voting ends in {} hours.",
                        proposal.id,
                        manager.config().voting_duration_secs / 3600
                    ),
                    proposal: Some(proposal),
                    fee_paid: fee,
                }),
            )
        }
        Err(e) => {
            error!("[GOVERNANCE API] Failed to create proposal: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(CreateProposalResponse {
                    success: false,
                    message: format!("Failed to create proposal: {}", e),
                    proposal: None,
                    fee_paid: 0,
                }),
            )
        }
    }
}

/// GET /governance/proposals - List proposals
pub async fn list_proposals(
    Query(params): Query<ListProposalsQuery>,
    State(state): State<Arc<GovernanceState>>,
) -> impl IntoResponse {
    let manager = &state.manager;
    let status_filter = params.status.as_ref().and_then(|s| match s.as_str() {
        "open" => Some(GovernanceStatus::Open),
        "passed" => Some(GovernanceStatus::Passed),
        "rejected" => Some(GovernanceStatus::Rejected),
        "expired" => Some(GovernanceStatus::Expired),
        _ => None,
    });

    match manager.list_proposals(status_filter, params.limit) {
        Ok(proposals) => {
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs();

            let summaries: Vec<ProposalSummary> = proposals
                .iter()
                .map(|p| {
                    let action_type = match &p.action {
                        GovernanceAction::TextOnly => "text_only",
                        GovernanceAction::AddBoardMember { .. } => "add_board_member",
                        GovernanceAction::RemoveBoardMember { .. } => "remove_board_member",
                        GovernanceAction::UpdateDevPayout { .. } => "update_dev_payout",
                    };

                    ProposalSummary {
                        id: p.id,
                        proposer: p.proposer.clone(),
                        title: p.title.clone(),
                        status: p.status.as_str().to_string(),
                        created_at: p.created_at,
                        voting_ends_at: p.voting_ends_at,
                        time_remaining_secs: p.time_remaining(now),
                        yes_votes: p.yes_votes,
                        no_votes: p.no_votes,
                        yes_percentage: p.yes_percentage_bps() as f64 / 100.0,
                        action_type: action_type.to_string(),
                    }
                })
                .collect();

            (
                StatusCode::OK,
                Json(ListProposalsResponse {
                    total: summaries.len(),
                    proposals: summaries,
                }),
            )
        }
        Err(e) => {
            error!("[GOVERNANCE API] Failed to list proposals: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ListProposalsResponse {
                    proposals: Vec::new(),
                    total: 0,
                }),
            )
        }
    }
}

/// GET /governance/proposals/:id - Get proposal detail
pub async fn get_proposal(
    Path(id): Path<u64>,
    Query(params): Query<ProposalDetailQuery>,
    State(state): State<Arc<GovernanceState>>,
) -> impl IntoResponse {
    let manager = &state.manager;
    match manager.get_proposal(id) {
        Ok(Some(proposal)) => {
            // Get all votes for this proposal
            let votes = manager.get_proposal_votes(id).unwrap_or_default();

            // If voter address provided, get their status
            let voter_status = params.voter.as_ref().map(|voter| {
                let has_voted = manager.has_voted(id, voter).unwrap_or(false);
                let vote = if has_voted {
                    manager.get_vote(id, voter).ok().flatten()
                } else {
                    None
                };

                let can_vote_flag = can_vote(&manager.db, voter);
                let vote_weight = calculate_vote_weight(&manager.db, voter);

                VoterStatus {
                    can_vote: can_vote_flag,
                    has_voted,
                    vote,
                    vote_weight,
                }
            });

            (
                StatusCode::OK,
                Json(ProposalDetailResponse {
                    proposal,
                    voter_status,
                    votes,
                }),
            )
        }
        Ok(None) => (
            StatusCode::NOT_FOUND,
            Json(ProposalDetailResponse {
                proposal: GovernanceProposal {
                    id: 0,
                    proposer: String::new(),
                    title: String::new(),
                    body: String::new(),
                    action: GovernanceAction::TextOnly,
                    created_at: 0,
                    voting_ends_at: 0,
                    status: GovernanceStatus::Expired,
                    yes_votes: 0,
                    no_votes: 0,
                    required_majority_bps: 0,
                },
                voter_status: None,
                votes: Vec::new(),
            }),
        ),
        Err(e) => {
            error!("[GOVERNANCE API] Failed to get proposal {}: {}", id, e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ProposalDetailResponse {
                    proposal: GovernanceProposal {
                        id: 0,
                        proposer: String::new(),
                        title: String::new(),
                        body: String::new(),
                        action: GovernanceAction::TextOnly,
                        created_at: 0,
                        voting_ends_at: 0,
                        status: GovernanceStatus::Expired,
                        yes_votes: 0,
                        no_votes: 0,
                        required_majority_bps: 0,
                    },
                    voter_status: None,
                    votes: Vec::new(),
                }),
            )
        }
    }
}

/// POST /governance/proposals/:id/vote - Cast a vote
pub async fn cast_vote(
    Path(id): Path<u64>,
    State(state): State<Arc<GovernanceState>>,
    Json(req): Json<CastVoteRequest>,
) -> impl IntoResponse {
    let manager = &state.manager;

    // Verify voter eligibility (founder or deed holder)
    if !can_vote(&manager.db, &req.voter) {
        return (
            StatusCode::FORBIDDEN,
            Json(CastVoteResponse {
                success: false,
                message: "Voter is not eligible. Must be founder or land deed holder.".to_string(),
                vote: None,
                updated_proposal: None,
            }),
        );
    }

    let weight = calculate_vote_weight(&manager.db, &req.voter);

    match manager.cast_vote(id, req.voter.clone(), req.support, weight) {
        Ok(vote) => {
            // Get updated proposal
            let updated_proposal = manager.get_proposal(id).ok().flatten();

            info!(
                "[GOVERNANCE API] Vote cast on proposal #{}: {} votes {} (weight: {})",
                id,
                req.voter,
                if req.support { "YES" } else { "NO" },
                weight
            );

            (
                StatusCode::OK,
                Json(CastVoteResponse {
                    success: true,
                    message: format!(
                        "Vote cast successfully: {} voted {}",
                        req.voter,
                        if req.support { "YES" } else { "NO" }
                    ),
                    vote: Some(vote),
                    updated_proposal,
                }),
            )
        }
        Err(e) => {
            error!("[GOVERNANCE API] Failed to cast vote: {}", e);
            (
                StatusCode::BAD_REQUEST,
                Json(CastVoteResponse {
                    success: false,
                    message: format!("Failed to cast vote: {}", e),
                    vote: None,
                    updated_proposal: None,
                }),
            )
        }
    }
}

/// State for governance routes
#[derive(Clone)]
pub struct GovernanceState {
    pub manager: Arc<GovernanceManager>,
}

/// Create the governance router
pub fn governance_router(manager: Arc<GovernanceManager>) -> Router {
    let state = GovernanceState { manager };

    Router::new()
        .route(
            "/governance/proposals",
            post(create_proposal).get(list_proposals),
        )
        .route("/governance/proposals/:id", get(get_proposal))
        .route("/governance/proposals/:id/vote", post(cast_vote))
        .with_state(Arc::new(state))
}
