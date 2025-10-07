param(
  [string]$Base = "http://127.0.0.1:7070",
  [string]$Token = "letmein"
)

Write-Host "[STEP] Health + height"
Invoke-RestMethod "$Base/health"
Invoke-RestMethod "$Base/height"

Write-Host "[STEP] Set gamemaster to alice"
$resp = Invoke-RestMethod -Method Post "$Base/set_gamemaster?token=$Token" -ContentType "application/json" -Body (@{ addr="alice" } | ConvertTo-Json)
$resp

Write-Host "[STEP] Airdrop via multi_mint (bob=25, charlie=40)"
$body = @{ payments = @(@{to="bob";amount=25}, @{to="charlie";amount=40}) } | ConvertTo-Json -Depth 5
$drop = Invoke-RestMethod -Method Post "$Base/airdrop?token=$Token" -ContentType "application/json" -Body $body
$drop

Write-Host "[STEP] Check balances"
"bob      = $(Invoke-RestMethod "$Base/balance/bob")"
"charlie  = $(Invoke-RestMethod "$Base/balance/charlie")"
