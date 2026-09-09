#!/usr/bin/env python3
"""Mock API Server for WinSLA Frontend Testing"""

from flask import Flask, jsonify, request, make_response
from datetime import datetime

app = Flask(__name__)

# In-memory storage for testing
accounts = []
auth_stats = {
    'connections_accepted': 0,
    'successful_auths': 0,
    'failed_auths': 0
}

@app.route('/api/status', methods=['GET'])
def status():
    auth_stats['connections_accepted'] += 1
    return jsonify({
        'running': True,
        'version': 'v2.2.6',
        **auth_stats
    })

@app.route('/api/accounts', methods=['GET'])
def get_accounts():
    auth_stats['connections_accepted'] += 1
    return jsonify(accounts)

@app.route('/api/accounts', methods=['POST'])
def create_account():
    auth_stats['connections_accepted'] += 1
    data = request.json
    
    new_account = {
        'id': str(datetime.now().timestamp()),
        'account_sid': data.get('account_sid', f'S-1-5-21-{datetime.now().timestamp()}'),
        'account_username': data.get('account_username', 'TestUser'),
        'approvers': [],
        'enabled': True,
        'created_at': datetime.now().isoformat(),
        'updated_at': datetime.now().isoformat()
    }
    accounts.append(new_account)
    return jsonify(new_account), 201

@app.route('/api/accounts/approvers', methods=['POST'])
def add_approver():
    auth_stats['connections_accepted'] += 1
    data = request.json
    
    account_sid = data.get('account_sid')
    approver_sid = data.get('approver_sid')
    approver_username = data.get('approver_username')
    
    account = next((a for a in accounts if a['account_sid'] == account_sid), None)
    if not account:
        return jsonify({'error': 'Account not found'}), 404
    
    # Check if approver already exists
    existing_approver = next((a for a in account['approvers'] if a['sid'] == approver_sid), None)
    if existing_approver:
        return jsonify({'error': 'Approver already exists'}), 409
    
    account['approvers'].append({
        'sid': approver_sid,
        'username': approver_username,
        'enabled': True
    })
    account['updated_at'] = datetime.now().isoformat()
    
    return '', 201

@app.route('/api/accounts/<account_sid>/approvers/<approver_sid>', methods=['DELETE'])
def remove_approver(account_sid, approver_sid):
    auth_stats['connections_accepted'] += 1
    
    account = next((a for a in accounts if a['account_sid'] == account_sid), None)
    if not account:
        return jsonify({'error': 'Account not found'}), 404
    
    initial_length = len(account['approvers'])
    account['approvers'] = [a for a in account['approvers'] if a['sid'] != approver_sid]
    
    if len(account['approvers']) == initial_length:
        return jsonify({'error': 'Approver not found'}), 404
    
    account['updated_at'] = datetime.now().isoformat()
    return '', 204

@app.route('/api/accounts/<account_sid>/enable', methods=['PUT'])
def toggle_account_enabled(account_sid):
    auth_stats['connections_accepted'] += 1
    data = request.json
    
    account = next((a for a in accounts if a['account_sid'] == account_sid), None)
    if not account:
        return jsonify({'error': 'Account not found'}), 404
    
    account['enabled'] = data.get('enabled', False)
    account['updated_at'] = datetime.now().isoformat()
    
    return jsonify({'account_sid': account_sid, 'enabled': account['enabled']})

@app.route('/api/accounts/<account_sid>', methods=['DELETE'])
def delete_account(account_sid):
    auth_stats['connections_accepted'] += 1
    
    initial_length = len(accounts)
    accounts[:] = [a for a in accounts if a['account_sid'] != account_sid]
    
    if len(accounts) == initial_length:
        return jsonify({'error': 'Account not found'}), 404
    
    return '', 204

@app.route('/api/validate-account', methods=['POST'])
def validate_account():
    auth_stats['connections_accepted'] += 1
    data = request.json
    
    # Accept any credentials for testing
    return jsonify({
        'success': True,
        'sid': f'S-1-5-21-mock-{datetime.now().timestamp()}',
        'display_name': data.get('username', 'TestUser'),
        'message': f'验证成功：{data.get("username", "TestUser")}'
    })

if __name__ == '__main__':
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
    print('Server running on http://localhost:19830')
    print('=' * 60)
    app.run(host='0.0.0.0', port=19830, debug=False)
