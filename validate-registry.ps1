# NSIS v1.0.2 Registry Write Validator
# Run this script AFTER installation to verify registry keys were auto-written

$clsid = "{E4D9F6E7-8A2B-4C3D-9E5F-1A2B3C4D5E6F}"

Write-Host ""
Write-Host "==============================================" -ForegroundColor Cyan
Write-Host "NSIS v1.0.2 Registry Validation Tool" -ForegroundColor Cyan
Write-Host "==============================================" -ForegroundColor Cyan
Write-Host ""

Write-Host "Validating NSIS installer automatic registry writes..." -ForegroundColor Yellow
Write-Host ""

$totalChecks = 0
$passedChecks = 0

# Check 1: Service exists and running
$totalChecks++
Write-Host "[$totalChecks] Checking WinSLA Service..." -ForegroundColor Gray
$service = Get-Service "WinSLA Service" -ErrorAction SilentlyContinue
if ($null -ne $service) {
    if ($service.Status -eq 'Running') {
        Write-Host "  ✅ PASS: Service exists and is RUNNING" -ForegroundColor Green
        $passedChecks++
    } else {
        Write-Host "  ⚠️  PARTIAL: Service exists but NOT running (Status: $($service.Status))" -ForegroundColor Yellow
    }
} else {
    Write-Host "  ❌ FAIL: Service NOT FOUND" -ForegroundColor Red
}

# Check 2: DLL exists in Program Files
$totalChecks++
Write-Host "`n[$totalChecks] Checking DualAuthCP.dll..." -ForegroundColor Gray
$dllPath = "$env:ProgramFiles\WinSLA\DualAuthCP.dll"
if (Test-Path $dllPath) {
    $fileInfo = Get-Item $dllPath
    Write-Host "  ✅ PASS: DLL exists at $dllPath" -ForegroundColor Green
    Write-Host "         Size: $($fileInfo.Length/1KB) KB" -ForegroundColor Gray
    Write-Host "         Modified: $($fileInfo.LastWriteTime)" -ForegroundColor Gray
    $passedChecks++
} else {
    Write-Host "  ❌ FAIL: DLL NOT found at $dllPath" -ForegroundColor Red
}

# Check 3: CLSID base key
$totalChecks++
Write-Host "`n[$totalChecks] Checking CLSID base registry key..." -ForegroundColor Gray
$clsidPath = "HKLM:\SOFTWARE\Classes\CLSID\$clsid"
if (Test-Path $clsidPath) {
    $regValue = Get-ItemProperty -Path $clsidPath -ErrorAction SilentlyContinue
    Write-Host "  ✅ PASS: CLSID key EXISTS" -ForegroundColor Green
    Write-Host "         Description: $($regValue.'(default)')" -ForegroundColor Gray
    Write-Host "         Version: $($regValue.Version)" -ForegroundColor Gray
    $passedChecks++
    
    # Check subkey InprocServer32
    $inprocPath = "$clsidPath\InprocServer32"
    if (Test-Path $inprocPath) {
        $inprocReg = Get-ItemProperty -Path $inprocPath -ErrorAction SilentlyContinue
        Write-Host "  ✅ PASS: InprocServer32 subkey EXISTS" -ForegroundColor Green
        Write-Host "         DLL Path: $($inprocReg.'(default)')" -ForegroundColor Gray
        Write-Host "         ThreadingModel: $($inprocReg.ThreadingModel)" -ForegroundColor Gray
        
        if ($inprocReg.ThreadingModel -eq 'Apartment') {
            Write-Host "         ✅ ThreadingModel correct!" -ForegroundColor Green
            $passedChecks++
        } else {
            Write-Host "         ❌ ThreadingModel incorrect! Expected 'Apartment'" -ForegroundColor Red
        }
    } else {
        Write-Host "  ❌ FAIL: InprocServer32 subkey NOT FOUND" -ForegroundColor Red
    }
} else {
    Write-Host "  ❌ FAIL: CLSID key NOT FOUND - REGISTRY WRITE FAILED!" -ForegroundColor Red
    Write-Host "         This means NSIS installer did NOT write registry!" -ForegroundColor Yellow
}

# Check 4: Credential Providers registration
$totalChecks++
Write-Host "`n[$totalChecks] Checking Credential Providers registration..." -ForegroundColor Gray
$cpPath = "HKLM:\SOFTWARE\Microsoft\Windows\CurrentVersion\Authentication\Credential Providers\$clsid"
if (Test-Path $cpPath) {
    $cpReg = Get-ItemProperty -Path $cpPath -ErrorAction SilentlyContinue
    
    Write-Host "  ✅ PASS: Credential Provider key EXISTS" -ForegroundColor Green
    
    if ($null -ne $cpReg.'(default)') {
        Write-Host "         Display Name: $($cpReg.'(default)')" -ForegroundColor Gray
    }
    
    if ($null -ne $cpReg.Disabled) {
        Write-Host "         Disabled: $($cpReg.Disabled)" -ForegroundColor Gray
        if ($cpReg.Disabled -eq 0) {
            Write-Host "         Status: ENABLED ✓✓✓" -ForegroundColor Green
            Write-Host "         🎉 Registry was AUTOMATICALLY written by NSIS!" -ForegroundColor Magenta
            $passedChecks++
        } elseif ($cpReg.Disabled -eq 1) {
            Write-Host "         Status: DISABLED ⚠️  (should be 0)" -ForegroundColor Yellow
            Write-Host "         Run: Set-ItemProperty ... Disabled 0" -ForegroundColor Gray
        } else {
            Write-Host "         Status: UNKNOWN value: $($cpReg.Disabled)" -ForegroundColor Yellow
        }
    } else {
        Write-Host "  ❌ FAIL: Disabled property NOT FOUND" -ForegroundColor Red
        Write-Host "         Running manual-register.ps1 may help..." -ForegroundColor Gray
    }
    
    if ($null -ne $cpReg."DllPath") {
        Write-Host "         DllPath: $($cpReg.DllPath)" -ForegroundColor Gray
    }
} else {
    Write-Host "  ❌ CRITICAL FAILURE: Credential Providers key NOT FOUND!" -ForegroundColor Red
    Write-Host "                    ✗ NSIS INSTALLER DID NOT WORK!" -ForegroundColor Red
    Write-Host "                    ✗ You still need to run manual-register.ps1" -ForegroundColor Red
    Write-Host ""
    Write-Host "🔧 Manual fix available:" -ForegroundColor Yellow
    Write-Host "   .\scripts\manual-register.ps1" -ForegroundColor White
}

