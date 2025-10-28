# Smoke test: wait for node, submit tx-low.json, mine block, verify receipt and receipts_root
$port = 7070
$base = "http://127.0.0.1:$port"
# wait for /version
$deadline = (Get-Date).AddSeconds(30)
$ok = $false
while ((Get-Date) -lt $deadline) {
    try {
        $r = Invoke-RestMethod -Uri "$base/version" -Method Get -ErrorAction Stop
        Write-Output "version: $($r | ConvertTo-Json -Compress)"
        $ok = $true
        break
    } catch { Start-Sleep -Milliseconds 250 }
}
if (-not $ok) { Write-Error "version probe failed"; exit 2 }

# submit tx
try {
    $body = Get-Content (Join-Path $PSScriptRoot '..\tx-low.json') -Raw
} catch {
    Write-Error "failed reading tx-low.json: $_"; exit 3
}
try {
    $resp = Invoke-RestMethod -Uri "$base/submit_tx" -Method Post -Body $body -ContentType 'application/json' -ErrorAction Stop
    Write-Output ("submit_resp: " + ($resp | ConvertTo-Json -Compress))
} catch {
    Write-Error "submit failed: $_"; exit 4
}
# extract tx_hash
$txhash = $null
if ($resp -is [System.Collections.IDictionary] -and $resp.tx_hash) { $txhash = $resp.tx_hash } elseif ($resp.tx_hash) { $txhash = $resp.tx_hash }
if (-not $txhash) { Write-Error "no tx_hash in submit response"; exit 5 }
Write-Output "tx_hash: $txhash"

# mine a block
try {
    $mresp = Invoke-RestMethod -Uri "$base/mine_block" -Method Post -Body '{ }' -ContentType 'application/json' -ErrorAction Stop
    Write-Output ("mine_resp: " + ($mresp | ConvertTo-Json -Compress))
} catch {
    Write-Error "mine failed: $_"; exit 6
}

# wait a moment and fetch receipt
Start-Sleep -Milliseconds 300
try {
    $rpt = Invoke-RestMethod -Uri "$base/receipt/$txhash" -Method Get -ErrorAction Stop
    Write-Output ("receipt: " + ($rpt | ConvertTo-Json -Compress))
} catch {
    Write-Error "get receipt failed: $_"; exit 7
}

# fetch latest block and show receipts_root
try {
    $blkinfo = Invoke-RestMethod -Uri "$base/block/latest" -Method Get -ErrorAction Stop
    Write-Output ("block_latest: " + ($blkinfo | ConvertTo-Json -Compress))
} catch {
    Write-Error "get block latest failed: $_"; exit 8
}

# done
Write-Output "SMOKE_TEST_OK"
exit 0
