param(
  [switch]$Fresh
)

# ---------------- helpers ----------------
function Write-OK   ($m) { Write-Host "[ OK ] $m"   -ForegroundColor Green }
function Write-Err  ($m) { Write-Host "[ERR ] $m"  -ForegroundColor Red }
function Write-Step ($m) { Write-Host "[STEP] $m"  -ForegroundColor Cyan }
function Write-Info ($m) { Write-Host "[INFO] $m"  -ForegroundColor DarkCyan }

$Base   = 'http://127.0.0.1'
$Token  = 'letmein'
$LeaderPort = 7070
$LeaderDir  = 'C:\vision-node'
$ExeLeader  = Join-Path $LeaderDir 'target\debug\vision-node.exe'

$Followers = @(
  @{ Port = 7071; Dir = 'C:\vision-node-7071'; Exe = 'C:\vision-node-7071\target\debug\vision-node.exe' },
  @{ Port = 7072; Dir = 'C:\vision-node-7072'; Exe = 'C:\vision-node-7072\target\debug\vision-node.exe' }
)

# ---- wipe data if asked ----
if($Fresh){
  Write-Step "Fresh mode: wiping ALL nodes' vision_data"
  $dirs = @("$LeaderDir\vision_data") + ($Followers | ForEach-Object { "$($_.Dir)\vision_data" })
  foreach($vd in $dirs){
    if(Test-Path $vd){ try { Remove-Item $vd -Recurse -Force -ErrorAction Stop } catch { Write-Err "Failed to wipe $vd: $($_.Exception.Message)" } }
  }
}

# ---- ensure folders exist ----
foreach($f in $Followers){
  if(!(Test-Path $f.Dir)){ New-Item -ItemType Directory -Force -Path $f.Dir | Out-Null }
}

# ---- launch leader ----
if(!(Test-Path $ExeLeader)){ throw "Leader exe not found at $ExeLeader — build first with:  cargo build" }
$cmdLeader = "& {`$env:VISION_ADMIN_TOKEN='$Token'; `$env:VISION_PORT='$LeaderPort'; Set-Location -LiteralPath '$LeaderDir'; & '$ExeLeader' }"
Start-Process powershell -ArgumentList '-NoProfile','-ExecutionPolicy','Bypass','-Command', $cmdLeader -WindowStyle Minimized | Out-Null
Write-Info "Launched node on :$LeaderPort in $LeaderDir"

# ---- launch followers ----
foreach($f in $Followers){
  if(!(Test-Path $f.Exe)){ Copy-Item $ExeLeader $f.Exe -Force -ErrorAction SilentlyContinue }
  if(!(Test-Path $f.Exe)){ throw "Follower exe not found at $($f.Exe) — copy or build it there." }
  $cmd = "& {`$env:VISION_ADMIN_TOKEN='$Token'; `$env:VISION_PORT='$($f.Port)'; Set-Location -LiteralPath '$($f.Dir)'; & '$($f.Exe)' }"
  Start-Process powershell -ArgumentList '-NoProfile','-ExecutionPolicy','Bypass','-Command', $cmd -WindowStyle Minimized | Out-Null
  Write-Info "Launched node on :$($f.Port) in $($f.Dir)"
}

# ---- wait for all ports ----
function Wait-Port($p){
  for($i=0;$i -lt 60;$i++){
    try { $null = Invoke-RestMethod "$($Base):$p/height" -TimeoutSec 2; return }
    catch { Start-Sleep -Milliseconds 300 }
  }
  throw "port $p did not respond"
}
Write-Step "Waiting for nodes..."
Wait-Port $LeaderPort
foreach($f in $Followers){ Wait-Port $f.Port }

# ---- mesh peers (best-effort) ----
function Add-Peer($from,$to){
  try {
    $url  = "$($Base):$from/peer/add?token=$Token"
    $body = @{ url = "$($Base):$to" } | ConvertTo-Json
    Invoke-RestMethod -Method Post $url -ContentType 'application/json' -Body $body | Out-Null
  } catch {
    Write-Err "peer add $from -> $to failed: $($_.Exception.Message)"
  }
}

Write-Step "Meshing peers"
$all = @($LeaderPort) + ($Followers | ForEach-Object { $_.Port })
foreach($a in $all){ foreach($b in $all){ if($a -ne $b){ Add-Peer $a $b } } }

# ---- util: force catch-up (pull blocks from src -> dst) ----
function Force-Catchup($dstPort, $srcPort){
  try{
    $srcH = [int](Invoke-RestMethod "$($Base):$srcPort/height")
    $dstH = [int](Invoke-RestMethod "$($Base):$dstPort/height")
  } catch { Write-Err "cannot fetch heights"; return }

  if($dstH -ge $srcH){ return }
  Write-Step "Force catch-up $dstPort from $srcPort (dst=$dstH to src=$srcH)"
  for($i = $dstH + 1; $i -le $srcH; $i++){
    try{
      $bObj = Invoke-RestMethod "$($Base):$srcPort/block/$i"
      $resp = Invoke-RestMethod -Method Post "$($Base):$dstPort/gossip/block" -ContentType 'application/json' -Body (@{ block = $bObj } | ConvertTo-Json -Depth 100)
      if($resp.status -ne 'accepted'){ Write-Err "dst $dstPort rejected block $i: $($resp.error)"; break }
    } catch {
      Write-Err "dst $dstPort rejected block $i: $($_.Exception.Message)"; break
    }
  }
}

# ---- mine a block on leader (so followers have something to import) ----
Write-Step "Mine one block on $LeaderPort"
try{
  Invoke-RestMethod -Method Post "$($Base):$LeaderPort/mine_block" -ContentType 'application/json' -Body (@{ max_txs = 0 } | ConvertTo-Json) | Out-Null
}catch{}

# ---- catch-up followers on every step ----
foreach($f in $Followers){ Force-Catchup $f.Port $LeaderPort }

# ---- set gamemaster on leader ----
Write-Step "Set gamemaster = alice on $LeaderPort"
try{
  Invoke-RestMethod -Method Post "$($Base):$LeaderPort/set_gamemaster?token=$Token" -ContentType 'application/json' -Body (@{ addr = 'alice' } | ConvertTo-Json) | Out-Null
}catch{
  Write-Err "set_gamemaster failed: $($_.Exception.Message)"
}

foreach($f in $Followers){ Force-Catchup $f.Port $LeaderPort }

# ---- airdrop via leader ----
Write-Step "Airdrop bob=25, charlie=40 via $LeaderPort"
try{
  $csv = "bob,25`ncharlie,40`n"
  Invoke-RestMethod -Method Post "$($Base):$LeaderPort/airdrop?token=$Token" -ContentType 'application/json' -Body (@{ from='alice'; tip=0; payments_csv=$csv } | ConvertTo-Json) | Out-Null
}catch{
  Write-Err "airdrop failed: $($_.Exception.Message)"
}

foreach($f in $Followers){ Force-Catchup $f.Port $LeaderPort }

# ---- status ----
Write-Host "`n--- STATUS ---"
$ports = @($LeaderPort) + ($Followers | ForEach-Object { $_.Port })
foreach($p in $ports){
  try {
    $h = Invoke-RestMethod "$($Base):$p/height"
    $b = Invoke-RestMethod "$($Base):$p/balance/bob"
    $c = Invoke-RestMethod "$($Base):$p/balance/charlie"
    Write-Host "$p: height=$h bob=$b charlie=$c"
  } catch {
    Write-Err "failed to query port $p: $($_.Exception.Message)"
  }
}

Write-OK "All set. Windows keep running; Ctrl+C in each to stop."
