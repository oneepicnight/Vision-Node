// =================== Legendary / Immortal Wallet Transfer System ===================
// Allows special status wallets to be sold/transferred with full security guarantees

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use uuid::Uuid;

/// Account flags for legendary/immortal wallet status
#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct AccountFlags {
    /// Wallet has legendary status (earned through achievements/history)
    pub legendary: bool,
    /// Wallet is tied to an immortal node (special node history)
    pub immortal_node: bool,
    /// Whether this wallet's status can currently be transferred/sold
    pub transferable: bool,
}

impl AccountFlags {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_legendary(mut self) -> Self {
        self.legendary = true;
        self
    }

    pub fn with_immortal_node(mut self) -> Self {
        self.immortal_node = true;
        self
    }

    pub fn with_transferable(mut self) -> Self {
        self.transferable = true;
        self
    }

    /// Check if wallet has any special status
    pub fn has_special_status(&self) -> bool {
        self.legendary || self.immortal_node
    }

    /// Serialize to bytes for state persistence
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = vec![0u8; 3];
        bytes[0] = if self.legendary { 1 } else { 0 };
        bytes[1] = if self.immortal_node { 1 } else { 0 };
        bytes[2] = if self.transferable { 1 } else { 0 };
        bytes
    }

    /// Deserialize from bytes
    pub fn from_bytes(bytes: &[u8]) -> Self {
        if bytes.len() < 3 {
            return Self::default();
        }
        Self {
            legendary: bytes[0] != 0,
            immortal_node: bytes[1] != 0,
            transferable: bytes[2] != 0,
        }
    }
}

/// Transaction payload for transferring wallet status
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TransferWalletStatusTx {
    /// Address of the current legendary/immortal wallet (seller)
    pub from: String,
    /// Address of the new owner's wallet (must be different from `from`)
    pub to: String,
    /// Move all balance as part of the transfer
    pub move_balance: bool,
    /// Transfer legendary status flag
    pub move_legendary: bool,
    /// Transfer immortal node status flag
    pub move_immortal_node: bool,
}

impl TransferWalletStatusTx {
    /// Encode transaction to bytes for signing/hashing
    pub fn encode(&self) -> Vec<u8> {
        serde_json::to_vec(self).unwrap_or_default()
    }
}

/// Marketplace offer for wallet status transfer
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct WalletOffer {
    pub id: Uuid,
    pub from: String,
    pub move_legendary: bool,
    pub move_immortal_node: bool,
    pub move_balance: bool,
    pub price_land: u128,
    pub status: OfferStatus,
    pub created_at: u64,
    /// Pre-signed transaction (optional, for instant completion)
    pub presigned_tx: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum OfferStatus {
    Open,
    Completed,
    Cancelled,
}

impl WalletOffer {
    pub fn new(
        from: String,
        move_legendary: bool,
        move_immortal_node: bool,
        move_balance: bool,
        price_land: u128,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            from,
            move_legendary,
            move_immortal_node,
            move_balance,
            price_land,
            status: OfferStatus::Open,
            created_at: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            presigned_tx: None,
        }
    }

    pub fn is_open(&self) -> bool {
        self.status == OfferStatus::Open
    }

    pub fn complete(&mut self) {
        self.status = OfferStatus::Completed;
    }

    pub fn cancel(&mut self) {
        self.status = OfferStatus::Cancelled;
    }
}

/// Transaction validation errors
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum WalletStatusError {
    SameAddress,
    NotLegendary,
    NotImmortalNode,
    NotTransferable,
    InsufficientBalance,
    BalanceOverflow,
    OfferNotFound,
    OfferNotOpen,
    InvalidAddress,
    FeatureDisabled,
}

impl std::fmt::Display for WalletStatusError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::SameAddress => write!(f, "from and to addresses must be different"),
            Self::NotLegendary => write!(f, "wallet does not have legendary status"),
            Self::NotImmortalNode => write!(f, "wallet does not have immortal node status"),
            Self::NotTransferable => write!(f, "wallet is not marked as transferable"),
            Self::InsufficientBalance => write!(f, "insufficient balance for transfer"),
            Self::BalanceOverflow => write!(f, "balance overflow during transfer"),
            Self::OfferNotFound => write!(f, "offer not found"),
            Self::OfferNotOpen => write!(f, "offer is not open"),
            Self::InvalidAddress => write!(f, "invalid address format"),
            Self::FeatureDisabled => write!(f, "legendary transfer feature is disabled"),
        }
    }
}

