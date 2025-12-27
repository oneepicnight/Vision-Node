//! Stub for governance_democracy module when staging is disabled

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DemocracyVote {
    pub voter: String,
    pub proposal_id: u64,
    pub vote: bool,
}

impl Default for DemocracyVote {
    fn default() -> Self {
        Self {
            voter: String::new(),
            proposal_id: 0,
            vote: false,
        }
    }
}

pub fn cast_vote(_vote: &DemocracyVote) -> Result<(), String> {
    Ok(())
}

pub fn tally_votes(_proposal_id: u64) -> Result<(u64, u64), String> {
    Ok((0, 0))
}
