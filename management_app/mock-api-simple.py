#!/usr/bin/env python3
"""Mock API Server for WinSLA Frontend Testing (no external dependencies)"""

from http.server import HTTPServer, BaseHTTPRequestHandler
import json
from datetime import datetime
from urllib.parse import urlparse, parse_qs

# In-memory storage for testing
accounts = []
auth_stats = {
    'connections_accepted': 0,
    'successful_auths': 0,
    'failed_auths': 0
}

class MockAPIHandler(BaseHTTPRequestHandler):
    def send_json(self, data, status=200):
        self.send_response(status)
        self.send_header('Content-Type', 'application/json')
        self.end_headers()
        self.wfile.write(json.dumps(data).encode())
    
    def read_body(self):
        content_length = int(self.headers.get('Content-Length', 0))
        if content_length > 0:
            return json.loads(self.rfile.read(content_length).decode())
        return {}
    
    def do_GET(self):
        auth_stats['connections_accepted'] += 1
        path = urlparse(self.path).path
        
        print(f"GET {self.path}")
        
        if path == '/api/status':
            response = {
                'running': True,
                'version': 'v2.2.6',
                **auth_stats
            }
            self.send_json(response)
        
        elif path == '/api/accounts':
            self.send_json(accounts)
        
        else:
            self.send_json({'error': 'Not found'}, 404)
    
    def do_POST(self):
        auth_stats['connections_accepted'] += 1
        path = urlparse(self.path).path
        body = self.read_body()
        
        print(f"POST {self.path}")
        
        if path == '/api/accounts':
            new_account = {
                'id': str(datetime.now().timestamp()),
                'account_sid': body.get('account_sid', f'S-1-5-21-{datetime.now().timestamp()}'),
                'account_username': body.get('account_username', 'TestUser'),
                'approvers': [],
                'enabled': True,
                'created_at': datetime.now().isoformat(),
                'updated_at': datetime.now().isoformat()
            }
            accounts.append(new_account)
            self.send_json(new_account, 201)
        
        elif path == '/api/accounts/approvers':
            account_sid = body.get('account_sid')
            approver_sid = body.get('approver_sid')
            approver_username = body.get('approver_username')
            
            account = next((a for a in accounts if a['account_sid'] == account_sid), None)
            if not account:
                self.send_json({'error': 'Account not found'}, 404)
                return
            
            existing_approver = next((a for a in account['approvers'] if a['sid'] == approver_sid), None)
            if existing_approver:
                self.send_json({'error': 'Approver already exists'}, 409)
                return
            
            account['approvers'].append({
                'sid': approver_sid,
                'username': approver_username,
                'enabled': True
            })
            account['updated_at'] = datetime.now().isoformat()
            
            self.send_response(201)
            self.end_headers()
        
        elif path == '/api/validate-account':
            response = {
                'success': True,
                'sid': f'S-1-5-21-mock-{datetime.now().timestamp()}',
                'display_name': body.get('username', 'TestUser'),
                'message': f'验证成功：{body.get("username", "TestUser")}'
            }
            self.send_json(response)
        
        else:
            self.send_json({'error': 'Not found'}, 404)
    
    def do_DELETE(self):
        auth_stats['connections_accepted'] += 1
        path = urlparse(self.path).path
        
        # Parse /api/accounts/<account_sid>/approvers/<approver_sid>
        parts = path.split('/')
        if len(parts) >= 6 and parts[1] == 'api' and parts[2] == 'accounts':
            account_sid = parts[3]
            if len(parts) >= 7 and parts[4] == 'approvers':
                approver_sid = parts[5]
                
                account = next((a for a in accounts if a['account_sid'] == account_sid), None)
                if not account:
                    self.send_json({'error': 'Account not found'}, 404)
                    return
                
                initial_length = len(account['approvers'])
                account['approvers'] = [a for a in account['approvers'] if a['sid'] != approver_sid]
                
                if len(account['approvers']) == initial_length:
                    self.send_json({'error': 'Approver not found'}, 404)
                    return
                
                account['updated_at'] = datetime.now().isoformat()
                self.send_response(204)
                self.end_headers()
                return
            
            elif len(parts) == 4 and parts[3]:
                account_sid = parts[3]
                initial_length = len(accounts)
                accounts[:] = [a for a in accounts if a['account_sid'] != account_sid]
                
                if len(accounts) == initial_length:
                    self.send_json({'error': 'Account not found'}, 404)
                    return
                
                self.send_response(204)
                self.end_headers()
                return
        
        self.send_json({'error': 'Not found'}, 404)
    
    def do_PUT(self):
        auth_stats['connections_accepted'] += 1
        path = urlparse(self.path).path
        body = self.read_body()
        
        # Parse /api/accounts/<account_sid>/enable
        parts = path.split('/')
        if len(parts) >= 5 and parts[1] == 'api' and parts[2] == 'accounts' and parts[4] == 'enable':
            account_sid = parts[3]
            
            account = next((a for a in accounts if a['account_sid'] == account_sid), None)
            if not account:
                self.send_json({'error': 'Account not found'}, 404)
                return
            
            account['enabled'] = body.get('enabled', False)
            account['updated_at'] = datetime.now().isoformat()
            
            self.send_json({
                'account_sid': account_sid,
                'enabled': account['enabled']
            })
        else:
            self.send_json({'error': 'Not found'}, 404)
    
    def log_message(self, format, *args):
        # Suppress default logging
        pass

if __name__ == '__main__':
    PORT = 19830
    server_address = ('', PORT)
    httpd = HTTPServer(server_address, MockAPIHandler)
    
    print('=' * 60)
    print('Mock API Server for WinSLA v2.2.6 Testing')
    print('=' * 60)
    print('Available endpoints:')
    print('  GET  /api/status          - Service status')
    print('  GET  /api/accounts        - List all accounts')
    print('  POST /api/accounts        - Create account')
    print('  POST /api/accounts/approvers - Add approver')
    print('  DELETE /api/accounts/:id/approvers/:approverId - Remove approver')
    print('  PUT  /api/accounts/:id/enable     - Toggle enable/disable')
    print('  DELETE /api/accounts/:id      - Delete account')
    print('  POST /api/validate-account  - Validate credentials')
    print('=' * 60)
    print(f'Server running on http://localhost:{PORT}')
    print('=' * 60)
    
    try:
        httpd.serve_forever()
    except KeyboardInterrupt:
        print('\nShutting down server...')
        httpd.shutdown()
