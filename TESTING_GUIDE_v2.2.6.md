# WinSLA v2.2.6 - Manual Testing Guide

## 📊 Test Environment Status

| Component | Status | URL/Port | Details |
|-----------|--------|----------|---------|
| Vite Dev Server | ✅ Running | http://localhost:5174 (or 5175) | Hot-reload enabled |
| Mock API Server | ✅ Running | http://localhost:19830 | Python-based |
| Vue Components | ✅ Fixed | DualPairs.vue | Critical bug fixed |

---

## 🔧 Critical Fix Applied

**Bug**: JavaScript error `Cannot read properties of undefined (reading 'validatedAccount')`

**Root Cause**: Incorrect Vue 3 template syntax using `.value` in templates

**Fix Applied**: Updated lines 375, 380, 422, 430 in `management_app/src/views/DualPairs.vue`:
```vue
<!-- BEFORE (Incorrect) -->
<el-form-item v-if="form.value.validatedAccount">
<el-button :disabled="!form.value.validatedAccount">

<!-- AFTER (Correct) -->
<el-form-item v-if="form.validatedAccount">
<el-button :disabled="!form.validatedAccount">
```

---

## 🧪 Step-by-Step Testing Instructions

### **Prerequisites**
Open Chrome DevTools first:
1. Press `F12` or `Ctrl+Shift+I`
2. Open both **Console** and **Network** tabs
3. Check "Preserve log" option

### **Test 1: Dialog Opens Successfully** ✅ EXPECTED

**Steps:**
1. Navigate to: http://localhost:5174/#/pairs
2. Click blue button: `+ 新增主账号`
3. Observe browser Console tab
4. Screenshot the dialog if it opens

**Expected Results:**
- ✅ Dialog opens successfully
- ✅ Form fields visible (Username, Password)
- ✅ No errors in Console
- ✅ No red error messages

**If Fails:**
- Check Console for ANY JavaScript errors
- Verify you're on port 5174 or 5175
- Refresh page and try again

---

### **Test 2: Validation Tag Display** ✨ NEW FEATURE

**Steps:**
1. In the dialog, enter:
   - Username: `HOT\ylw` (or your test account)
   - Password: `your_password`
2. Click blue button: `验证主账号`
3. Wait for response
4. Look for green success tag
5. Check "创建主账号" button state
6. Take screenshot

**Expected Results:**
- ✅ Success toast: "验证成功：HOT\ylw"
- ✅ Green tag appears: "主账号已验证，准备创建配对规则"
- ✅ "创建主账号" button turns green and becomes clickable
- ✅ "取消" button available

**Evidence Capture:**
Take screenshot showing all three elements above. This proves Bug #2 is FIXED!

---

### **Test 3: Approver Validation** ✅ BUG #2 FIX

**Steps:**
1. In SAME dialog, fill approver section:
   - Username: `HOT\zbj`
   - Password: `test123`
2. Click `验证审批人` button
3. Observe validation result
4. Take screenshot

**Expected Results:**
- ✅ Green tag: "审批人已验证，可添加到主账号"
- ✅ Green "添加审批人" button becomes enabled
- ✅ Button text changes from gray/disabled to green/enabled

**This demonstrates:** The form validation state synchronization works correctly!

---

### **Test 4: Create Account + Auto Add** ✅ BUG #3

**Scenario A: Both Validated**
1. Ensure BOTH main account AND approver are validated (green tags visible)
2. Click green "创建主账号" button
3. Observe behavior

**Expected Outcomes:**

**Option A1 - Immediate Success:**
- Message: "主账号与审批人已成功关联"
- Both dialogs close automatically
- Table refreshes with new entry
- Entry shows: Main account + 1 approver

**OR Option A2 - Jump Flow:**
- If only main account validated:
- Message: "请添加第一个审批人"
- Approver dialog auto-opens immediately
- Focus moves to approver input field
- Can continue adding approvers without reopening dialog

**Evidence Capture:**
Screenshot showing either complete creation flow OR auto-jump behavior!

---

### **Test 5: Toggle Enable/Disable** ✅ BUG #1 FIX

**Steps:**
1. After creating an account, find it in table
2. Click the enable/disable switch control
3. Confirm the action dialog ("确定启用/禁用...")
4. Watch Network tab in DevTools
5. Record the HTTP status code

