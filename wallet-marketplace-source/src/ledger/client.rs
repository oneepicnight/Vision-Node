use anyhow::Result;

#[cfg(any(test, feature = "dev"))]
#[allow(dead_code)]
pub async fn move_land(from: &str, to: &str, qty: u64) -> Result<()> {
    println!("Ledger: move {} LAND from {} -> {}", qty, from, to);
    Ok(())
}

pub async fn mint_cash(addr: &str, amount: u64) -> Result<()> {
    println!("Ledger: mint {} CASH to {}", amount, addr);
    Ok(())
}

#[cfg(any(test, feature = "dev"))]
#[allow(dead_code)]
pub async fn reserve_land(addr: &str, qty: u64) -> Result<()> {
    println!("Ledger: reserve {} LAND for {}", qty, addr);
    Ok(())
}
