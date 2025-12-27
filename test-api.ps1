# Test Vision Node API Endpoints
Write-Host "`n=== Vision Node API Test ===" -ForegroundColor Cyan
Write-Host "Testing after API cleanup`n" -ForegroundColor Yellow

# Check if node is running
$process = Get-Process -Name "vision-node" -ErrorAction SilentlyContinue
if ($process) {
    Write-Host "✓ Node is running (PID: $($process.Id))" -ForegroundColor Green
} else {
    Write-Host "✗ Node is NOT running!" -ForegroundColor Red
    exit 1
}

Start-Sleep -Seconds 3

# Test endpoints
$endpoints = @(
    '/api/status',
    '/api/constellation',
    '/api/mood',
    '/api/guardian',
    '/api/health/public',
    '/api/health',
    '/api/nodes',
    '/api/trauma',
    '/api/patterns',
    '/api/reputation',
    '/api/chain/status',
    '/api/bootstrap'
)

Write-Host "`nTesting Endpoints:" -ForegroundColor Cyan
Write-Host "==================" -ForegroundColor Cyan

foreach ($endpoint in $endpoints) {
    $url = "http://127.0.0.1:7070$endpoint"
    try {
        $response = Invoke-WebRequest -Uri $url -UseBasicParsing -TimeoutSec 3 -ErrorAction Stop
        $status = $response.StatusCode
        
        Write-Host "$endpoint" -NoNewline
        Write-Host " - " -NoNewline
        if ($status -eq 200) {
            Write-Host "✓ OK ($status)" -ForegroundColor Green
            
            # Try to parse JSON
            try {
                $json = $response.Content | ConvertFrom-Json
                if ($json.ok -ne $null) {
                    Write-Host "    ok: $($json.ok)" -ForegroundColor DarkGray
                }
            } catch {
                # Not JSON or cannot parse, that is fine
            }
        } else {
            Write-Host "? Status $status" -ForegroundColor Yellow
        }
    } catch {
        Write-Host "$endpoint" -NoNewline
        Write-Host " - " -NoNewline
        Write-Host "✗ FAIL ($($_.Exception.Message))" -ForegroundColor Red
    }
}

Write-Host "`n=== Test Complete ===" -ForegroundColor Cyan
