#!/usr/bin/env python3
"""
Asset Loading Debug Script

This script helps debug asset loading issues by:
1. Testing the collector's embedded web server
2. Checking individual asset endpoints  
3. Verifying frontend build artifacts
4. Testing the integrated server-frontend flow
"""

import requests
import json
import time
import subprocess
import os
import sys
from pathlib import Path

class AssetDebugger:
    def __init__(self, base_url="http://localhost:8080"):
        self.base_url = base_url
        self.session = requests.Session()
        self.results = []
        
    def log(self, message, status="INFO"):
        timestamp = time.strftime("%Y-%m-%d %H:%M:%S")
        log_msg = f"[{timestamp}] {status}: {message}"
        print(log_msg)
        self.results.append(log_msg)
        
    def test_server_health(self):
        """Test if the server is running and responsive"""
        self.log("Testing server health...")
        try:
            response = self.session.get(f"{self.base_url}/health", timeout=5)
            if response.status_code == 200:
                self.log("Server health check passed", "SUCCESS")
                return True
            else:
                self.log(f"Server health check failed: {response.status_code}", "ERROR")
                return False
        except requests.exceptions.RequestException as e:
            self.log(f"Server is not responding: {e}", "ERROR")
            return False
    
    def test_root_endpoints(self):
        """Test various root endpoints"""
        endpoints = ["/", "/index.html", "/timeline", "/timeline/"]
        
        self.log("Testing root endpoints...")
        for endpoint in endpoints:
            try:
                response = self.session.get(f"{self.base_url}{endpoint}", timeout=10)
                if response.status_code == 200:
                    content_type = response.headers.get('content-type', 'unknown')
                    self.log(f"✓ {endpoint} -> {response.status_code} ({content_type}, {len(response.content)} bytes)", "SUCCESS")
                elif response.status_code == 404:
                    self.log(f"✗ {endpoint} -> {response.status_code} (Asset not found)", "ERROR")
                else:
                    self.log(f"? {endpoint} -> {response.status_code}", "WARNING")
            except requests.exceptions.RequestException as e:
                self.log(f"✗ {endpoint} -> Exception: {e}", "ERROR")
    
    def test_static_assets(self):
        """Test common static assets"""
        static_assets = [
            "/_next/static/css/app.css",
            "/_next/static/js/app.js", 
            "/_next/static/chunks/main.js",
            "/_next/static/chunks/webpack.js",
            "/favicon.ico"
        ]
        
        self.log("Testing static assets...")
        for asset in static_assets:
            try:
                response = self.session.get(f"{self.base_url}{asset}", timeout=10)
                if response.status_code == 200:
                    content_type = response.headers.get('content-type', 'unknown')
                    self.log(f"✓ {asset} -> {response.status_code} ({content_type}, {len(response.content)} bytes)", "SUCCESS")
                elif response.status_code == 404:
                    self.log(f"✗ {asset} -> {response.status_code} (Not found)", "ERROR")
                else:
                    self.log(f"? {asset} -> {response.status_code}", "WARNING")
            except requests.exceptions.RequestException as e:
                self.log(f"✗ {asset} -> Exception: {e}", "ERROR")
    
    def test_api_endpoints(self):
        """Test API endpoints"""
        api_endpoints = ["/api/health", "/api/status", "/events"]
        
        self.log("Testing API endpoints...")
        for endpoint in api_endpoints:
            try:
                response = self.session.get(f"{self.base_url}{endpoint}", timeout=5)
                if response.status_code == 200:
                    self.log(f"✓ {endpoint} -> {response.status_code}", "SUCCESS")
                elif response.status_code == 404:
                    self.log(f"✗ {endpoint} -> {response.status_code} (Not implemented)", "INFO")
                else:
                    self.log(f"? {endpoint} -> {response.status_code}", "WARNING")
            except requests.exceptions.RequestException as e:
                self.log(f"✗ {endpoint} -> Exception: {e}", "ERROR")
    
    def check_frontend_build(self):
        """Check if frontend is properly built"""
        self.log("Checking frontend build artifacts...")
        
        frontend_dir = Path(__file__).parent.parent / "frontend"
        build_dir = frontend_dir / ".next"
        
        if not frontend_dir.exists():
            self.log("Frontend directory not found", "ERROR")
            return False
            
        if not build_dir.exists():
            self.log("Frontend not built (.next directory missing)", "ERROR")
            self.log("Run: cd frontend && npm run build", "INFO")
            return False
            
        self.log("Frontend build directory exists", "SUCCESS")
        
        # Check for key build artifacts
        static_dir = build_dir / "static"
        if static_dir.exists():
            static_files = list(static_dir.rglob("*"))
            self.log(f"Found {len(static_files)} static files in build", "INFO")
        else:
            self.log("No static directory in build", "WARNING")
            
        return True
    
    def test_collector_server(self):
        """Test if collector server is running properly"""
        self.log("Testing collector server process...")
        
        try:
            # Check if collector server process is running
            result = subprocess.run(
                ["pgrep", "-f", "collector.*server"], 
                capture_output=True, 
                text=True
            )
            
            if result.returncode == 0:
                pids = result.stdout.strip().split('\n')
                self.log(f"Found collector server process(es): {', '.join(pids)}", "SUCCESS")
            else:
                self.log("No collector server process found", "WARNING")
                self.log("Start with: cd collector && cargo run server", "INFO")
                
        except Exception as e:
            self.log(f"Error checking collector process: {e}", "ERROR")
    
    def run_comprehensive_test(self):
        """Run all tests"""
        self.log("Starting comprehensive asset debug test...")
        
        # Pre-checks
        self.check_frontend_build()
        self.test_collector_server()
        
        # Server tests
        if not self.test_server_health():
            self.log("Server is not responding. Start it with: cd collector && cargo run server", "ERROR")
            return False
            
        # Asset tests
        self.test_root_endpoints()
        self.test_static_assets()
        self.test_api_endpoints()
        
        self.log("Comprehensive test completed")
        return True
        
    def save_results(self, filename="asset_debug_results.txt"):
        """Save test results to file"""
        output_path = Path(__file__).parent / filename
        with open(output_path, 'w') as f:
            f.write("Asset Loading Debug Results\n")
            f.write("=" * 40 + "\n\n")
            for result in self.results:
                f.write(result + "\n")
        
        self.log(f"Results saved to {output_path}")

def main():
    print("Asset Loading Debug Script")
    print("=" * 40)
    
    # Parse command line arguments
    server_url = "http://localhost:8080"
    if len(sys.argv) > 1:
        server_url = sys.argv[1]
    
    debugger = AssetDebugger(server_url)
    
    try:
        success = debugger.run_comprehensive_test()
        debugger.save_results()
        
        if success:
            print("\n" + "=" * 40)
            print("Debug test completed. Check the results above.")
            print("If assets are failing, try:")
            print("1. cd frontend && npm run build")
            print("2. cd collector && cargo run server")
            print("3. Check http://localhost:8080/timeline")
        else:
            print("\n" + "=" * 40)
            print("Debug test found issues. Check the log above.")
            
    except KeyboardInterrupt:
        print("\nTest interrupted by user")
    except Exception as e:
        print(f"\nUnexpected error: {e}")

if __name__ == "__main__":
    main()