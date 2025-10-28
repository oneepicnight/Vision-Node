try {
    $r = Invoke-WebRequest -Uri 'http://127.0.0.1:7070/panel/' -UseBasicParsing -TimeoutSec 5 -ErrorAction Stop
    Write-Output ("PANEL_STATUS:" + $r.StatusCode + " LENGTH:" + ($r.RawContentLength))
} catch {
    Write-Output ("PANEL_ERR:" + $_.Exception.Message)
}
try {
    $r2 = Invoke-WebRequest -Uri 'http://127.0.0.1:7070/' -UseBasicParsing -TimeoutSec 5 -ErrorAction Stop
    Write-Output ("ROOT_STATUS:" + $r2.StatusCode + " LENGTH:" + ($r2.RawContentLength))
} catch {
    Write-Output ("ROOT_ERR:" + $_.Exception.Message)
}
