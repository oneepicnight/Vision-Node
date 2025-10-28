param(
    [switch]$Foreground,
    [switch]$NoBrowser,
    [int]$KeepLogs = 10,
    [int]$Port = 7070
)

# Stop any existing vision-node processes (best-effort)
Stop-Process -Name vision-node -ErrorAction SilentlyContinue
Start-Sleep -Milliseconds 200

$repoRoot = Resolve-Path (Join-Path $PSScriptRoot "..")

# Prefer release binary when VISION_RELEASE=1 is set; fallback to debug
if ($env:VISION_RELEASE -eq '1') {
    $exe = Join-Path $repoRoot "target\release\vision-node.exe"
} else {
    $exe = Join-Path $repoRoot "target\debug\vision-node.exe"
}

if (-not (Test-Path $exe)) {
    Write-Error "Executable not found: $exe"
    exit 1
}

$logDir = Join-Path $repoRoot "logs"
New-Item -ItemType Directory -Path $logDir -Force | Out-Null

$logFile = Join-Path $repoRoot "vision-node.log"

# If a log exists, rotate it into logs/vision-node-<ts>.log
if (Test-Path $logFile) {
    $ts = (Get-Date).ToString('yyyyMMdd-HHmmss')
    $rotated = Join-Path $logDir ("vision-node-$ts.log")
    Move-Item -Path $logFile -Destination $rotated -Force
}

# Prune old rotated logs, keep the most recent $KeepLogs
$rotatedFiles = Get-ChildItem -Path $logDir -Filter 'vision-node-*.log' | Sort-Object LastWriteTime -Descending
if ($rotatedFiles.Count -gt $KeepLogs) {
    $toDelete = $rotatedFiles | Select-Object -Skip $KeepLogs
    foreach ($f in $toDelete) { Remove-Item -LiteralPath $f.FullName -Force }
}

# Start the node with stdout/err redirected into $logFile
$argsList = @('--port',$Port)

# Start the node in a background job and redirect both stdout and stderr
# into the same log file. Using Start-Process with both RedirectStandardOutput
# and RedirectStandardError pointing to the same file can fail on some
# PowerShell/runtime combinations, so we run the exe inside a job and perform
# shell-level redirection (2>&1) there.
$job = Start-Job -ScriptBlock {
    param($exePath, $argsArr, $cwd, $outFile)
    # Enable Rust backtraces for foreground debugging so panics are visible in logs
    $env:RUST_BACKTRACE = '1'
    Set-Location $cwd
    & $exePath @argsArr 2>&1 | Out-File -FilePath $outFile -Encoding UTF8 -Append
} -ArgumentList $exe, $argsList, $repoRoot, $logFile

Start-Sleep -Milliseconds 200

# Attempt to find the process PID for user feedback. This may be slightly
# racy but usually succeeds immediately after the job launches the process.
try {
    $procName = [System.IO.Path]::GetFileNameWithoutExtension($exe)
    $proc = Get-Process -Name $procName -ErrorAction SilentlyContinue | Sort-Object StartTime -Descending | Select-Object -First 1
    if ($proc) {
        Write-Output ("PID: $($proc.Id)")
    } else {
        Write-Output ("JobId: $($job.Id) (process PID not yet available)")
    }
} catch {
    Write-Output ("JobId: $($job.Id) (failed to determine PID)")
}

# Helper: wait for /version to respond (returns true on success)
function Wait-For-Version {
    param([int]$port = 7070, [int]$timeoutSec = 30)
    $uri = "http://127.0.0.1:$port/version"
    $deadline = (Get-Date).AddSeconds($timeoutSec)
    while ((Get-Date) -lt $deadline) {
        try {
            $resp = Invoke-RestMethod -Uri $uri -Method Get -ErrorAction Stop
            if ($null -ne $resp) { return $true }
        } catch { Start-Sleep -Milliseconds 300 }
    }
    return $false
}

if ($Foreground) {
    # In foreground mode we open the browser once the server is ready and stream logs to console
    $ready = Wait-For-Version -port $Port -timeoutSec 30
    if ($ready -and (-not $NoBrowser)) {
        Start-Process "http://127.0.0.1:$Port"
    }

    Write-Output "Tailing logs ($logFile). Press Ctrl+C to exit tail (process remains running)."
    try {
        Get-Content -Path $logFile -Tail 50 -Wait
    } catch {
        Write-Warning "Log tailing stopped: $_"
    }
} else {
    Write-Output "Node started in background; logs: $logFile"
}
