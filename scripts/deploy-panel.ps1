$repoRoot = Split-Path -Parent $PSScriptRoot
$candidate1 = Join-Path $repoRoot 'vision-panel\dist'
$candidate2 = Join-Path $repoRoot 'vision-node-restored\vision-panel\dist'

# Prefer the primary dist, but fall back to restored backup only if present
$src = if (Test-Path $candidate1) { $candidate1 } elseif (Test-Path $candidate2) { $candidate2 } else { $null }
$public = Join-Path $repoRoot 'public'

if (-not (Test-Path $src)) {
    Write-Error "Source dist not found: $candidate1 (or fallback $candidate2)"
    exit 2
}

if (Test-Path $public) {
    $ts = (Get-Date).ToString('yyyyMMdd-HHmmss')
    $bak = "${public}.bak.${ts}"
    Rename-Item -Path $public -NewName $bak -ErrorAction Stop
    Write-Output "Backed up existing public to: $bak"
} else {
    Write-Output 'No existing public dir to back up'
}

# Ensure public exists
New-Item -ItemType Directory -Path $public -Force | Out-Null

# Copy dist contents into public
Copy-Item -Path (Join-Path $src '*') -Destination $public -Recurse -Force
Write-Output 'Copied dist -> public'

# Probe endpoints
$endpoints = @('/panel/','/','/panel/index.html')
foreach ($e in $endpoints) {
    $uri = "http://127.0.0.1:7070$e"
    Write-Output "==> GET $uri"
    try {
        $r = Invoke-WebRequest -Uri $uri -UseBasicParsing -Method Get -TimeoutSec 10 -ErrorAction Stop
        Write-Output "Status: $($r.StatusCode)"
        $text = $r.Content
        if ($null -ne $text -and $text.Length -gt 400) { $text = $text.Substring(0,400) + '...[truncated]' }
        Write-Output $text
    } catch {
        Write-Warning "Request failed: $_"
    }
    Write-Output ''
}
