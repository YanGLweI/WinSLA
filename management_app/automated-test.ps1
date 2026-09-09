# WinSLA v2.2.6 Automated Test Execution Script

param(
    [switch]$FullTest,
    [switch]$QuickTest
)

$baseUrl = "http://localhost:5173"
$apiUrl = "http://localhost:19830"

Write-Host "=== WinSLA v2.2.6 Automated Browser Testing ===" -ForegroundColor Cyan
Write-Host ""

# 检查开发服务器
Write-Host "[Setup] Checking development server..." -ForegroundColor Yellow
try {
    $response = Invoke-RestMethod -Uri "$baseUrl" -TimeoutSec 5 -ErrorAction Stop
    Write-Host "✓ Development server running at $baseUrl" -ForegroundColor Green
} catch {
    Write-Host "✗ Development server not accessible" -ForegroundColor Red
    exit 1
}

# 测试 API 连接
Write-Host "`n[API Test] Checking backend API..." -ForegroundColor Yellow
try {
    $status = Invoke-RestMethod -Uri "$apiUrl/api/status" -TimeoutSec 5 -ErrorAction Stop
    Write-Host "  ✓ Backend API: $($status.running ? 'Running' : 'Not Running')" -ForegroundColor White
    Write-Host "  ✓ Version: $($status.version)" -ForegroundColor White
} catch {
    Write-Host "  ⚠ Backend API not accessible (may be expected)" -ForegroundColor Gray
}

Write-Host "`n[Browser Test] Opening automated test..." -ForegroundColor Yellow
Write-Host "Please follow the instructions below:" -ForegroundColor Cyan
Write-Host ""
Write-Host "1. Open your browser and navigate to: $baseUrl" -ForegroundColor White
Write-Host "2. Wait for Vue app to fully load" -ForegroundColor White
Write-Host "3. Perform the following manual tests:" -ForegroundColor White
Write-Host ""

