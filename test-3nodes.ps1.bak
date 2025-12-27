param(
  [int[]]$Ports = @(7070,7071,7072),
  [string]$Token = "letmein",
  [string]$Base  = "http://127.0.0.1",
  [switch]$PeerOnly = $false,
  [switch]$SkipAirdrop = $false
)

# -------- helpers --------
function Write-Step([string]$msg)  { Write-Host "[STEP] $msg" -ForegroundColor Cyan }
function Write-OK([string]$msg)    { Write-Host "[ OK ] $msg" -ForegroundColor Green }
function Write-ERR([string]$msg)   { Write-Host "[ERR ] $msg" -ForegroundColor Red }

function Url([int]$port, [string]$path) {
  return "$($Base):$port$path"
}

function Get-Height([int]$port) {
  try { return [int](Invoke-RestMethod (Url $port "/height")) } catch { return -1 }
}

function Add-Peer([int]$from, [int]$to) {
  $url  = (Url $from "/peer/add?token=$Token")
  $body = @{ url = "$($Base):$to" } | ConvertTo-Json -Compress
  try {
    $resp = Invoke-RestMethod -Method Post -ContentType "application/json" -Body $body -Uri $url
    return $true
  } catch {
    Write-ERR "peer add $from -> $to failed: $($_.Exception.Message)"
    return $false
  }
}

function Force-Catchup([int]$src, [int]$dst) {
  $srcH = Get-Height $src
  $dstH = Get-Height $dst
  if ($srcH -lt 0 -or $dstH -lt 0) { Write-ERR "cannot fetch heights"; return }
  if ($dstH -ge $srcH) { return }
  Write-Step ("Force catch-up {0} from {1} (dst={2} to src={3})" -f $dst,$src,$dstH,$srcH)
  for ($i = $dstH + 1; $i -le $srcH; $i++) {
    try {
      $bObj = Invoke-RestMethod (Url $src "/block/$i")
      $env  = @{ block = $bObj } | ConvertTo-Json -Depth 100 -Compress
      $resp = Invoke-RestMethod -Method Post -ContentType "application/json" -Body $env -Uri (Url $dst "/gossip/block")
      Start-Sleep -Milliseconds 50
    } catch {
      Write-ERR "push block $i to $dst failed: $($_.Exception.Message)"
    }
  }
}

function Wait-Heights([int[]]$ports, [int]$target, [int]$timeoutSec=15) {
  $deadline = (Get-Date).AddSeconds($timeoutSec)
  while ((Get-Date) -lt $deadline) {
    $all = $true
    foreach ($p in $ports) {
      $h = Get-Height $p
      if ($h -lt $target) { $all = $false; break }
    }
    if ($all) { return $true }
    Start-Sleep -Milliseconds 200
  }
  return $false
}

function Mine-One([int]$port) {
  $url = Url $port "/mine_block"
  $body = @{} | ConvertTo-Json -Compress
  try {
    $r = Invoke-RestMethod -Method Post -ContentType "application/json" -Body $body -Uri $url
    return $r.height
  } catch {
    Write-ERR "mine on $port failed: $($_.Exception.Message)"
    return -1
  }
}

function Set-GM([int]$port, [string]$addr) {
  $url  = Url $port "/set_gamemaster?token=$Token"
  $body = @{ addr = $addr } | ConvertTo-Json -Compress
  try { Invoke-RestMethod -Method Post -ContentType "application/json" -Body $body -Uri $url | Out-Null; return $true }
  catch { Write-ERR "set_gamemaster failed: $($_.Exception.Message)"; return $false }
}

function Airdrop([int]$port, [string]$from, [string]$csv, [int]$tip=2) {
  $url  = Url $port "/airdrop?token=$Token"
  $body = @{
    from = $from
    tip  = $tip
    payments_csv = $csv
  } | ConvertTo-Json -Compress
  try { Invoke-RestMethod -Method Post -ContentType "application/json" -Body $body -Uri $url | Out-Null; return $true }
  catch { Write-ERR "airdrop failed: $($_.Exception.Message)"; return $false }
}

function Get-Bal([int]$port, [string]$addr) {
  try { return [int64](Invoke-RestMethod (Url $port "/balance/$addr")) } catch { return -1 }
}

# -------- run --------
Write-Step "Mesh peers"
$N = $Ports.Count
for ($i=0; $i -lt $N; $i++) {
  for ($j=0; $j -lt $N; $j++) {
    if ($i -eq $j) { continue }
    $null = Add-Peer $Ports[$i] $Ports[$j]
  }
}

if (-not $PeerOnly) {
  Write-Step "Mine one block on $($Ports[0])"
  $h = Mine-One $Ports[0]
  if ($h -gt 0) {
    if (-not (Wait-Heights $Ports $h 20)) {
      # last resort, force catch-up to the highest
      $maxH = -1; $src = $Ports[0]
      foreach ($p in $Ports) { $hh = Get-Height $p; if ($hh -gt $maxH) { $maxH = $hh; $src = $p } }
      foreach ($p in $Ports) { if ($p -ne $src) { Force-Catchup $src $p } }
    }
  }

  Write-Step "Set gamemaster = alice on $($Ports[0])"
  $ok = Set-GM $Ports[0] "alice"
  if (-not $ok) { Write-ERR "failed to set gamemaster"; }

  if (-not $SkipAirdrop) {
    Write-Step "Airdrop bob=25, charlie=40 via $($Ports[0])"
    $csv = "bob,25`ncharlie,40"
    $ok = Airdrop $Ports[0] "alice" $csv 2
    if (-not $ok) { Write-ERR "airdrop call failed" }
  }
}

# Verify balances everywhere (allow catch-up)
Write-Step "Verify balances on all nodes"
$expected = @{ bob = 25; charlie = 40 }
foreach ($p in $Ports) {
  $b = Get-Bal $p "bob"
  $c = Get-Bal $p "charlie"
  if ($b -ne $expected.bob -or $c -ne $expected.charlie) {
    Write-Host "[INFO] Port $p needs catch-up (bob=$b, charlie=$c)"
    # Find best source (highest height)
    $maxH = -1; $src = $Ports[0]
    foreach ($q in $Ports) { $hh = Get-Height $q; if ($hh -gt $maxH) { $maxH = $hh; $src = $q } }
    if ($src -ne $p) { Force-Catchup $src $p }
    # re-check
    $b = Get-Bal $p "bob"; $c = Get-Bal $p "charlie"
  }
  if ($b -eq $expected.bob -and $c -eq $expected.charlie) {
    Write-OK ("Port {0} balances OK (bob={1}, charlie={2})" -f $p,$b,$c)
  } else {
    Write-ERR ("Port {0} balances WRONG (bob={1}, charlie={2})" -f $p,$b,$c)
  }
}

Write-OK "3-node test completed"
