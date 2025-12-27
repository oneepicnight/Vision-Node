// Advanced UTXO Selection Algorithms
// Implements Branch and Bound, and various coin selection strategies

use std::collections::HashMap;
use serde::{Deserialize, Serialize};
use anyhow::{Result, anyhow};

use crate::utxo_manager::Utxo;

// ============================================================================
// COIN SELECTION STRATEGIES
// ============================================================================

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum CoinSelectionStrategy {
    /// Branch and Bound - optimal selection with waste minimization
    BranchAndBound,
    /// Select largest UTXOs first
    LargestFirst,
    /// Select smallest UTXOs first (good for consolidation)
    SmallestFirst,
    /// First-In-First-Out - spend oldest coins first
    Fifo,
    /// Last-In-First-Out - spend newest coins first
    Lifo,
    /// Random selection (privacy-focused)
    Random,
    /// Single Random Draw - maximize privacy
    SingleRandomDraw,
}

/// Result of coin selection
#[derive(Debug, Clone)]
pub struct CoinSelection {
    /// Selected UTXOs
    pub utxos: Vec<Utxo>,
    /// Total input amount (satoshis)
    pub total_input: u64,
    /// Change amount (satoshis)
    pub change: u64,
    /// Estimated fee (satoshis)
    pub fee: u64,
    /// Waste metric (lower is better)
    pub waste: i64,
    /// Strategy used
    pub strategy: CoinSelectionStrategy,
}

impl CoinSelection {
    pub fn new(
        utxos: Vec<Utxo>,
        target: u64,
        fee: u64,
        strategy: CoinSelectionStrategy,
    ) -> Self {
        let total_input: u64 = utxos.iter().map(|u| u.amount_satoshis()).sum();
        let change = total_input.saturating_sub(target + fee);
        let waste = Self::calculate_waste(total_input, target, fee, change);
        
        Self {
            utxos,
            total_input,
            change,
            fee,
            waste,
            strategy,
        }
    }
    
    fn calculate_waste(input: u64, target: u64, fee: u64, change: u64) -> i64 {
        // Waste = excess + cost of change output
        let excess = input as i64 - (target + fee) as i64;
        let change_cost = if change > 0 { 
            // Cost to create and spend change output later
            34 * 2 // ~34 bytes for output, will cost similar to spend
        } else { 
            0 
        };
        
        excess.abs() + change_cost
    }
}

// ============================================================================
// BRANCH AND BOUND ALGORITHM
// ============================================================================

pub struct BranchAndBound {
    /// Maximum number of iterations
    max_iterations: usize,
    /// Cost of change output (in satoshis)
    cost_of_change: u64,
}

impl BranchAndBound {
    pub fn new() -> Self {
        Self {
            max_iterations: 100_000,
            cost_of_change: 68, // 34 bytes output * ~2 sat/byte
        }
    }
    
    /// Select UTXOs using Branch and Bound algorithm
    /// Finds the combination with minimal waste
    pub fn select(
        &self,
        mut utxos: Vec<Utxo>,
        target: u64,
        fee_rate: f64, // sat/vbyte
    ) -> Result<CoinSelection> {
        if utxos.is_empty() {
            return Err(anyhow!("No UTXOs available"));
        }
        
        // Sort by effective value (descending)
        utxos.sort_by(|a, b| {
            let a_val = self.effective_value(a, fee_rate);
            let b_val = self.effective_value(b, fee_rate);
            b_val.partial_cmp(&a_val).unwrap()
        });
        
        // Calculate target with fee
        let base_fee = self.estimate_base_fee(fee_rate);
        let target_with_fee = target + base_fee;
        
        // Try Branch and Bound
        if let Some(selection) = self.branch_and_bound(&utxos, target_with_fee, fee_rate) {
            let fee = self.calculate_fee(&selection, fee_rate);
            return Ok(CoinSelection::new(
                selection,
                target,
                fee,
                CoinSelectionStrategy::BranchAndBound,
            ));
        }
        
        // Fallback to largest-first if BnB fails
        self.fallback_selection(utxos, target, fee_rate)
    }
    
