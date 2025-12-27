# Mock Bootstrap Server for Vision Identity Testing
# PowerShell implementation - no Python required
#
# Usage: .\scripts\mock-bootstrap-server.ps1
# Then set: $env:VISION_BOOTSTRAP_URL="http://localhost:8888/api/bootstrap/handshake"

$port = 8888
$url = "http://localhost:$port/"

Write-Host "🎫 Mock Bootstrap Server starting on http://localhost:$port" -ForegroundColor Green
Write-Host "   Endpoint: POST /api/bootstrap/handshake" -ForegroundColor Cyan
Write-Host ""
Write-Host "To use with Vision Node, set:" -ForegroundColor Yellow
Write-Host '   $env:VISION_BOOTSTRAP_URL="http://localhost:' -NoNewline -ForegroundColor White
Write-Host "$port" -NoNewline -ForegroundColor Cyan
Write-Host '/api/bootstrap/handshake"' -ForegroundColor White
Write-Host ""
Write-Host "Press Ctrl+C to stop" -ForegroundColor Gray
Write-Host ""

# Create HTTP listener
$listener = New-Object System.Net.HttpListener
$listener.Prefixes.Add($url)
$listener.Start()

function Generate-NodeTag {
    $chars = "ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789"
    $part1 = -join ((1..4) | ForEach-Object { $chars[(Get-Random -Maximum $chars.Length)] })
    $part2 = -join ((1..4) | ForEach-Object { Get-Random -Maximum 10 })
    return "VNODE-$part1-$part2"
}

function Encode-Base64Url {
    param([string]$text)
    $bytes = [System.Text.Encoding]::UTF8.GetBytes($text)
    $base64 = [Convert]::ToBase64String($bytes)
    return $base64.Replace('+', '-').Replace('/', '_').TrimEnd('=')
}

function Create-MockJWT {
    param(
        [string]$nodeTag,
        [string]$networkId,
        [string]$publicKey,
        [string]$role
    )
    
    $now = [DateTimeOffset]::UtcNow.ToUnixTimeSeconds()
    $expires = $now + (30 * 24 * 60 * 60)  # 30 days
    
    $header = @{
        alg = "HS256"
        typ = "JWT"
    } | ConvertTo-Json -Compress
    
    $payload = @{
        node_tag = $nodeTag
        network_id = $networkId
        public_key = $publicKey
        role = $role
        iat = $now
        exp = $expires
    } | ConvertTo-Json -Compress
    
    $headerB64 = Encode-Base64Url $header
    $payloadB64 = Encode-Base64Url $payload
    $signature = Encode-Base64Url "mock_signature_for_testing"
    
    return "$headerB64.$payloadB64.$signature"
}

try {
    while ($listener.IsListening) {
        $context = $listener.GetContext()
        $request = $context.Request
        $response = $context.Response
        
        if ($request.HttpMethod -eq "POST" -and $request.Url.AbsolutePath -eq "/api/bootstrap/handshake") {
            # Read request body
            $reader = New-Object System.IO.StreamReader($request.InputStream)
            $body = $reader.ReadToEnd()
            $reader.Close()
            
            $requestData = $body | ConvertFrom-Json
            
            Write-Host "[BOOTSTRAP] Received handshake request:" -ForegroundColor Cyan
            Write-Host "  Public Key: $($requestData.public_key.Substring(0, [Math]::Min(20, $requestData.public_key.Length)))..." -ForegroundColor Gray
            Write-Host "  Network ID: $($requestData.network_id)" -ForegroundColor Gray
            Write-Host "  Version: $($requestData.version)" -ForegroundColor Gray
            Write-Host "  Role: $($requestData.role)" -ForegroundColor Gray
            Write-Host "  Address: $($requestData.address)" -ForegroundColor Gray
            
            # Generate node tag
            $nodeTag = Generate-NodeTag
            
            # Create JWT admission ticket
            $admissionTicket = Create-MockJWT `
                -nodeTag $nodeTag `
                -networkId $requestData.network_id `
                -publicKey $requestData.public_key `
                -role $requestData.role
            
            # Calculate expiration
            $expiresAt = (Get-Date).AddDays(30).ToUniversalTime().ToString("yyyy-MM-ddTHH:mm:ssZ")
            
            # Build response
            $responseData = @{
                node_tag = $nodeTag
                admission_ticket = $admissionTicket
                network_id = $requestData.network_id
                expires_at = $expiresAt
            } | ConvertTo-Json
            
            Write-Host ""
            Write-Host "[BOOTSTRAP] ✅ Issuing identity:" -ForegroundColor Green
            Write-Host "  Node Tag: $nodeTag" -ForegroundColor White
            Write-Host "  Ticket: $($admissionTicket.Substring(0, [Math]::Min(40, $admissionTicket.Length)))..." -ForegroundColor Gray
            Write-Host "  Expires: $expiresAt" -ForegroundColor Gray
            Write-Host ""
            
            # Send response
            $buffer = [System.Text.Encoding]::UTF8.GetBytes($responseData)
            $response.ContentLength64 = $buffer.Length
            $response.ContentType = "application/json"
            $response.StatusCode = 200
            $response.OutputStream.Write($buffer, 0, $buffer.Length)
            $response.Close()
        } else {
            # 404 for other paths
            $response.StatusCode = 404
            $response.Close()
        }
    }
} catch {
    Write-Host "Error: $_" -ForegroundColor Red
} finally {
    if ($listener.IsListening) {
        $listener.Stop()
    }
    Write-Host ""
    Write-Host "👋 Mock server stopped" -ForegroundColor Yellow
}
