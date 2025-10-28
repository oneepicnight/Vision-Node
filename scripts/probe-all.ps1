Write-Host '=== TCP probe ==='
Test-NetConnection -ComputerName 127.0.0.1 -Port 7070 | Format-List
Write-Host ""
Write-Host '=== process list (vision-node) ==='
Get-Process -Name vision-node -ErrorAction SilentlyContinue | Select-Object Id,ProcessName,StartTime,CPU,WS | Format-Table -AutoSize
Write-Host ""
Write-Host '=== vision-node.log tail ==='
if (Test-Path 'C:\vision-node\vision-node.log') { Get-Content -Path 'C:\vision-node\vision-node.log' -Tail 200 } else { Write-Host 'vision-node.log not found' }
Write-Host ""
Write-Host '=== rotated logs (top 5) ==='
if (Test-Path 'C:\vision-node\logs') { Get-ChildItem -Path 'C:\vision-node\logs' -File | Sort-Object LastWriteTime -Descending | Select-Object -First 5 | Format-Table Name,LastWriteTime,Length -AutoSize } else { Write-Host 'logs dir missing' }
Write-Host ""
Write-Host '=== newest rotated log tail ==='
$f = Get-ChildItem -Path 'C:\vision-node\logs' -File | Sort-Object LastWriteTime -Descending | Select-Object -First 1
if ($f) { Write-Host 'File:' $f.FullName; Get-Content -Path $f.FullName -Tail 200 } else { Write-Host 'no rotated logs' }
