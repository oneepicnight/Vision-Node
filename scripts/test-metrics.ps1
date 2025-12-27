# Test script for Prometheus metrics endpoint
# Run this after starting the Vision node

$baseUrl = "http://127.0.0.1:7070"

Write-Host "=== Vision Node Prometheus Metrics Test ===" -ForegroundColor Cyan
Write-Host ""

# 1. Test metrics endpoint
Write-Host "[1] Fetching metrics from /metrics..." -ForegroundColor Yellow
try {
    $response = Invoke-RestMethod -Uri "$baseUrl/metrics" -Method Get
    Write-Host "✓ Metrics endpoint responding" -ForegroundColor Green
    Write-Host ""
    
    # Parse and display key metrics
    $lines = $response -split "`n"
    
    Write-Host "Tokenomics Metrics:" -ForegroundColor Cyan
    $lines | Where-Object { $_ -match "^vision_tok_" -and $_ -notmatch "^#" } | ForEach-Object {
        Write-Host "  $_" -ForegroundColor White
    }
    Write-Host ""
    
    Write-Host "Operational Metrics:" -ForegroundColor Cyan
    $lines | Where-Object { $_ -match "^vision_(blocks_height|mempool_len|peers_connected)" -and $_ -notmatch "^#" } | ForEach-Object {
        Write-Host "  $_" -ForegroundColor White
    }
    Write-Host ""
    
} catch {
    Write-Host "✗ Failed to fetch metrics: $_" -ForegroundColor Red
    exit 1
}

# 2. Validate Prometheus format
Write-Host "[2] Validating Prometheus format..." -ForegroundColor Yellow
$hasHelp = ($response -match "# HELP")
$hasType = ($response -match "# TYPE")
$hasGauges = ($response -match "gauge")

if ($hasHelp -and $hasType -and $hasGauges) {
    Write-Host "✓ Valid Prometheus text format" -ForegroundColor Green
} else {
    Write-Host "✗ Invalid format (missing HELP/TYPE/gauge)" -ForegroundColor Red
}
Write-Host ""

# 3. Check specific metrics exist
Write-Host "[3] Checking for required metrics..." -ForegroundColor Yellow
$requiredMetrics = @(
    "vision_tok_supply",
    "vision_tok_burned_total",
    "vision_tok_vault_total",
    "vision_tok_fund_total",
    "vision_tok_treasury_total",
    "vision_blocks_height",
    "vision_mempool_len",
    "vision_peers_connected"
)

$missingMetrics = @()
foreach ($metric in $requiredMetrics) {
    if ($response -match "^$metric\s") {
        Write-Host "  ✓ $metric" -ForegroundColor Green
    } else {
        Write-Host "  ✗ $metric (missing)" -ForegroundColor Red
        $missingMetrics += $metric
    }
}
Write-Host ""

if ($missingMetrics.Count -eq 0) {
    Write-Host "✓ All required metrics present" -ForegroundColor Green
} else {
    Write-Host "✗ Missing metrics: $($missingMetrics -join ', ')" -ForegroundColor Red
}
Write-Host ""

# 4. Extract and display values
Write-Host "[4] Current metric values:" -ForegroundColor Yellow
$metrics = @{}
$lines | Where-Object { $_ -match "^vision_" -and $_ -notmatch "^#" } | ForEach-Object {
    if ($_ -match "^(vision_\w+)\s+(.+)$") {
        $name = $matches[1]
        $value = $matches[2]
        $metrics[$name] = $value
    }
}

Write-Host ""
Write-Host "Tokenomics:" -ForegroundColor Cyan
Write-Host "  Supply:    $(if ($metrics['vision_tok_supply']) { $metrics['vision_tok_supply'] } else { 'N/A' })"
Write-Host "  Burned:    $(if ($metrics['vision_tok_burned_total']) { $metrics['vision_tok_burned_total'] } else { 'N/A' })"
Write-Host "  Vault:     $(if ($metrics['vision_tok_vault_total']) { $metrics['vision_tok_vault_total'] } else { 'N/A' })"
Write-Host "  Fund:      $(if ($metrics['vision_tok_fund_total']) { $metrics['vision_tok_fund_total'] } else { 'N/A' })"
Write-Host "  Treasury:  $(if ($metrics['vision_tok_treasury_total']) { $metrics['vision_tok_treasury_total'] } else { 'N/A' })"
Write-Host ""
Write-Host "Operations:" -ForegroundColor Cyan
Write-Host "  Height:    $(if ($metrics['vision_blocks_height']) { $metrics['vision_blocks_height'] } else { 'N/A' })"
Write-Host "  Mempool:   $(if ($metrics['vision_mempool_len']) { $metrics['vision_mempool_len'] } else { 'N/A' })"
Write-Host "  Peers:     $(if ($metrics['vision_peers_connected']) { $metrics['vision_peers_connected'] } else { 'N/A' })"
Write-Host ""

# 5. Check Content-Type header
Write-Host "[5] Checking response headers..." -ForegroundColor Yellow
try {
    $headers = Invoke-WebRequest -Uri "$baseUrl/metrics" -Method Get
    $contentType = $headers.Headers['Content-Type']
    
    if ($contentType -match "text/plain") {
        Write-Host "✓ Correct Content-Type: $contentType" -ForegroundColor Green
    } else {
        Write-Host "✗ Unexpected Content-Type: $contentType" -ForegroundColor Red
    }
} catch {
    Write-Host "✗ Failed to check headers: $_" -ForegroundColor Red
}
Write-Host ""

# 6. Test Prometheus scrape compatibility
Write-Host "[6] Testing Prometheus compatibility..." -ForegroundColor Yellow
Write-Host "Sample prometheus.yml config:" -ForegroundColor White
Write-Host @"
scrape_configs:
  - job_name: 'vision-node'
    scrape_interval: 15s
    static_configs:
      - targets: ['127.0.0.1:7070']
    metrics_path: /metrics
"@ -ForegroundColor Gray
Write-Host ""

Write-Host "=== Test Complete ===" -ForegroundColor Cyan
Write-Host ""
Write-Host "Next steps:" -ForegroundColor Yellow
Write-Host "1. Set up Prometheus to scrape this endpoint"
Write-Host "2. Create Grafana dashboard with these metrics"
Write-Host "3. Populate tokenomics tree in sled (see docs/PROMETHEUS_METRICS.md)"
Write-Host "4. Set up alerting rules for critical metrics"
Write-Host ""

# 7. Save metrics to file for inspection
$outputFile = "metrics-output-$(Get-Date -Format 'yyyyMMdd-HHmmss').txt"
$response | Out-File -FilePath $outputFile -Encoding UTF8
Write-Host "Metrics saved to: $outputFile" -ForegroundColor Green
Write-Host ""
