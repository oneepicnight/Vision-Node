$repoRoot = Split-Path -Parent $PSScriptRoot
$candidate1 = Join-Path $repoRoot 'vision-panel\dist'
$candidate2 = Join-Path $repoRoot 'vision-node-restored\vision-panel\dist'

$dist = if (Test-Path $candidate1) { $candidate1 } elseif (Test-Path $candidate2) { $candidate2 } else { $null }
$public = Join-Path $repoRoot 'public'

if (-not (Test-Path $dist)) {
    Write-Error "dist missing: $candidate1 or $candidate2"
    exit 2
}
if (Test-Path $public) {
    $ts = (Get-Date).ToString('yyyyMMdd-HHmmss')
    $bak = "$public.bak.$ts"
    Rename-Item -LiteralPath $public -NewName (Split-Path $bak -Leaf) -Force
    Write-Output "Backed up existing public to $bak"
}
New-Item -ItemType Directory -Path $public -Force | Out-Null
Copy-Item -Path (Join-Path $dist '*') -Destination $public -Recurse -Force
Write-Output "COPIED_DIST_TO_PUBLIC"
try {
    $r = Invoke-WebRequest -Uri 'http://127.0.0.1:7070/panel/' -UseBasicParsing -ErrorAction Stop
    Write-Output ("PANEL_STATUS:" + $r.StatusCode + " LENGTH:" + ($r.RawContentLength))
} catch {
    Write-Output ("PANEL_FAIL:" + $_.Exception.Message)
}
try {
    $r2 = Invoke-WebRequest -Uri 'http://127.0.0.1:7070/' -UseBasicParsing -ErrorAction Stop
    Write-Output ("ROOT_STATUS:" + $r2.StatusCode + " LENGTH:" + ($r2.RawContentLength))
} catch {
    Write-Output ("ROOT_FAIL:" + $_.Exception.Message)
}
