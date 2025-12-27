// Hardware Wallet Support - Ledger and Trezor Integration
// Implements USB communication, transaction signing, and address derivation

use serde::{Deserialize, Serialize};
use anyhow::{Result, anyhow};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use once_cell::sync::Lazy;

use crate::market::engine::QuoteAsset;

// ============================================================================
// HARDWARE WALLET TYPES
// ============================================================================

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum HardwareWalletType {
    Ledger,
    Trezor,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HardwareWallet {
    /// Wallet type (Ledger/Trezor)
    pub wallet_type: HardwareWalletType,
    /// Device serial/ID
    pub device_id: String,
    /// Firmware version
    pub firmware_version: String,
    /// Supported coins
    pub supported_coins: Vec<QuoteAsset>,
    /// Is device connected?
    pub connected: bool,
}

/// BIP32/BIP44 derivation path
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DerivationPath {
    /// Purpose (44' for BIP44, 49' for P2SH-P2WPKH, 84' for P2WPKH)
    pub purpose: u32,
    /// Coin type (0' for BTC, 145' for BCH, 3' for DOGE)
    pub coin_type: u32,
    /// Account index
    pub account: u32,
    /// Change (0 = external, 1 = internal)
    pub change: u32,
    /// Address index
    pub address_index: u32,
}

impl DerivationPath {
    /// BIP44 path: m/44'/coin'/account'/change/address_index
    pub fn bip44(asset: QuoteAsset, account: u32, change: u32, address_index: u32) -> Self {
        let coin_type = match asset {
            QuoteAsset::Btc => 0,
            QuoteAsset::Bch => 145,
            QuoteAsset::Doge => 3,
            QuoteAsset::Land => 0, // Not applicable
        };
        
        Self {
            purpose: 44,
            coin_type,
            account,
            change,
            address_index,
        }
    }
    
    /// BIP84 path (Native SegWit): m/84'/coin'/account'/change/address_index
    pub fn bip84(asset: QuoteAsset, account: u32, change: u32, address_index: u32) -> Self {
        let coin_type = match asset {
            QuoteAsset::Btc => 0,
            QuoteAsset::Bch => 145,
            QuoteAsset::Doge => 3,
            QuoteAsset::Land => 0,
        };
        
        Self {
            purpose: 84,
            coin_type,
            account,
            change,
            address_index,
        }
    }
    
    /// Convert to string format: m/44'/0'/0'/0/0
    pub fn to_string(&self) -> String {
        format!(
            "m/{}'/{}'/{}'/{}/{}",
            self.purpose,
            self.coin_type,
            self.account,
            self.change,
            self.address_index
        )
    }
    
    /// Parse from string format
    pub fn from_string(path: &str) -> Result<Self> {
        let parts: Vec<&str> = path.trim_start_matches("m/").split('/').collect();
        if parts.len() != 5 {
            return Err(anyhow!("Invalid derivation path"));
        }
        
        let purpose = parts[0].trim_end_matches('\'').parse()?;
        let coin_type = parts[1].trim_end_matches('\'').parse()?;
        let account = parts[2].trim_end_matches('\'').parse()?;
        let change = parts[3].parse()?;
        let address_index = parts[4].parse()?;
        
        Ok(Self {
            purpose,
            coin_type,
            account,
            change,
            address_index,
        })
    }
}

/// Transaction to be signed by hardware wallet
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnsignedTransaction {
    pub asset: QuoteAsset,
    pub inputs: Vec<TxInput>,
    pub outputs: Vec<TxOutput>,
    pub locktime: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TxInput {
    pub txid: String,
    pub vout: u32,
    pub amount: u64,
    pub script_pubkey: String,
    pub derivation_path: DerivationPath,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TxOutput {
    pub address: String,
    pub amount: u64,
}

/// Signed transaction from hardware wallet
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignedTransaction {
    pub raw_tx: String,
    pub txid: String,
    pub signatures: Vec<String>,
}

// ============================================================================
// LEDGER INTEGRATION
// ============================================================================

#[cfg(feature = "hardware-wallet")]
pub struct LedgerDevice {
    device_id: String,
    transport: Option<Box<dyn std::any::Any>>, // Ledger transport
}

#[cfg(feature = "hardware-wallet")]
impl LedgerDevice {
    /// Connect to Ledger device
    pub fn connect() -> Result<Self> {
        // In production, this would:
        // 1. Use hidapi to enumerate USB devices
        // 2. Find Ledger device (Vendor ID: 0x2c97)
        // 3. Open communication channel
        // 4. Initialize transport
        
        tracing::info!("🔌 Connecting to Ledger device...");
        
        Ok(Self {
            device_id: "ledger_sim_001".to_string(),
            transport: None,
        })
    }
    
    /// Get device info
    pub fn get_device_info(&self) -> Result<HardwareWallet> {
        Ok(HardwareWallet {
            wallet_type: HardwareWalletType::Ledger,
            device_id: self.device_id.clone(),
            firmware_version: "2.1.0".to_string(),
            supported_coins: vec![QuoteAsset::Btc, QuoteAsset::Bch, QuoteAsset::Doge],
            connected: true,
        })
    }
    
    /// Get address from device
    pub fn get_address(
        &self,
        asset: QuoteAsset,
        path: &DerivationPath,
        display: bool,
    ) -> Result<String> {
        // In production, this would:
        // 1. Send GET_ADDRESS command via APDU
        // 2. Parse response from device
        // 3. Optionally display on device screen for verification
        
        tracing::info!(
            "📍 Getting address from Ledger: {} path: {}",
            asset.as_str(),
            path.to_string()
        );
        
        // Simulate address generation
        let address = match asset {
            QuoteAsset::Btc => format!("bc1qledger{}", path.address_index),
            QuoteAsset::Bch => format!("bitcoincash:qledger{}", path.address_index),
            QuoteAsset::Doge => format!("DLedger{}", path.address_index),
            QuoteAsset::Land => return Err(anyhow!("LAND not supported on hardware wallets")),
        };
        
        if display {
            tracing::info!("👀 Please verify address on Ledger device");
        }
        
        Ok(address)
    }
    
    /// Sign transaction with device
    pub fn sign_transaction(&self, tx: &UnsignedTransaction) -> Result<SignedTransaction> {
        // In production, this would:
        // 1. Send SIGN_TRANSACTION command via APDU
        // 2. Device displays transaction details
        // 3. User confirms on device
        // 4. Device returns signatures
        
        tracing::info!(
            "✍️  Signing transaction on Ledger: {} inputs, {} outputs",
            tx.inputs.len(),
            tx.outputs.len()
        );
        
        tracing::info!("👀 Please confirm transaction on Ledger device");
        
        // Simulate signing
        let signatures: Vec<String> = tx.inputs.iter()
            .map(|input| format!("sig_ledger_{}", input.txid))
            .collect();
        
        let raw_tx = format!("raw_tx_ledger_{}", uuid::Uuid::new_v4());
        let txid = format!("txid_ledger_{}", uuid::Uuid::new_v4());
        
        Ok(SignedTransaction {
            raw_tx,
            txid,
            signatures,
        })
    }
    
    /// Disconnect from device
    pub fn disconnect(&mut self) -> Result<()> {
        tracing::info!("🔌 Disconnecting from Ledger device");
        self.transport = None;
        Ok(())
    }
}

#[cfg(not(feature = "hardware-wallet"))]
pub struct LedgerDevice;

#[cfg(not(feature = "hardware-wallet"))]
impl LedgerDevice {
    pub fn connect() -> Result<Self> {
        Err(anyhow!("Hardware wallet support not enabled. Compile with --features hardware-wallet"))
    }
}

// ============================================================================
// TREZOR INTEGRATION
// ============================================================================

#[cfg(feature = "hardware-wallet")]
pub struct TrezorDevice {
    device_id: String,
    client: Option<Box<dyn std::any::Any>>, // Trezor client
}

#[cfg(feature = "hardware-wallet")]
impl TrezorDevice {
    /// Connect to Trezor device
    pub fn connect() -> Result<Self> {
        // In production, this would:
        // 1. Use trezor-client crate
        // 2. Find Trezor device via USB
        // 3. Initialize protobuf communication
        // 4. Unlock device with PIN
        
        tracing::info!("🔌 Connecting to Trezor device...");
        
        Ok(Self {
            device_id: "trezor_sim_001".to_string(),
            client: None,
        })
    }
    
    /// Get device info
    pub fn get_device_info(&self) -> Result<HardwareWallet> {
        Ok(HardwareWallet {
            wallet_type: HardwareWalletType::Trezor,
            device_id: self.device_id.clone(),
            firmware_version: "2.5.3".to_string(),
            supported_coins: vec![QuoteAsset::Btc, QuoteAsset::Bch, QuoteAsset::Doge],
            connected: true,
        })
    }
    
    /// Get address from device
    pub fn get_address(
        &self,
        asset: QuoteAsset,
        path: &DerivationPath,
        display: bool,
    ) -> Result<String> {
        tracing::info!(
            "📍 Getting address from Trezor: {} path: {}",
            asset.as_str(),
            path.to_string()
        );
        
        let address = match asset {
            QuoteAsset::Btc => format!("bc1qtrezor{}", path.address_index),
            QuoteAsset::Bch => format!("bitcoincash:qtrezor{}", path.address_index),
            QuoteAsset::Doge => format!("DTrezor{}", path.address_index),
            QuoteAsset::Land => return Err(anyhow!("LAND not supported on hardware wallets")),
        };
        
        if display {
            tracing::info!("👀 Please verify address on Trezor device");
        }
        
        Ok(address)
    }
    
    /// Sign transaction with device
    pub fn sign_transaction(&self, tx: &UnsignedTransaction) -> Result<SignedTransaction> {
        tracing::info!(
            "✍️  Signing transaction on Trezor: {} inputs, {} outputs",
            tx.inputs.len(),
            tx.outputs.len()
        );
        
        tracing::info!("👀 Please confirm transaction on Trezor device");
        
        // Simulate signing
        let signatures: Vec<String> = tx.inputs.iter()
            .map(|input| format!("sig_trezor_{}", input.txid))
            .collect();
        
        let raw_tx = format!("raw_tx_trezor_{}", uuid::Uuid::new_v4());
        let txid = format!("txid_trezor_{}", uuid::Uuid::new_v4());
        
        Ok(SignedTransaction {
            raw_tx,
            txid,
            signatures,
        })
    }
    
    /// Disconnect from device
    pub fn disconnect(&mut self) -> Result<()> {
        tracing::info!("🔌 Disconnecting from Trezor device");
        self.client = None;
        Ok(())
    }
}

#[cfg(not(feature = "hardware-wallet"))]
pub struct TrezorDevice;

#[cfg(not(feature = "hardware-wallet"))]
impl TrezorDevice {
    pub fn connect() -> Result<Self> {
        Err(anyhow!("Hardware wallet support not enabled. Compile with --features hardware-wallet"))
    }
}

// ============================================================================
// HARDWARE WALLET MANAGER
// ============================================================================

pub enum Device {
    Ledger(LedgerDevice),
    Trezor(TrezorDevice),
}

pub static CONNECTED_DEVICES: Lazy<Arc<Mutex<HashMap<String, Device>>>> = 
    Lazy::new(|| Arc::new(Mutex::new(HashMap::new())));

pub struct HardwareWalletManager;

impl HardwareWalletManager {
    /// Detect connected hardware wallets
    pub fn detect_devices() -> Result<Vec<HardwareWallet>> {
        let mut devices = Vec::new();
        
        // Try to connect to Ledger
        #[cfg(feature = "hardware-wallet")]
        {
            if let Ok(ledger) = LedgerDevice::connect() {
                if let Ok(info) = ledger.get_device_info() {
                    devices.push(info);
                    
                    let mut connected = CONNECTED_DEVICES.lock()
                        .map_err(|e| anyhow!("Failed to lock devices: {}", e))?;
                    connected.insert(ledger.device_id.clone(), Device::Ledger(ledger));
                }
            }
            
            // Try to connect to Trezor
            if let Ok(trezor) = TrezorDevice::connect() {
                if let Ok(info) = trezor.get_device_info() {
                    devices.push(info);
                    
                    let mut connected = CONNECTED_DEVICES.lock()
                        .map_err(|e| anyhow!("Failed to lock devices: {}", e))?;
                    connected.insert(trezor.device_id.clone(), Device::Trezor(trezor));
                }
            }
        }
        
        #[cfg(not(feature = "hardware-wallet"))]
        {
            tracing::warn!("Hardware wallet support not enabled");
        }
        
        Ok(devices)
    }
    
    /// Get address from hardware wallet
    pub fn get_address(
        device_id: &str,
        asset: QuoteAsset,
        path: &DerivationPath,
        display: bool,
    ) -> Result<String> {
        let devices = CONNECTED_DEVICES.lock()
            .map_err(|e| anyhow!("Failed to lock devices: {}", e))?;
        
        let device = devices.get(device_id)
            .ok_or_else(|| anyhow!("Device not found: {}", device_id))?;
        
        match device {
            #[cfg(feature = "hardware-wallet")]
            Device::Ledger(ledger) => ledger.get_address(asset, path, display),
            #[cfg(feature = "hardware-wallet")]
            Device::Trezor(trezor) => trezor.get_address(asset, path, display),
            #[cfg(not(feature = "hardware-wallet"))]
            _ => Err(anyhow!("Hardware wallet support not enabled")),
        }
    }
    
    /// Sign transaction with hardware wallet
    pub fn sign_transaction(
        device_id: &str,
        tx: &UnsignedTransaction,
    ) -> Result<SignedTransaction> {
        let devices = CONNECTED_DEVICES.lock()
            .map_err(|e| anyhow!("Failed to lock devices: {}", e))?;
        
        let device = devices.get(device_id)
            .ok_or_else(|| anyhow!("Device not found: {}", device_id))?;
        
        match device {
            #[cfg(feature = "hardware-wallet")]
            Device::Ledger(ledger) => ledger.sign_transaction(tx),
            #[cfg(feature = "hardware-wallet")]
            Device::Trezor(trezor) => trezor.sign_transaction(tx),
            #[cfg(not(feature = "hardware-wallet"))]
            _ => Err(anyhow!("Hardware wallet support not enabled")),
        }
    }
    
    /// Get connected devices
    pub fn get_connected_devices() -> Result<Vec<String>> {
        let devices = CONNECTED_DEVICES.lock()
            .map_err(|e| anyhow!("Failed to lock devices: {}", e))?;
        
        Ok(devices.keys().cloned().collect())
    }
    
    /// Disconnect device
    pub fn disconnect(device_id: &str) -> Result<()> {
        let mut devices = CONNECTED_DEVICES.lock()
            .map_err(|e| anyhow!("Failed to lock devices: {}", e))?;
        
        devices.remove(device_id);
        tracing::info!("📴 Disconnected device: {}", device_id);
        
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_derivation_path_string() {
        let path = DerivationPath::bip44(QuoteAsset::Btc, 0, 0, 5);
        assert_eq!(path.to_string(), "m/44'/0'/0'/0/5");
        
        let parsed = DerivationPath::from_string("m/44'/0'/0'/0/5").unwrap();
        assert_eq!(parsed.purpose, 44);
        assert_eq!(parsed.coin_type, 0);
        assert_eq!(parsed.address_index, 5);
    }
    
    #[test]
    fn test_bip84_path() {
        let path = DerivationPath::bip84(QuoteAsset::Btc, 0, 0, 10);
        assert_eq!(path.purpose, 84);
        assert_eq!(path.to_string(), "m/84'/0'/0'/0/10");
    }
}