if ($QuickTest.IsPresent) {
    Write-Host "=== QUICK TEST CHECKLIST ===" -ForegroundColor Magenta
    Write-Host ""
    Write-Host "Test A: Toggle Switch" -ForegroundColor White
    Write-Host "  1. Find any account in the table" -ForegroundColor Gray
    Write-Host "  2. Click the enable/disable switch" -ForegroundColor Gray
    Write-Host "  3. Check Network tab for PUT request to /api/accounts/:id/enable" -ForegroundColor Gray
    Write-Host "  4. Verify status code is 200 (NOT 422)" -ForegroundColor Gray
    Write-Host "  Status: ☐ Pass / ☐ Fail" -ForegroundColor Yellow
    Write-Host ""
    
    Write-Host "Test B: Validation Tags" -ForegroundColor White
    Write-Host "  1. Add main account or approver" -ForegroundColor Gray
    Write-Host "  2. Click validate button with credentials" -ForegroundColor Gray
    Write-Host "  3. Check if green success tag appears" -ForegroundColor Gray
    Write-Host "     Expected: '主账号已验证，准备创建配对规则'" -ForegroundColor Gray
    Write-Host "     OR: '审批人已验证，可添加到主账号'" -ForegroundColor Gray
    Write-Host "  Status: ☐ Pass / ☐ Fail" -ForegroundColor Yellow
    Write-Host ""
    
    Write-Host "Test C: Create Button State" -ForegroundColor White
    Write-Host "  1. Before validation: '创建主账号' button should be disabled" -ForegroundColor Gray
    Write-Host "  2. After validation: Button should turn green/enabled" -ForegroundColor Gray
    Write-Host "  Status: ☐ Pass / ☐ Fail" -ForegroundColor Yellow
    Write-Host ""
    
} elseif ($FullTest.IsPresent) {
    Write-Host "=== COMPREHENSIVE TEST CHECKLIST ===" -ForegroundColor Magenta
    Write-Host ""
    
    Write-Host "Test 1: Toggle Enable/Disable Switch (Bug #1 Fix)" -ForegroundColor White
    Write-Host "  Steps:" -ForegroundColor Gray
    Write-Host "    a) Navigate to pair configuration page" -ForegroundColor Gray
    Write-Host "    b) Find any existing account entry" -ForegroundColor Gray
    Write-Host "    c) Click the enable/disable switch control" -ForegroundColor Gray
    Write-Host "    d) Confirm the action dialog" -ForegroundColor Gray
    Write-Host "  Expected Results:" -ForegroundColor Green
    Write-Host "    ✓ Confirmation dialog appears" -ForegroundColor Gray
    Write-Host "    ✓ HTTP 200 OK returned (not 422)" -ForegroundColor Gray
    Write-Host "    ✓ State updates successfully in UI" -ForegroundColor Gray
    Write-Host "  Test Result: ☐ PASS / ☐ FAIL" -ForegroundColor Yellow
    Write-Host ""
    
    Write-Host "Test 2: Approver Validation State Sync (Bug #2 Fix)" -ForegroundColor White
    Write-Host "  Steps:" -ForegroundColor Gray
    Write-Host "    a) Open 'Add Approver' dialog" -ForegroundColor Gray
    Write-Host "    b) Enter valid domain username & password" -ForegroundColor Gray
    Write-Host "    c) Click 'Validate Approver' button" -ForegroundColor Gray
    Write-Host "    d) Observe the success message and button state" -ForegroundColor Gray
    Write-Host "  Expected Results:" -ForegroundColor Green
    Write-Host "    ✓ Green success tag appears immediately" -ForegroundColor Gray
    Write-Host "    ✓ Tag text: '审批人已验证，可添加到主账号'" -ForegroundColor Gray
    Write-Host "    ✓ Add approver button turns green and enables" -ForegroundColor Gray
    Write-Host "  Further Test:" -ForegroundColor Gray
    Write-Host "    e) Change password to invalid value" -ForegroundColor Gray
    Write-Host "    f) Validate again" -ForegroundColor Gray
    Write-Host "    g) Observe validation state resets" -ForegroundColor Gray
    Write-Host "  Expected:" -ForegroundColor Green
    Write-Host "    ✓ Green tag disappears" -ForegroundColor Gray
    Write-Host "    ✓ Form state properly resets" -ForegroundColor Gray
    Write-Host "    ✓ Add button re-disables" -ForegroundColor Gray
    Write-Host "  Test Result: ☐ PASS / ☐ FAIL" -ForegroundColor Yellow
    Write-Host ""
    
    Write-Host "Test 3: Simultaneous Creation (Bug #3 Fix)" -ForegroundColor White
    Write-Host ""
    Write-Host "  Scenario A - Both Validated:" -ForegroundColor Cyan
    Write-Host "    Steps:" -ForegroundColor Gray
    Write-Host "      1) Click '+ 新增主账号' button" -ForegroundColor Gray
    Write-Host "      2) Enter main account credentials" -ForegroundColor Gray
    Write-Host "      3) Enter approver credentials (in same dialog)" -ForegroundColor Gray
    Write-Host "      4) Validate both accounts" -ForegroundColor Gray
    Write-Host "      5) Click 'Create Main Account' button" -ForegroundColor Gray
    Write-Host "    Expected:" -ForegroundColor Green
    Write-Host "      ✓ Both accounts created simultaneously" -ForegroundColor Gray
    Write-Host "      ✓ Auto-adds first approver" -ForegroundColor Gray
    Write-Host "      ✓ Success message shows" -ForegroundColor Gray
    Write-Host "      ✓ Dialog closes correctly" -ForegroundColor Gray
    Write-Host "      ✓ Table refreshes with new entry" -ForegroundColor Gray
    Write-Host "    Test Result: ☐ PASS / ☐ FAIL" -ForegroundColor Yellow
    Write-Host ""
    
    Write-Host "  Scenario B - Main Only (Auto-Jump):" -ForegroundColor Cyan
    Write-Host "    Steps:" -ForegroundColor Gray
    Write-Host "      1) Click '+ 新增主账号' button" -ForegroundColor Gray
    Write-Host "      2) Enter ONLY main account credentials" -ForegroundColor Gray
    Write-Host "      3) Leave approver fields empty" -ForegroundColor Gray
    Write-Host "      4) Validate main account only" -ForegroundColor Gray
    Write-Host "      5) Click 'Create Main Account' button" -ForegroundColor Gray
    Write-Host "    Expected:" -ForegroundColor Green
    Write-Host "      ✓ Main account created successfully" -ForegroundColor Gray
    Write-Host "      ✓ Message: '请添加第一个审批人'" -ForegroundColor Gray
    Write-Host "      ✓ Approver dialog auto-opens" -ForegroundColor Gray
    Write-Host "      ✓ Focus moves to approver input" -ForegroundColor Gray
    Write-Host "    Test Result: ☐ PASS / ☐ FAIL" -ForegroundColor Yellow
    Write-Host ""
    
    Write-Host "Test 4: API Response Codes" -ForegroundColor White
    Write-Host "  Use DevTools → Network tab to monitor:" -ForegroundColor Gray
    Write-Host ""
    Write-Host "    POST /api/accounts:" -ForegroundColor Cyan
    Write-Host "      Expected: 201 Created" -ForegroundColor Gray
    Write-Host "      NOT: 422 Unprocessable Entity" -ForegroundColor Gray
    Write-Host ""
    Write-Host "    POST /api/accounts/approvers:" -ForegroundColor Cyan
    Write-Host "      Expected: 201 Created" -ForegroundColor Gray
    Write-Host ""
    Write-Host "    PUT /api/accounts/{sid}/enable:" -ForegroundColor Cyan
    Write-Host "      Expected: 200 OK" -ForegroundColor Gray
    Write-Host "      Critical: SHOULD NOT BE 422!" -ForegroundColor Red
    Write-Host ""
    
} else {
    Write-Host "=== STANDARD TEST CHECKLIST ===" -ForegroundColor Magenta
    Write-Host "Please run either -QuickTest or -FullTest parameter for more details." -ForegroundColor Yellow
    Write-Host ""
}

Write-Host "========================================" -ForegroundColor Cyan
Write-Host "Testing Complete" -ForegroundColor Cyan
Write-Host "========================================" -ForegroundColor Cyan
Write-Host ""
Write-Host "Next Steps:" -ForegroundColor White
Write-Host "1. Review the test checklist above" -ForegroundColor Gray
Write-Host "2. Execute tests in your browser" -ForegroundColor Gray
Write-Host "3. Record results using the template" -ForegroundColor Gray
Write-Host "4. Report back with findings" -ForegroundColor Gray
Write-Host ""
Write-Host "Debug Tips:" -ForegroundColor White
Write-Host "- Press F12 to open DevTools" -ForegroundColor Gray
Write-Host "- Go to Console tab for JS errors" -ForegroundColor Gray
Write-Host "- Go to Network tab to see API calls" -ForegroundColor Gray
Write-Host "- Look for failed requests (red color)" -ForegroundColor Gray
Write-Host ""
Write-Host "Happy Testing!" -ForegroundColor Green
