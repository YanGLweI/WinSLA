// Mock API Server for Frontend Testing
const express = require('express');
const cors = require('cors');
const path = require('path');

const app = express();
app.use(cors());
app.use(express.json());

// In-memory storage for testing
let accounts = [];
let authStats = {
    connections_accepted: 0,
    successful_auths: 0,
    failed_auths: 0
};

// Status endpoint
app.get('/api/status', (req, res) => {
    authStats.connections_accepted++;
    res.json({
        running: true,
        version: "v2.2.6",
        ...authStats
    });
});

// Get accounts
app.get('/api/accounts', (req, res) => {
    authStats.connections_accepted++;
    res.json(accounts);
});

// Create account
app.post('/api/accounts', (req, res) => {
    authStats.connections_accepted++;
    const newAccount = {
        id: Date.now().toString(),
        account_sid: req.body.account_sid || `S-1-5-21-${Date.now()}`,
        account_username: req.body.account_username || 'TestUser',
        approvers: [],
        enabled: true,
        created_at: new Date().toISOString(),
        updated_at: new Date().toISOString()
    };
    accounts.push(newAccount);
    res.status(201).json(newAccount);
});

// Add approver
app.post('/api/accounts/approvers', (req, res) => {
    authStats.connections_accepted++;
    const { account_sid, approver_sid, approver_username } = req.body;
    
    const account = accounts.find(a => a.account_sid === account_sid);
    if (!account) {
        return res.status(404).json({ error: 'Account not found' });
    }
    
    // Check if approver already exists
    const existingApprover = account.approvers.find(a => a.sid === approver_sid);
    if (existingApprover) {
        return res.status(409).json({ error: 'Approver already exists' });
    }
    
    account.approvers.push({
        sid: approver_sid,
        username: approver_username,
        enabled: true
    });
    account.updated_at = new Date().toISOString();
    
    res.status(201).send();
});

// Remove approver
app.delete('/api/accounts/:accountSid/approvers/:approverSid', (req, res) => {
    authStats.connections_accepted++;
    const { accountSid, approverSid } = req.params;
    
    const account = accounts.find(a => a.account_sid === accountSid);
    if (!account) {
        return res.status(404).json({ error: 'Account not found' });
    }
    
    const initialLength = account.approvers.length;
    account.approvers = account.approvers.filter(a => a.sid !== approverSid);
    
    if (account.approvers.length === initialLength) {
        return res.status(404).json({ error: 'Approver not found' });
    }
    
    account.updated_at = new Date().toISOString();
    res.status(204).send();
});

// Toggle account enabled
app.put('/api/accounts/:accountSid/enable', (req, res) => {
    authStats.connections_accepted++;
    const { accountSid } = req.params;
    const { enabled } = req.body;
    
    const account = accounts.find(a => a.account_sid === accountSid);
    if (!account) {
        return res.status(404).json({ error: 'Account not found' });
    }
    
    account.enabled = enabled;
    account.updated_at = new Date().toISOString();
    
    res.json({ account_sid: accountSid, enabled });
});

// Delete account
app.delete('/api/accounts/:accountSid', (req, res) => {
    authStats.connections_accepted++;
    const { accountSid } = req.params;
    
    const initialLength = accounts.length;
    accounts = accounts.filter(a => a.account_sid !== accountSid);
    
    if (accounts.length === initialLength) {
        return res.status(404).json({ error: 'Account not found' });
    }
    
    res.status(204).send();
});

// Validate account (mock)
app.post('/api/validate-account', (req, res) => {
    authStats.connections_accepted++;
    const { username, password } = req.body;
    
    // Accept any credentials for testing
    res.json({
        success: true,
        sid: `S-1-5-21-mock-${Date.now()}`,
        display_name: username,
        message: `验证成功：${username}`
    });
});

const PORT = 19830;
app.listen(PORT, () => {
    console.log(`Mock API Server running on http://localhost:${PORT}`);
    console.log('Available endpoints:');
    console.log('  GET  /api/status          - Service status');
    console.log('  GET  /api/accounts        - List all accounts');
    console.log('  POST /api/accounts        - Create account');
    console.log('  POST /api/accounts/approvers - Add approver');
    console.log('  DELETE /api/accounts/:sid/approvers/:approverId - Remove approver');
    console.log('  PUT  /api/accounts/:id/enable     - Toggle enable/disable');
    console.log('  DELETE /api/accounts/:id      - Delete account');
    console.log('  POST /api/validate-account  - Validate credentials');
});
