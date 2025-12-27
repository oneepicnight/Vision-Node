use anyhow::Result;
use sled::Db;

/// Thin staking helpers that wrap the vault epoch payout machinery.
/// These provide a small, well-typed surface used by HTTP handlers or tests.
pub fn maybe_pay_epoch(db: &Db, best_height: u64) -> Result<Option<crate::vault_epoch::PayoutSummary>> {
    // Delegate to vault_epoch::pay_epoch_if_due; convert anyhow::Error to anyhow::Result
    crate::vault_epoch::pay_epoch_if_due(db, best_height)
}

pub fn epoch_status(db: &Db, best_height: u64) -> Result<crate::vault_epoch::EpochStatus> {
    crate::vault_epoch::get_epoch_status(db, best_height)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_epoch_payout_simple() {
        // Create temporary sled DB
        let td = tempdir().unwrap();
        let path = td.path().join("db");
        let db = sled::open(path).expect("open sled");

        // Seed tokenomics vault total with 1000 units
        let tok = db.open_tree("tokenomics").unwrap();
        tok.insert(b"vault_total", 1000u128.to_le_bytes().to_vec()).unwrap();

        // Seed owner_weights with single staker "alice" weight = 1000
        let weights = db.open_tree("owner_weights").unwrap();
        weights.insert(b"alice", 1000u128.to_le_bytes().to_vec()).unwrap();

        // Ensure balances tree exists
        let _ = db.open_tree("balances").unwrap();

        // Trigger payout at default epoch boundary (epoch_blocks default = 180)
        let res = maybe_pay_epoch(&db, 180).expect("payout call");
        assert!(res.is_some(), "expected a payout summary");
        let summary = res.unwrap();
        assert!(summary.distributed > 0, "distributed should be > 0");
        assert_eq!(summary.recipients, 1, "one recipient expected");
    }
}
