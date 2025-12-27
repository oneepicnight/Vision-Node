# Test Vision Node Website Access
Write-Host "🌐 Testing Vision Node Website..." -ForegroundColor Cyan
Write-Host ""

# Check if node is running
$proc = Get-Process -Name "vision-node" -ErrorAction SilentlyContinue
if (-not $proc) {
    Write-Host "❌ Vision Node is not running!" -ForegroundColor Red
    Write-Host "Start it with: .\target\release\vision-node.exe" -ForegroundColor Yellow
    exit 1
}

Write-Host "✅ Vision Node is running (PID: $($proc.Id))" -ForegroundColor Green
Write-Host ""

# Test endpoints
$endpoints = @{
    "/health" = "Health Check"
    "/api/status" = "Chain Status API"
    "/api/constellation" = "Constellation API"
    "/api/mood" = "Mood API"
    "/dashboard.html" = "Dashboard"
    "/panel.html" = "Miner Panel"
    "/" = "Website Root"
}

Write-Host "Testing Endpoints:" -ForegroundColor Yellow
Write-Host "-" * 60
foreach ($path in $endpoints.Keys) {
    try {
        $response = Invoke-WebRequest -Uri "http://127.0.0.1:7070$path" -TimeoutSec 3 -MaximumRedirection 0 -ErrorAction Stop
        $status = $response.StatusCode
        $size = $response.Content.Length
        Write-Host "✅ $path - $($endpoints[$path])" -ForegroundColor Green
        Write-Host "   Status: $status, Size: $size bytes" -ForegroundColor Gray
    } catch {
        $statusCode = $_.Exception.Response.StatusCode.value__
        if ($statusCode -eq 308 -or $statusCode -eq 307 -or $statusCode -eq 301 -or $statusCode -eq 302) {
            Write-Host "↪️  $path - $($endpoints[$path])" -ForegroundColor Yellow
            Write-Host "   Redirect ($statusCode)" -ForegroundColor Gray
        } else {
            Write-Host "❌ $path - $($endpoints[$path])" -ForegroundColor Red
            Write-Host "   Error: $($_.Exception.Message)" -ForegroundColor Gray
        }
    }
    Write-Host ""
}

Write-Host ""
Write-Host "📍 Access URLs:" -ForegroundColor Cyan
Write-Host "   Website:   http://127.0.0.1:7070/" -ForegroundColor White
Write-Host "   Dashboard: http://127.0.0.1:7070/dashboard.html" -ForegroundColor White
Write-Host "   Panel:     http://127.0.0.1:7070/panel.html" -ForegroundColor White