    fn branch_and_bound(
        &self,
        utxos: &[Utxo],
        target: u64,
        fee_rate: f64,
    ) -> Option<Vec<Utxo>> {
        let mut best_selection: Option<Vec<Utxo>> = None;
        let mut best_waste = i64::MAX;
        
        // Calculate effective values
        let effective_values: Vec<i64> = utxos.iter()
            .map(|u| self.effective_value(u, fee_rate))
            .collect();
        
        let mut current_selection = Vec::new();
        let mut current_value: i64 = 0;
        let mut iteration = 0;
        
        self.bnb_recursive(
            utxos,
            &effective_values,
            target as i64,
            0,
            &mut current_selection,
            current_value,
            &mut best_selection,
            &mut best_waste,
            &mut iteration,
        );
        
        best_selection
    }
    
    #[allow(clippy::too_many_arguments)]
    fn bnb_recursive(
        &self,
        utxos: &[Utxo],
        effective_values: &[i64],
        target: i64,
        index: usize,
        current_selection: &mut Vec<Utxo>,
        current_value: i64,
        best_selection: &mut Option<Vec<Utxo>>,
        best_waste: &mut i64,
        iteration: &mut usize,
    ) -> bool {
        *iteration += 1;
        if *iteration > self.max_iterations {
            return false;
        }
        
        // Check if we found a better solution
        if current_value >= target {
            let waste = current_value - target;
            
            // Perfect match or better than previous
            if waste < *best_waste {
                *best_waste = waste;
                *best_selection = Some(current_selection.clone());
                
                // Early exit if perfect match
                if waste == 0 {
                    return true;
                }
            }
            
            return false;
        }
        
        // Reached end without finding solution
        if index >= utxos.len() {
            return false;
        }
        
        // Pruning: check if remaining UTXOs can reach target
        let remaining_value: i64 = effective_values[index..].iter().sum();
        if current_value + remaining_value < target {
            return false;
        }
        
        // Try including current UTXO
        current_selection.push(utxos[index].clone());
        let new_value = current_value + effective_values[index];
        
        if self.bnb_recursive(
            utxos,
            effective_values,
            target,
            index + 1,
            current_selection,
            new_value,
            best_selection,
            best_waste,
            iteration,
        ) {
            return true;
        }
        
        // Backtrack: try excluding current UTXO
        current_selection.pop();
        
        self.bnb_recursive(
            utxos,
            effective_values,
            target,
            index + 1,
            current_selection,
            current_value,
            best_selection,
            best_waste,
            iteration,
        )
    }
    
    fn effective_value(&self, utxo: &Utxo, fee_rate: f64) -> i64 {
        let value = utxo.amount_satoshis() as i64;
        let input_cost = self.estimate_input_size() as i64 * fee_rate as i64;
        value - input_cost
    }
    
    fn estimate_input_size(&self) -> u64 {
        // P2WPKH: ~68 vbytes, P2PKH: ~148 vbytes
        // Use P2WPKH as default
        68
    }
    
    fn estimate_base_fee(&self, fee_rate: f64) -> u64 {
        // Base transaction size: 10 bytes (version, locktime, etc.)
        // + outputs (34 bytes each)
        let base_size = 10 + 34 * 2; // Assume 1 recipient + 1 change
        (base_size as f64 * fee_rate) as u64
    }
    
    fn calculate_fee(&self, utxos: &[Utxo], fee_rate: f64) -> u64 {
        let input_size = utxos.len() as u64 * self.estimate_input_size();
        let output_size = 34 * 2; // 1 recipient + 1 change
        let total_size = 10 + input_size + output_size;
        (total_size as f64 * fee_rate) as u64
    }
    
