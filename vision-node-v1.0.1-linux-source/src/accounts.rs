use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use std::{fs, path::Path};

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct TokenAccountsCfg {
    pub vault_address: String,
    pub fund_address: String,
    pub founder1_address: String,
    pub founder2_address: String,
    pub vault_pct: u32,
    pub fund_pct: u32,
    pub treasury_pct: u32,
    pub founder1_pct: u32,
    pub founder2_pct: u32,
    // Miners vault deposit addresses for fee settlement (50% - auto-buy hot wallet)
    #[serde(default)]
    pub miners_btc_address: Option<String>,
    #[serde(default)]
    pub miners_bch_address: Option<String>,
    #[serde(default)]
    pub miners_doge_address: Option<String>,
    // Founder 1 crypto addresses (25% of exchange fees)
    #[serde(default)]
    pub founder1_btc_address: Option<String>,
    #[serde(default)]
    pub founder1_bch_address: Option<String>,
    #[serde(default)]
    pub founder1_doge_address: Option<String>,
    // Founder 2 crypto addresses (25% of exchange fees)
    #[serde(default)]
    pub founder2_btc_address: Option<String>,
    #[serde(default)]
    pub founder2_bch_address: Option<String>,
    #[serde(default)]
    pub founder2_doge_address: Option<String>,
}

impl TokenAccountsCfg {
    pub fn validate(&self) -> Result<()> {
        if self.vault_pct + self.fund_pct + self.treasury_pct != 100 {
            return Err(anyhow!(
                "vault_pct + fund_pct + treasury_pct must equal 100"
            ));
        }
        if self.founder1_pct + self.founder2_pct != 100 {
            return Err(anyhow!("founder1_pct + founder2_pct must equal 100"));
        }
        for (label, addr) in [
            ("vault_address", &self.vault_address),
            ("fund_address", &self.fund_address),
            ("founder1_address", &self.founder1_address),
            ("founder2_address", &self.founder2_address),
        ] {
            if addr.len() < 12 {
                return Err(anyhow!("{} looks invalid/empty", label));
            }
        }
        Ok(())
    }
}

pub fn load_token_accounts(path: &str) -> Result<TokenAccountsCfg> {
    let p = Path::new(path);
    let raw = fs::read_to_string(p)?;
    let cfg: TokenAccountsCfg = toml::from_str(&raw)?;
    cfg.validate()?;
    Ok(cfg)
}
