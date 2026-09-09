# WinSLA v2.2.6 Automated Test Script

Write-Host "=== WinSLA v2.2.6 One-to-Many Pairing Test ===" -ForegroundColor Cyan
Write-Host ""

# 测试配置
$baseUrl = "http://localhost:5173"
$apiUrl = "http://localhost:19830"

# 步骤 1: 检查开发服务器
Write-Host "[Step 1/4] Checking development server..." -ForegroundColor Yellow
try {
    $response = Invoke-WebRequest -Uri "$baseUrl" -TimeoutSec 5 -ErrorAction Stop
    Write-Host "  ✓ Development server is accessible at $baseUrl" -ForegroundColor Green
} catch {
    Write-Host "  ✗ Development server not accessible at $baseUrl" -ForegroundColor Red
    Write-Host "    Please start the dev server first: .\quick-start.ps1 -Dev" -ForegroundColor Gray
    exit 1
}

# 步骤 2: 检查后端 API
Write-Host "`n[Step 2/4] Checking backend API (port 19830)..." -ForegroundColor Yellow
try {
    $status = Invoke-WebRequest -Uri "$apiUrl/api/status" -TimeoutSec 5 -ErrorAction Stop | ConvertFrom-Json
    Write-Host "  ✓ Backend API is running" -ForegroundColor Green
    Write-Host "  Service Status: $($status.running)" -ForegroundColor White
    Write-Host "  Version: $($status.version)" -ForegroundColor White
    Write-Host "  Connections: $($status.connections_accepted)" -ForegroundColor White
    Write-Host "  Successful Auths: $($status.successful_auths)" -ForegroundColor White
    Write-Host "  Failed Auths: $($status.failed_auths)" -ForegroundColor White
} catch {
    Write-Host "  ⚠ Backend API not accessible at $apiUrl" -ForegroundColor Yellow
    Write-Host "    This might be expected if the service is not running." -ForegroundColor Gray
    Write-Host "    You can still test the UI functionality manually." -ForegroundColor Gray
}

# 步骤 3: 尝试获取配对记录
Write-Host "`n[Step 3/4] Fetching existing account pairs..." -ForegroundColor Yellow
try {
    $pairs = Invoke-WebRequest -Uri "$apiUrl/api/accounts" -TimeoutSec 5 -ErrorAction Stop | ConvertFrom-Json
    Write-Host "  ✓ Found $($pairs.Count) account(s) with approvers" -ForegroundColor Green
    
    if ($pairs.Count -gt 0) {
        Write-Host "`n  Account Details:" -ForegroundColor White
        foreach ($account in $pairs) {
            Write-Host "    - $($account.account_username) ($(($account.approvers | Measure-Object).Count) approver(s))" -ForegroundColor Gray
        }
    }
} catch {
    Write-Host "  ℹ No data available or API error (expected during initial testing)" -ForegroundColor Gray
}

# 步骤 4: 显示测试指引
Write-Host "`n[Step 4/4] Manual Testing Instructions" -ForegroundColor Yellow
Write-Host "`nPlease follow these steps to test the one-to-many pairing feature:" -ForegroundColor Cyan
Write-Host ""
Write-Host "1. Open your web browser and navigate to: $baseUrl" -ForegroundColor White
Write-Host "   (or http://127.0.0.1:5173)" -ForegroundColor Gray
Write-Host ""
Write-Host "2. Click the '+ 新增主账号' button to open the dialog" -ForegroundColor White
Write-Host ""
Write-Host "3. Fill in and validate the account credentials:" -ForegroundColor White
Write-Host "   - Username: Enter your domain account" -ForegroundColor Gray
Write-Host "   - Password: Enter the password" -ForegroundColor Gray
Write-Host "   - Click '验证主账号' button" -ForegroundColor Gray
Write-Host ""
Write-Host "4. Check for the green success tag:" -ForegroundColor White
Write-Host "   [✓] Should see: '主账号已验证，准备创建配对规则'" -ForegroundColor Green
Write-Host "   [✓] '创建主账号' button should turn green/enabled" -ForegroundColor Green
Write-Host ""
Write-Host "5. If also filling in approver info, verify similar validation" -ForegroundColor White
Write-Host ""
Write-Host "6. Test the automatic approver addition logic:" -ForegroundColor White
Write-Host "   - Both validated → Should auto-add approver after creating account" -ForegroundColor Green
Write-Host "   - Only account validated → Should auto-open approver dialog" -ForegroundColor Green
Write-Host ""
Write-Host "7. Test other features:" -ForegroundColor White
Write-Host "   - Enable/disable switch (should NOT return 422 error)" -ForegroundColor Green
Write-Host "   - Delete single approver" -ForegroundColor Green
Write-Host "   - Delete entire account pair" -ForegroundColor Green
Write-Host ""
Write-Host "========================================" -ForegroundColor Cyan
Write-Host "Expected Results Summary:" -ForegroundColor Cyan
Write-Host "========================================" -ForegroundColor Cyan
Write-Host "✓ Bug 1 Fixed: Enable/Disable switch works without 422 error" -ForegroundColor Green
Write-Host "✓ Bug 2 Fixed: Approver validation state properly synchronized" -ForegroundColor Green
Write-Host "✓ Bug 3 Fixed: Can add main account and approver together" -ForegroundColor Green
Write-Host "✓ Enhancement: Green '已验证' tag appears after validation" -ForegroundColor Green
Write-Host "✓ Enhancement: Create button enabled after validation" -ForegroundColor Green
Write-Host "✓ Enhancement: Auto-jump to approver dialog when needed" -ForegroundColor Green
Write-Host ""
Write-Host "Happy Testing!" -ForegroundColor Green
Write-Host ""
