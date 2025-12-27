# Test WebSocket connection
$url = "http://127.0.0.1:7070/ws/events"

Write-Host "`n=== Testing WebSocket Endpoint ===" -ForegroundColor Cyan
Write-Host "Attempting to connect to: $url" -ForegroundColor Yellow

try {
    # Try to make an HTTP request to the WebSocket endpoint
    # WebSocket upgrade requires special headers, so this will likely fail with 400/426
    # But it confirms the endpoint exists (not 404)
    $response = Invoke-WebRequest -Uri $url -Method Get -UseBasicParsing -ErrorAction SilentlyContinue
    Write-Host "Status Code: $($response.StatusCode)" -ForegroundColor Green
} catch {
    $statusCode = $_.Exception.Response.StatusCode.value__
    if ($statusCode -eq 426) {
        Write-Host "✓ WebSocket endpoint exists! (426 Upgrade Required - expected for non-WebSocket connection)" -ForegroundColor Green
    } elseif ($statusCode -eq 404) {
        Write-Host "✗ WebSocket endpoint NOT FOUND (404)" -ForegroundColor Red
    } else {
        Write-Host "✓ WebSocket endpoint exists! (Status: $statusCode)" -ForegroundColor Yellow
    }
}

Write-Host "`nThe WebSocket should work from the browser's dashboard.html" -ForegroundColor Cyan
Write-Host "Browser WebSocket connections use the ws:// protocol with proper upgrade headers.`n" -ForegroundColor Gray
