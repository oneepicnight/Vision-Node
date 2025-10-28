$exe = 'C:\vision-node\target\debug\vision-node.exe'
if (-not (Test-Path $exe)) { Write-Error "exe not found: $exe"; exit 2 }
$p = Start-Process -FilePath $exe -ArgumentList '--port','7070' -WorkingDirectory 'C:\vision-node' -PassThru
Write-Output "Started PID: $($p.Id)"
Start-Sleep -Seconds 1
Get-Process -Id $p.Id -ErrorAction SilentlyContinue | Select-Object Id,ProcessName,StartTime | Format-List
if (Test-Path 'C:\vision-node\vision-node.log') { Write-Output '--- tail of log ---'; Get-Content -Path 'C:\vision-node\vision-node.log' -Tail 80 } else { Write-Output 'no log file' }