    fn fallback_selection(
        &self,
        utxos: Vec<Utxo>,
        target: u64,
        fee_rate: f64,
    ) -> Result<CoinSelection> {
        LargestFirst::new().select(utxos, target, fee_rate)
    }
}

// ============================================================================
// LARGEST FIRST STRATEGY
// ============================================================================

pub struct LargestFirst;

impl LargestFirst {
    pub fn new() -> Self {
        Self
    }
    
    pub fn select(
        &self,
        mut utxos: Vec<Utxo>,
        target: u64,
        fee_rate: f64,
    ) -> Result<CoinSelection> {
        if utxos.is_empty() {
            return Err(anyhow!("No UTXOs available"));
        }
        
        // Sort by amount descending
        utxos.sort_by(|a, b| {
            b.amount_satoshis().cmp(&a.amount_satoshis())
        });
        
        let mut selected = Vec::new();
        let mut total: u64 = 0;
        let base_fee = (10 + 34 * 2) as f64 * fee_rate; // Base tx size
        
        for utxo in utxos {
            selected.push(utxo.clone());
            total += utxo.amount_satoshis();
            
            let input_fee = (selected.len() as f64 * 68.0 * fee_rate) as u64;
            let total_fee = base_fee as u64 + input_fee;
            
            if total >= target + total_fee {
                return Ok(CoinSelection::new(
                    selected,
                    target,
                    total_fee,
                    CoinSelectionStrategy::LargestFirst,
                ));
            }
        }
        
        Err(anyhow!("Insufficient funds"))
    }
}

// ============================================================================
// SMALLEST FIRST STRATEGY (Good for consolidation)
// ============================================================================

pub struct SmallestFirst;

impl SmallestFirst {
    pub fn new() -> Self {
        Self
    }
    
    pub fn select(
        &self,
        mut utxos: Vec<Utxo>,
        target: u64,
        fee_rate: f64,
    ) -> Result<CoinSelection> {
        if utxos.is_empty() {
            return Err(anyhow!("No UTXOs available"));
        }
        
        // Sort by amount ascending
        utxos.sort_by(|a, b| {
            a.amount_satoshis().cmp(&b.amount_satoshis())
        });
        
        let mut selected = Vec::new();
        let mut total: u64 = 0;
        let base_fee = (10 + 34 * 2) as f64 * fee_rate;
        
        for utxo in utxos {
            selected.push(utxo.clone());
            total += utxo.amount_satoshis();
            
            let input_fee = (selected.len() as f64 * 68.0 * fee_rate) as u64;
            let total_fee = base_fee as u64 + input_fee;
            
            if total >= target + total_fee {
                return Ok(CoinSelection::new(
                    selected,
                    target,
                    total_fee,
                    CoinSelectionStrategy::SmallestFirst,
                ));
            }
        }
        
        Err(anyhow!("Insufficient funds"))
    }
}

// ============================================================================
// FIFO (First-In-First-Out)
// ============================================================================

pub struct Fifo;

impl Fifo {
    pub fn new() -> Self {
        Self
    }
    
    pub fn select(
        &self,
        mut utxos: Vec<Utxo>,
        target: u64,
        fee_rate: f64,
    ) -> Result<CoinSelection> {
        if utxos.is_empty() {
            return Err(anyhow!("No UTXOs available"));
        }
        
        // Sort by last_updated ascending (oldest first)
        utxos.sort_by(|a, b| a.last_updated.cmp(&b.last_updated));
        
        let mut selected = Vec::new();
        let mut total: u64 = 0;
        let base_fee = (10 + 34 * 2) as f64 * fee_rate;
        
        for utxo in utxos {
            selected.push(utxo.clone());
            total += utxo.amount_satoshis();
            
            let input_fee = (selected.len() as f64 * 68.0 * fee_rate) as u64;
            let total_fee = base_fee as u64 + input_fee;
            
            if total >= target + total_fee {
                return Ok(CoinSelection::new(
                    selected,
                    target,
                    total_fee,
                    CoinSelectionStrategy::Fifo,
                ));
            }
        }
        
        Err(anyhow!("Insufficient funds"))
    }
}