# Check 5: WinSLA install folder
$totalChecks++
Write-Host "`n[$totalChecks] Checking WinSLA install directory..." -ForegroundColor Gray
$installDir = "$env:ProgramFiles\WinSLA"
if (Test-Path $installDir) {
    Write-Host "  ✅ PASS: Install directory exists" -ForegroundColor Green
    Write-Host "         Location: $installDir" -ForegroundColor Gray
    
    $files = Get-ChildItem $installDir | Select-Object -First 10
    foreach ($file in $files) {
        Write-Host "         - $($file.Name)" -ForegroundColor Gray
    }
    $passedChecks++
} else {
    Write-Host "  ❌ FAIL: Install directory NOT FOUND" -ForegroundColor Red
}

# Check 6: No stale WOW6432Node keys (32-bit redirected view should stay clean)
$totalChecks++
Write-Host "`n[$totalChecks] Checking WOW6432Node for stale redirected keys..." -ForegroundColor Gray
$wowCpPath = "HKLM:\SOFTWARE\WOW6432Node\Microsoft\Windows\CurrentVersion\Authentication\Credential Providers\$clsid"
$wowClsidPath = "HKLM:\SOFTWARE\WOW6432Node\Classes\CLSID\$clsid"
if ((Test-Path $wowCpPath) -or (Test-Path $wowClsidPath)) {
    Write-Host "  ⚠️  WARNING: Stale keys found under WOW6432Node (from old 32-bit redirected installs)" -ForegroundColor Yellow
    Write-Host "         These are harmless but should be cleaned up:" -ForegroundColor Gray
    if (Test-Path $wowCpPath) { Write-Host "         - $wowCpPath" -ForegroundColor Gray }
    if (Test-Path $wowClsidPath) { Write-Host "         - $wowClsidPath" -ForegroundColor Gray }
} else {
    Write-Host "  ✅ PASS: No stale WOW6432Node keys" -ForegroundColor Green
    $passedChecks++
}

# Summary
Write-Host ""
Write-Host "==============================================" -ForegroundColor Cyan
Write-Host "VALIDATION SUMMARY" -ForegroundColor Cyan
Write-Host "==============================================" -ForegroundColor Cyan
Write-Host ""
Write-Host "Total checks: $totalChecks" -ForegroundColor Gray
Write-Host "Passed: $passedChecks" -ForegroundColor Gray
Write-Host "Failed: $($totalChecks - $passedChecks)" -ForegroundColor Gray
Write-Host ""

if ($passedChecks -eq $totalChecks) {
    Write-Host "🎉 SUCCESS! All checks passed!" -ForegroundColor Green
    Write-Host ""
    Write-Host "✅ NSIS installer v1.0.2 WORKS CORRECTLY!" -ForegroundColor Green
    Write-Host "✅ Registry was automatically written during installation" -ForegroundColor Green
    Write-Host "✅ No manual PowerShell scripts needed!" -ForegroundColor Green
    Write-Host ""
    Write-Host "Next step: Restart computer and test login screen" -ForegroundColor Cyan
    Write-Host ""
    Write-Host "To restart now, press Enter..." -ForegroundColor Yellow
    $null = Read-Host
    shutdown /r /t 0
} else {
    Write-Host "❌ Some checks failed!" -ForegroundColor Red
    Write-Host ""
    
    # Find the critical failure
    if (-not (Test-Path $cpPath)) {
        Write-Host "⚠️  CRITICAL: NSIS installer did NOT write registry!" -ForegroundColor Red
        Write-Host ""
        Write-Host "You have two options:" -ForegroundColor Yellow
        Write-Host ""
        Write-Host "Option 1: Use manual register (temporary fix):" -ForegroundColor White
        Write-Host "  .\scripts\manual-register.ps1" -ForegroundColor Gray
        Write-Host ""
        Write-Host "Option 2: Re-run NSIS installer as Administrator:" -ForegroundColor White
        Write-Host "  1. Right-click WinSLA-v1.0.2-Setup.exe" -ForegroundColor Gray
        Write-Host "  2. Select 'Run as Administrator'" -ForegroundColor Gray
        Write-Host "  3. Say YES to UAC prompt" -ForegroundColor Gray
        Write-Host "  4. Follow installation wizard" -ForegroundColor Gray
        Write-Host ""
    }
}

Write-Host "==============================================" -ForegroundColor Cyan
Write-Host ""
