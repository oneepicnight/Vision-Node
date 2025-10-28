Set-Location -LiteralPath 'C:\vision-node'
$hash = '9e094e6'
$bn = "backup/pre-reset-$(Get-Date -Format yyyyMMddHHmmss)"
try {
    git branch $bn $hash
    Write-Host "Created branch: $bn -> $hash"
} catch {
    Write-Host "Failed to create branch:" $_.Exception.Message
}
$bundle = 'C:\vision-node-backup-9e094e6.bundle'
try {
    if (Test-Path -LiteralPath $bundle) { Remove-Item -LiteralPath $bundle -Force }
    git bundle create $bundle $hash
    if (Test-Path -LiteralPath $bundle) { Get-Item -LiteralPath $bundle | Format-List Name,Length,LastWriteTime } else { Write-Host 'Bundle not created' }
} catch {
    Write-Host "Bundle step failed:" $_.Exception.Message
}
