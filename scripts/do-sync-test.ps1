Write-Host '== GET /version =='
try {
    $v = Invoke-RestMethod -Uri 'http://127.0.0.1:7070/version' -TimeoutSec 5 -ErrorAction Stop
    $v | ConvertTo-Json -Depth 5 | Write-Host
} catch { Write-Host 'version err:' $_.Exception.Message }

Write-Host ''
Write-Host '== POST /sync/pull (synthetic failing peer) =='
$body = @{ src = 'http://127.0.0.1:9999' } | ConvertTo-Json
try {
    $r = Invoke-RestMethod -Uri 'http://127.0.0.1:7070/sync/pull' -Method Post -Body $body -ContentType 'application/json' -TimeoutSec 10 -ErrorAction Stop
    Write-Host 'OK:'; $r | ConvertTo-Json -Depth 5 | Write-Host
} catch { Write-Host 'sync pull error:' $_.Exception.Message; if ($_.Exception.Response -ne $null) { Write-Host 'Response status:' ($_.Exception.Response.StatusCode.value__); try { $txt = $_.Exception.Response.GetResponseStream(); $sr = New-Object System.IO.StreamReader($txt); $sr.ReadToEnd() | Write-Host } catch { } } }

Write-Host ''
Write-Host '== Tail vision-node.log (last 200 lines) =='
if (Test-Path 'C:\vision-node\vision-node.log') { Get-Content -Path 'C:\vision-node\vision-node.log' -Tail 200 } else { Write-Host 'vision-node.log not found' }
