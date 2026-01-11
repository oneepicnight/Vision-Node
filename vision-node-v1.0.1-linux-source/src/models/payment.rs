// src/models/payment.rs
use serde::{Deserialize, Serialize};
use crate::crypto::address::Address;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Payment {
    pub to: Address,
    pub amount: u64,       // if you use U256, switch to String and store amount as decimal string
    #[serde(skip_serializing_if = "Option::is_none")]
    pub memo: Option<String>,
}
