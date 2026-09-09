# WinSLA v2.2.6 Quick Test Script

This script will install and start the WinSLA service with proper permissions.

```powershell
param(
    [switch]$Install,
    [switch]$Start,
    [switch]$TestAPI
)

$servicePath = "C:\Program Files\WinSLA\winsla-service.exe"

if ($Install.IsPresent) {
    Write-Host "=== Installing WinSLA Service ===" -ForegroundColor Cyan
    
    # Copy service exe to Program Files (requires admin)
    if (-not (Test-Path $servicePath)) {
        Write-Host "Copying service executable..." -ForegroundColor Yellow
        Copy-Item "..\target\release\winsla-service.exe" -Destination $servicePath -Force
    }
    
    # Create service
    Write-Host "Creating Windows Service..." -ForegroundColor Yellow
    sc create "WinSLA Service" binPath="`"$servicePath`" --service" start= auto
    
    Write-Host "Installation complete!" -ForegroundColor Green
}

if ($Start.IsPresent) {
    Write-Host "=== Starting WinSLA Service ===" -ForegroundColor Cyan
    
    try {
        sc start "WinSLA Service"
        Write-Host "✓ Service started successfully!" -ForegroundColor Green
    } catch {
        Write-Host "✗ Failed to start service" -ForegroundColor Red
        exit 1
    }
}

if ($TestAPI.IsPresent) {
    Write-Host "`n=== Testing API Endpoints ===" -ForegroundColor Cyan
    
    $apiUrl = "http://localhost:19830"
    
    Write-Host "`nChecking /api/status..." -ForegroundColor Yellow
    try {
        $status = Invoke-RestMethod -Uri "$apiUrl/api/status" -TimeoutSec 5 -ErrorAction Stop
        Write-Host "✓ Status: $($status.running)" -ForegroundColor Green
        Write-Host "  Connections: $($status.connections_accepted)" -ForegroundColor Gray
        Write-Host "  Successful Auths: $($status.successful_auths)" -ForegroundColor Gray
        Write-Host "  Failed Auths: $($status.failed_auths)" -ForegroundColor Gray
    } catch {
        Write-Host "✗ API not responding: $_" -ForegroundColor Red
    }
    
    Write-Host "`nChecking /api/accounts..." -ForegroundColor Yellow
    try {
        $accounts = Invoke-RestMethod -Uri "$apiUrl/api/accounts" -TimeoutSec 5 -ErrorAction Stop
        Write-Host "✓ Found $($accounts.Count) account(s)" -ForegroundColor Green
        
        if ($accounts.Count -gt 0) {
            Write-Host "`nAccount Details:" -ForegroundColor White
            foreach ($account in $accounts) {
                Write-Host "  - $($account.account_username) with $($account.approvers.Count) approver(s)" -ForegroundColor Gray
            }
        }
    } catch {
        Write-Host "✗ Failed to fetch accounts: $_" -ForegroundColor Red
    }
    
    Write-Host "`n✓ All API tests completed!" -ForegroundColor Green
}

Write-Host "`nTo run browser testing:" -ForegroundColor Cyan
Write-Host "1. Open browser at http://localhost:5174/#/pairs" -ForegroundColor White
Write-Host "2. Click '+ 新增主账号' button" -ForegroundColor White
Write-Host "3. Follow on-screen instructions" -ForegroundColor White
```

Usage:
```powershell
# Install service (runs as admin automatically)
.\test-script.ps1 -Install -Verb RunAs

# Start existing service
.\test-script.ps1 -Start

# Test API endpoints
.\test-script.ps1 -TestAPI

# Full setup
.\test-script.ps1 -Install; Start-Sleep -Seconds 3; .\test-script.ps1 -Start; .\test-script.ps1 -TestAPI
```