// ============================================================================
// LIFO (Last-In-First-Out)
// ============================================================================

pub struct Lifo;

impl Lifo {
    pub fn new() -> Self {
        Self
    }
    
    pub fn select(
        &self,
        mut utxos: Vec<Utxo>,
        target: u64,
        fee_rate: f64,
    ) -> Result<CoinSelection> {
        if utxos.is_empty() {
            return Err(anyhow!("No UTXOs available"));
        }
        
        // Sort by last_updated descending (newest first)
        utxos.sort_by(|a, b| b.last_updated.cmp(&a.last_updated));
        
        let mut selected = Vec::new();
        let mut total: u64 = 0;
        let base_fee = (10 + 34 * 2) as f64 * fee_rate;
        
        for utxo in utxos {
            selected.push(utxo.clone());
            total += utxo.amount_satoshis();
            
            let input_fee = (selected.len() as f64 * 68.0 * fee_rate) as u64;
            let total_fee = base_fee as u64 + input_fee;
            
            if total >= target + total_fee {
                return Ok(CoinSelection::new(
                    selected,
                    target,
                    total_fee,
                    CoinSelectionStrategy::Lifo,
                ));
            }
        }
        
        Err(anyhow!("Insufficient funds"))
    }
}

// ============================================================================
// RANDOM SELECTION (Privacy-focused)
// ============================================================================

pub struct RandomSelection;

impl RandomSelection {
    pub fn new() -> Self {
        Self
    }
    
    pub fn select(
        &self,
        mut utxos: Vec<Utxo>,
        target: u64,
        fee_rate: f64,
    ) -> Result<CoinSelection> {
        use rand::seq::SliceRandom;
        use rand::thread_rng;
        
        if utxos.is_empty() {
            return Err(anyhow!("No UTXOs available"));
        }
        
        // Shuffle randomly
        let mut rng = thread_rng();
        utxos.shuffle(&mut rng);
        
        let mut selected = Vec::new();
        let mut total: u64 = 0;
        let base_fee = (10 + 34 * 2) as f64 * fee_rate;
        
        for utxo in utxos {
            selected.push(utxo.clone());
            total += utxo.amount_satoshis();
            
            let input_fee = (selected.len() as f64 * 68.0 * fee_rate) as u64;
            let total_fee = base_fee as u64 + input_fee;
            
            if total >= target + total_fee {
                return Ok(CoinSelection::new(
                    selected,
                    target,
                    total_fee,
                    CoinSelectionStrategy::Random,
                ));
            }
        }
        
        Err(anyhow!("Insufficient funds"))
    }
}

// ============================================================================
// ADVANCED UTXO SELECTOR
// ============================================================================

pub struct AdvancedUtxoSelector;

impl AdvancedUtxoSelector {
    /// Select UTXOs using specified strategy
    pub fn select(
        utxos: Vec<Utxo>,
        target: u64,
        fee_rate: f64,
        strategy: CoinSelectionStrategy,
    ) -> Result<CoinSelection> {
        match strategy {
            CoinSelectionStrategy::BranchAndBound => {
                BranchAndBound::new().select(utxos, target, fee_rate)
            }
            CoinSelectionStrategy::LargestFirst => {
                LargestFirst::new().select(utxos, target, fee_rate)
            }
            CoinSelectionStrategy::SmallestFirst => {
                SmallestFirst::new().select(utxos, target, fee_rate)
            }
            CoinSelectionStrategy::Fifo => {
                Fifo::new().select(utxos, target, fee_rate)
            }
            CoinSelectionStrategy::Lifo => {
                Lifo::new().select(utxos, target, fee_rate)
            }
            CoinSelectionStrategy::Random => {
                RandomSelection::new().select(utxos, target, fee_rate)
            }
            CoinSelectionStrategy::SingleRandomDraw => {
                // Try to use a single UTXO for maximum privacy
                Self::single_random_draw(utxos, target, fee_rate)
            }
        }
    }
    
