#!/usr/bin/env python3

import http.server
import socketserver
import threading
import time
import urllib.request
import urllib.error

class SimpleHandler(http.server.BaseHTTPRequestHandler):
    def do_GET(self):
        if self.path == '/':
            self.send_response(200)
            self.send_header('Content-type', 'application/json')
            self.end_headers()
            response = '{"message": "Hello from Nanvix!", "status": "success"}'
            self.wfile.write(response.encode())
        else:
            self.send_response(404)
            self.end_headers()
    
    def log_message(self, format, *args):
        print(f"SERVER: {format % args}")

def start_server():
    PORT = 8080
    with socketserver.TCPServer(("localhost", PORT), SimpleHandler) as httpd:
        print(f"Server started on localhost:{PORT}")
        httpd.serve_forever()

def test_client():
    time.sleep(1)  # Give server time to start
    
    try:
        print("Making HTTP request to localhost:8080...")
        with urllib.request.urlopen('http://localhost:8080/') as response:
            content = response.read().decode()
            print(f"Response: {content}")
            
    except urllib.error.URLError as e:
        print(f"Request failed: {e}")
    except Exception as e:
        print(f"Error: {e}")

def main():
    print("Testing HTTP server and client in Nanvix...")
    
    try:
        # Start server in a separate thread
        server_thread = threading.Thread(target=start_server, daemon=True)
        server_thread.start()
        
        # Test client
        test_client()
        
        print("Test complete!")
        
    except Exception as e:
        print(f"Error: {e}")
        import traceback
        traceback.print_exc()

if __name__ == "__main__":
    main()