impl std::error::Error for WalletStatusError {}

/// Validation logic for wallet status transfer
pub fn validate_transfer_wallet_status(
    tx: &TransferWalletStatusTx,
    from_balance: u128,
    from_flags: &AccountFlags,
    feature_enabled: bool,
) -> Result<(), WalletStatusError> {
    // Feature gate
    if !feature_enabled {
        return Err(WalletStatusError::FeatureDisabled);
    }

    // Must be different wallets
    if tx.from == tx.to {
        return Err(WalletStatusError::SameAddress);
    }

    // Must actually have something to transfer
    if tx.move_legendary && !from_flags.legendary {
        return Err(WalletStatusError::NotLegendary);
    }
    if tx.move_immortal_node && !from_flags.immortal_node {
        return Err(WalletStatusError::NotImmortalNode);
    }

    // Wallet must be marked transferable
    if !from_flags.transferable {
        return Err(WalletStatusError::NotTransferable);
    }

    // If moving balance, ensure sufficient funds
    if tx.move_balance && from_balance == 0 {
        // Allow zero balance transfers (status-only)
        // This is not an error, just informational
    }

    Ok(())
}

/// Apply wallet status transfer to state
pub fn apply_transfer_wallet_status(
    tx: &TransferWalletStatusTx,
    from_balance: &mut u128,
    from_flags: &mut AccountFlags,
    to_balance: &mut u128,
    to_flags: &mut AccountFlags,
) -> Result<(), WalletStatusError> {
    // Move flags
    if tx.move_legendary {
        from_flags.legendary = false;
        to_flags.legendary = true;
    }
    if tx.move_immortal_node {
        from_flags.immortal_node = false;
        to_flags.immortal_node = true;
    }

    // CRITICAL: Once transferred, old wallet loses transferable flag
    // This prevents the seller from transferring again
    from_flags.transferable = false;

    // Move balance if requested
    if tx.move_balance {
        let amount = *from_balance;
        *from_balance = 0;
        *to_balance = to_balance
            .checked_add(amount)
            .ok_or(WalletStatusError::BalanceOverflow)?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_account_flags_serialization() {
        let flags = AccountFlags::new()
            .with_legendary()
            .with_transferable();
        
        let bytes = flags.to_bytes();
        let restored = AccountFlags::from_bytes(&bytes);
        
        assert_eq!(flags, restored);
        assert!(restored.legendary);
        assert!(!restored.immortal_node);
        assert!(restored.transferable);
    }

    #[test]
    fn test_validation_same_address() {
        let tx = TransferWalletStatusTx {
            from: "addr1".to_string(),
            to: "addr1".to_string(),
            move_balance: false,
            move_legendary: true,
            move_immortal_node: false,
        };
        
        let flags = AccountFlags::new().with_legendary().with_transferable();
        let result = validate_transfer_wallet_status(&tx, 100, &flags, true);
        
        assert!(matches!(result, Err(WalletStatusError::SameAddress)));
    }

    #[test]
    fn test_validation_not_transferable() {
        let tx = TransferWalletStatusTx {
            from: "addr1".to_string(),
            to: "addr2".to_string(),
            move_balance: false,
            move_legendary: true,
            move_immortal_node: false,
        };
        
        let flags = AccountFlags::new().with_legendary(); // NOT transferable
        let result = validate_transfer_wallet_status(&tx, 100, &flags, true);
        
        assert!(matches!(result, Err(WalletStatusError::NotTransferable)));
    }

    #[test]
    fn test_apply_transfer() {
        let tx = TransferWalletStatusTx {
            from: "addr1".to_string(),
            to: "addr2".to_string(),
            move_balance: true,
            move_legendary: true,
            move_immortal_node: true,
        };
        
        let mut from_balance = 1000u128;
        let mut from_flags = AccountFlags::new()
            .with_legendary()
            .with_immortal_node()
            .with_transferable();
        
        let mut to_balance = 500u128;
        let mut to_flags = AccountFlags::new();
        
        let result = apply_transfer_wallet_status(
            &tx,
            &mut from_balance,
            &mut from_flags,
            &mut to_balance,
            &mut to_flags,
        );
        
        assert!(result.is_ok());
        assert_eq!(from_balance, 0);
        assert_eq!(to_balance, 1500);
        assert!(!from_flags.legendary);
        assert!(!from_flags.immortal_node);
        assert!(!from_flags.transferable); // CRITICAL: can't transfer again
        assert!(to_flags.legendary);
        assert!(to_flags.immortal_node);
    }
}
