Write-Host "Testing Vision Peers API..." -ForegroundColor Cyan

Write-Host "`n1. Testing /api/peers/trusted" -ForegroundColor Yellow
$trusted = Invoke-RestMethod -Uri "http://localhost:7070/api/peers/trusted"
Write-Host "   Count: $($trusted.count)" -ForegroundColor Green
$trusted | ConvertTo-Json -Depth 3

Write-Host "`n2. Testing /api/peers/moods" -ForegroundColor Yellow
$moods = Invoke-RestMethod -Uri "http://localhost:7070/api/peers/moods"
Write-Host "   Total: $($moods.total)" -ForegroundColor Green
$moods | ConvertTo-Json -Depth 3

Write-Host "`nDone!" -ForegroundColor Green