    /// Try to find a single UTXO that covers the target (best for privacy)
    fn single_random_draw(
        mut utxos: Vec<Utxo>,
        target: u64,
        fee_rate: f64,
    ) -> Result<CoinSelection> {
        use rand::seq::SliceRandom;
        use rand::thread_rng;
        
        let base_fee = (10 + 34 * 2 + 68) as f64 * fee_rate; // 1 input
        let needed = target + base_fee as u64;
        
        // Filter UTXOs that can cover the amount
        let suitable: Vec<_> = utxos.iter()
            .filter(|u| u.amount_satoshis() >= needed)
            .cloned()
            .collect();
        
        if suitable.is_empty() {
            // Fall back to random selection with multiple UTXOs
            return RandomSelection::new().select(utxos, target, fee_rate);
        }
        
        // Pick one randomly
        let mut rng = thread_rng();
        let selected = suitable.choose(&mut rng)
            .ok_or_else(|| anyhow!("Failed to select UTXO"))?;
        
        Ok(CoinSelection::new(
            vec![selected.clone()],
            target,
            base_fee as u64,
            CoinSelectionStrategy::SingleRandomDraw,
        ))
    }
    
    /// Auto-select best strategy based on context
    pub fn auto_select(
        utxos: Vec<Utxo>,
        target: u64,
        fee_rate: f64,
        prefer_privacy: bool,
    ) -> Result<CoinSelection> {
        // Try single random draw first if privacy preferred
        if prefer_privacy {
            if let Ok(selection) = Self::single_random_draw(utxos.clone(), target, fee_rate) {
                return Ok(selection);
            }
        }
        
        // Try Branch and Bound for optimal selection
        if let Ok(selection) = BranchAndBound::new().select(utxos.clone(), target, fee_rate) {
            return Ok(selection);
        }
        
        // Fall back to largest-first
        LargestFirst::new().select(utxos, target, fee_rate)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    fn create_test_utxo(amount_sat: u64) -> Utxo {
        Utxo {
            txid: "test".to_string(),
            vout: 0,
            amount: amount_sat as f64 / 100_000_000.0,
            script_pubkey: "".to_string(),
            confirmations: 6,
            spendable: true,
            locked: false,
            address: "test".to_string(),
            last_updated: Utc::now(),
        }
    }

    #[test]
    fn test_branch_and_bound() {
        let utxos = vec![
            create_test_utxo(100_000),
            create_test_utxo(50_000),
            create_test_utxo(30_000),
            create_test_utxo(20_000),
        ];
        
        let result = BranchAndBound::new().select(utxos, 80_000, 1.0);
        assert!(result.is_ok());
        
        let selection = result.unwrap();
        assert!(selection.total_input >= 80_000);
    }
    
    #[test]
    fn test_largest_first() {
        let utxos = vec![
            create_test_utxo(10_000),
            create_test_utxo(50_000),
            create_test_utxo(30_000),
        ];
        
        let result = LargestFirst::new().select(utxos, 40_000, 1.0);
        assert!(result.is_ok());
        
        let selection = result.unwrap();
        assert_eq!(selection.utxos.len(), 1); // Should select the 50k UTXO
        assert_eq!(selection.utxos[0].amount_satoshis(), 50_000);
    }
    
    #[test]
    fn test_smallest_first() {
        let utxos = vec![
            create_test_utxo(100_000),
            create_test_utxo(50_000),
            create_test_utxo(10_000),
        ];
        
        let result = SmallestFirst::new().select(utxos, 55_000, 1.0);
        assert!(result.is_ok());
        
        let selection = result.unwrap();
        // Should select 10k + 50k
        assert!(selection.utxos.len() >= 2);
    }
}
