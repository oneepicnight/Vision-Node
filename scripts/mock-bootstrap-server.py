#!/usr/bin/env python3
"""
Mock Bootstrap Server for Vision Identity Testing

Simulates the Vision website's /api/bootstrap/handshake endpoint
for local testing of the Vision Identity system.

Usage:
    python scripts/mock-bootstrap-server.py

Then set: $env:VISION_BOOTSTRAP_URL="http://localhost:8888/api/bootstrap/handshake"
"""

from http.server import HTTPServer, BaseHTTPRequestHandler
import json
import base64
import time
import random
import string

class BootstrapHandler(BaseHTTPRequestHandler):
    def do_POST(self):
        if self.path == '/api/bootstrap/handshake':
            # Read request body
            content_length = int(self.headers['Content-Length'])
            body = self.rfile.read(content_length)
            request_data = json.loads(body)
            
            print(f"\n[BOOTSTRAP] Received handshake request:")
            print(f"  Public Key: {request_data.get('public_key', 'N/A')[:20]}...")
            print(f"  Network ID: {request_data.get('network_id')}")
            print(f"  Version: {request_data.get('version')}")
            print(f"  Role: {request_data.get('role')}")
            print(f"  Address: {request_data.get('address')}")
            
            # Generate node tag
            node_tag = f"VNODE-{''.join(random.choices(string.ascii_uppercase + string.digits, k=4))}-{''.join(random.choices(string.digits, k=4))}"
            
            # Create JWT-style admission ticket
            header = {
                "alg": "HS256",
                "typ": "JWT"
            }
            
            payload = {
                "node_tag": node_tag,
                "network_id": request_data.get('network_id'),
                "public_key": request_data.get('public_key'),
                "role": request_data.get('role'),
                "iat": int(time.time()),
                "exp": int(time.time()) + (30 * 24 * 60 * 60),  # 30 days
            }
            
            # Encode JWT (mock signature - not cryptographically secure)
            header_b64 = base64.urlsafe_b64encode(json.dumps(header).encode()).decode().rstrip('=')
            payload_b64 = base64.urlsafe_b64encode(json.dumps(payload).encode()).decode().rstrip('=')
            signature_b64 = base64.urlsafe_b64encode(b"mock_signature_for_testing").decode().rstrip('=')
            
            admission_ticket = f"{header_b64}.{payload_b64}.{signature_b64}"
            
            # Build response
            response = {
                "node_tag": node_tag,
                "admission_ticket": admission_ticket,
                "network_id": request_data.get('network_id'),
                "expires_at": time.strftime('%Y-%m-%dT%H:%M:%SZ', time.gmtime(payload['exp']))
            }
            
            print(f"\n[BOOTSTRAP] ✅ Issuing identity:")
            print(f"  Node Tag: {node_tag}")
            print(f"  Ticket: {admission_ticket[:40]}...")
            print(f"  Expires: {response['expires_at']}")
            
            # Send response
            self.send_response(200)
            self.send_header('Content-Type', 'application/json')
            self.end_headers()
            self.wfile.write(json.dumps(response).encode())
        else:
            self.send_error(404)
    
    def log_message(self, format, *args):
        # Suppress default request logging
        pass

def run_server(port=8888):
    server = HTTPServer(('localhost', port), BootstrapHandler)
    print(f"🎫 Mock Bootstrap Server running on http://localhost:{port}")
    print(f"   Endpoint: POST /api/bootstrap/handshake")
    print(f"\nTo use with Vision Node, set:")
    print(f'   PowerShell: $env:VISION_BOOTSTRAP_URL="http://localhost:{port}/api/bootstrap/handshake"')
    print(f'   Bash: export VISION_BOOTSTRAP_URL="http://localhost:{port}/api/bootstrap/handshake"')
    print(f"\nPress Ctrl+C to stop\n")
    
    try:
        server.serve_forever()
    except KeyboardInterrupt:
        print("\n\n👋 Shutting down mock server...")
        server.shutdown()

if __name__ == '__main__':
    run_server()
