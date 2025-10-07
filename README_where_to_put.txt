WHERE THESE FILES GO
--------------------
- Put `main.rs` into your project at:  C:\vision-node\src\main.rs   (replace the existing file)
- Optional helper: `test-airdrop.ps1` can live anywhere; I assume C:\vision-node\test-airdrop.ps1 below.

QUICK APPLY
-----------
1) Back up your current file (optional):
   Copy-Item C:\vision-node\src\main.rs C:\vision-node\src\main.before-multi-mint.rs -Force

2) Copy in the new file (the one you just downloaded):
   Copy-Item $env:USERPROFILE\Downloads\main.rs C:\vision-node\src\main.rs -Force

3) Build & run:
   cd C:\vision-node
   cargo run

4) Test (PowerShell):
   $base='http://127.0.0.1:7070'
   $tok='letmein'
   # health + height
   irm "$base/health"
   irm "$base/height"

   # set GameMaster to 'alice' (consensus-safe)
   irm -Method Post "$base/set_gamemaster?token=$tok" -ContentType "application/json" -Body (@{ addr="alice" } | ConvertTo-Json)

   # airdrop via multi_mint (bob=25, charlie=40)
   $body = @{ payments = @(@{to="bob";amount=25}, @{to="charlie";amount=40}) } | ConvertTo-Json -Depth 5
   irm -Method Post "$base/airdrop?token=$tok" -ContentType "application/json" -Body $body

   # balances
   irm "$base/balance/bob"
   irm "$base/balance/charlie"

NOTES
-----
- This `main.rs` includes:
  * GameMaster stored in the state root (consensus-safe).
  * Admin endpoints that BUILD on-chain txs (`system/set_gamemaster`, `cash/multi_mint`).
  * New `cash/multi_mint` method: GM-only, mints to many addresses in one tx, no fees.
  * Borrow-checker cleanups to avoid E0499/E0502.
  * Per-port data directory: `VISION_PORT=7071` uses `./vision_data_7071` etc.