**CRITICAL VERIFICATION:**
Open **Network** tab → Look for request:
```
PUT /api/accounts/{accountSid}/enable
```

Check response:
- ✅ **Status Code: 200 OK** ← THIS IS THE FIX!
- ❌ Should NOT be 422 Unprocessable Entity

**Response Body Should Include:**
```json
{
  "account_sid": "...",
  "enabled": true
}
```

**UI Verification:**
- Switch visual state updates
- Table row may highlight
- No error messages appear

---

## 📝 Complete Test Results Template

Copy this template and fill in results:

```markdown
# WinSLA v2.2.6 Test Results Report

**Date**: YYYY-MM-DD
**Tester**: [Your Name]
**Environment**: Chrome/Firefox/Edge Version X.X

## Summary
[ ] All tests PASSED
[ ] Some issues found (see details)
[ ] Critical issues (cannot deploy)

## Detailed Results

### Test 1: Dialog Opens
- Status: ☐ PASS / ☐ FAIL
- Screenshots: Attached (dialog.png)
- Notes: 

### Test 2: Validation Tags
- Status: ☐ PASS / ☐ FAIL
- Evidence: Green tags appeared as expected
- Screenshots: Attachment(s)
- Notes: 

### Test 3: Approver Validation State Sync
- Status: ☐ PASS / ☐ FAIL
- Evidence: Form state properly synchronized
- Screenshots: Attachment(s)
- Notes: 

### Test 4: Create + Auto Add Logic
- Status: ☐ PASS / ☐ FAIL
- Scenario Used: ☐ Both validated / ☐ Only main validated
- Behavior Observed: 
- Screenshots: Attachment(s)
- Notes: 

### Test 5: Toggle Switch (HTTP 200)
- Status: ☐ PASS / ☐ FAIL
- HTTP Status Code: [Write code here]
- Expected: 200 OK (NOT 422!)
- Network Log Screenshot: attachment(s)
- Notes: 

## Conclusion
[Describe overall functionality and any issues found]

## Recommended Next Steps
☐ Deploy to production  
☐ Fix identified issues before deployment  
☐ Additional testing required
```

---

## 🎯 Quick Verification Commands

**Terminal Commands to Run Before Testing:**

```powershell
# 1. Verify ports are listening
netstat -ano | findstr "5174 5175 19830"

# 2. Test API endpoint directly
curl http://localhost:19830/api/status

# 3. List all node processes
Get-Process node | Select-Object Id,StartTime

# 4. Clear browser cache if needed
# Ctrl+Shift+Delete → Cache images and files
```

---

## 💡 Troubleshooting Tips

### Issue: Dialog Still Won't Open
**Solution:**
1. Hard refresh browser: `Ctrl+F5`
2. Clear browser cache completely
3. Restart dev server: Stop Node process, run `npm run dev` again
4. Check Console for NEW error messages

### Issue: API Returns 500 Error
**Solution:**
1. Verify mock server running: `netstat -ano \| findstr 19830`
2. Check Python process alive: `Get-Process python`
3. Restart Python server: Close terminal, run command again

### Issue: Validation Not Working
**Solution:**
1. Ensure proper credentials entered
2. Check Network tab for POST to `/api/validate-account`
3. Verify response includes `"success": true`

---

## ✅ Final Deployment Checklist

Before deploying v2.2.6 to production/virtual machine:

- [ ] All 5 tests pass with evidence screenshots
- [ ] No console errors
- [ ] Network requests return correct status codes
- [ ] Mock API tested successfully
- [ ] Full end-to-end flow verified
- [ ] Bug #1 fixed (200 OK not 422)
- [ ] Bug #2 fixed (validation state sync)
- [ ] Bug #3 fixed (auto-add logic works)
- [ ] New features working (green tags, button states)

Once checklist complete, ready to:
1. Build production version: `npm run build`
2. Compile management app: `cargo build --release -p winsla-management`
3. Generate installer: NSIS packaging
4. Deploy to VM/environment

---

**Happy Testing!** 🚀

For questions or issues encountered during testing, please provide:
1. Detailed description of what happened
2. Browser Console errors (full message)
3. Network tab screenshots (request + response)
4. Any additional context that might help diagnose
