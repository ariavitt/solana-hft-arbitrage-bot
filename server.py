#!/usr/bin/env python3
"""
Simple HTTP server with CORS proxy for metrics
Serves dashboard and proxies requests to bot metrics
"""

import http.server
import socketserver
import urllib.request
import json
from urllib.error import URLError

PORT = 8080
METRICS_URL = "http://localhost:9090/metrics"

class CORSRequestHandler(http.server.SimpleHTTPRequestHandler):
    def end_headers(self):
        self.send_header('Access-Control-Allow-Origin', '*')
        self.send_header('Access-Control-Allow-Methods', 'GET, OPTIONS')
        self.send_header('Access-Control-Allow-Headers', '*')
        super().end_headers()
    
    def do_OPTIONS(self):
        self.send_response(200)
        self.end_headers()
    
    def do_GET(self):
        # Proxy /metrics to bot
        if self.path == '/api/metrics':
            try:
                with urllib.request.urlopen(METRICS_URL, timeout=5) as response:
                    data = response.read().decode('utf-8')
                    self.send_response(200)
                    self.send_header('Content-Type', 'text/plain')
                    self.end_headers()
                    self.wfile.write(data.encode())
            except URLError as e:
                self.send_response(503)
                self.send_header('Content-Type', 'application/json')
                self.end_headers()
                self.wfile.write(json.dumps({"error": str(e)}).encode())
            except Exception as e:
                self.send_response(500)
                self.send_header('Content-Type', 'application/json')
                self.end_headers()
                self.wfile.write(json.dumps({"error": str(e)}).encode())
        else:
            super().do_GET()

if __name__ == "__main__":
    with socketserver.TCPServer(("", PORT), CORSRequestHandler) as httpd:
        print(f"🚀 Dashboard server running at http://localhost:{PORT}")
        print(f"📊 Open: http://localhost:{PORT}/dashboard.html")
        print(f"📡 Metrics proxy: http://localhost:{PORT}/api/metrics")
        print(f"\nPress Ctrl+C to stop")
        try:
            httpd.serve_forever()
        except KeyboardInterrupt:
            print("\n👋 Server stopped")



