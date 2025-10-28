Write-Output '==> Test TCP port 7070'

try {
    Start-Sleep -Seconds 1
    if (Get-Command Test-NetConnection -ErrorAction SilentlyContinue) {
        Write-Output '=== Test-NetConnection ==='
        Test-NetConnection -ComputerName 127.0.0.1 -Port 7070 | Format-List
    } else {
        Write-Warning 'Test-NetConnection not available on this host'
        Write-Output '=== netstat ==='
        netstat -ano | Select-String ':7070' | ForEach-Object { $_.Line }
    }
} catch { Write-Warning "Test-NetConnection failed: $_" }

Write-Output '=== Processes ==='
Get-Process -Name vision-node -ErrorAction SilentlyContinue | Select-Object Id,ProcessName,StartTime | Format-List

Write-Output '=== tail vision-node.log (last 120 lines) ==='
if (Test-Path 'C:\vision-node\vision-node.log') { Get-Content -Path 'C:\vision-node\vision-node.log' -Tail 120 } else { Write-Output 'no log file' }

$endpoints = @('/version','/panel/','/')
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
