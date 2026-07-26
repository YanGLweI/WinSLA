# Force Credential Provider Cache Refresh
# Run this to clear LogonUI's credential provider cache

$clsid = "{E4D9F6E7-8A2B-4C3D-9E5F-1A2B3C4D5E6F}"

Write-Host "=========================================" -ForegroundColor Cyan
Write-Host "Credential Provider Cache Clear" -ForegroundColor Cyan
Write-Host "=========================================" -ForegroundColor Cyan
Write-Host ""

# Stop Windows Explorer first (it uses LogonUI)
Write-Host "[Step 1] Stopping Explorer..." -ForegroundColor Yellow
try {
    Stop-Process -Name "explorer" -Force -ErrorAction SilentlyContinue
    Write-Host "✓ Explorer stopped" -ForegroundColor Green
} catch {
    Write-Host "⚠ Could not stop Explorer: $_" -ForegroundColor Yellow
}

Start-Sleep -Seconds 3

# Delete the CLSID from Credential Providers (re-register it immediately)
Write-Host "`n[Step 2] Resetting CP registration..." -ForegroundColor Yellow
try {
    # Remove existing entry
    $cpPath = "HKLM:\SOFTWARE\Microsoft\Windows\CurrentVersion\Authentication\Credential Providers\$clsid"
    if (Test-Path $cpPath) {
        Remove-Item -Path $cpPath -Force -Recurse -ErrorAction SilentlyContinue
        Write-Host "  ✓ Removed existing CP entry" -ForegroundColor Gray
    }
    
    # Re-create fresh
    New-Item -ItemType RegistryKey -Path "HKLM:\SOFTWARE\Microsoft\Windows\CurrentVersion\Authentication\Credential Providers\$clsid" -Force | Out-Null
    Set-ItemProperty -Path "HKLM:\SOFTWARE\Microsoft\Windows\CurrentVersion\Authentication\Credential Providers\$clsid" -Name "(default)" -Value "WinSLA Dual-Auth" -Force
    Set-ItemProperty -Path "HKLM:\SOFTWARE\Microsoft\Windows\CurrentVersion\Authentication\Credential Providers\$clsid" -Name "Disabled" -Value 0 -Type DWord -Force
    Write-Host "  ✓ Re-created CP entry with Disabled=0" -ForegroundColor Green
} catch {
    Write-Host "ERROR: Failed to reset CP registration: $_" -ForegroundColor Red
    exit 1
}

# Restart Explorer
Write-Host "`n[Step 3] Starting Explorer..." -ForegroundColor Yellow
try {
    Start-Process explorer.exe
    Write-Host "✓ Explorer restarted" -ForegroundColor Green
} catch {
    Write-Host "ERROR: Failed to start Explorer: $_" -ForegroundColor Red
    exit 1
}

Start-Sleep -Seconds 5

# Verify the registry
Write-Host "`n[Step 4] Verifying registration..." -ForegroundColor Yellow
$verifyPath = "HKLM:\SOFTWARE\Microsoft\Windows\CurrentVersion\Authentication\Credential Providers\$clsid"
if (Test-Path $verifyPath) {
    $value = Get-ItemProperty -Path $verifyPath -Name "(default)" -ErrorAction SilentlyContinue
    if ($value."(default)" -eq "WinSLA Dual-Auth") {
        Write-Host "  ✓ Registration verified!" -ForegroundColor Green
    } else {
        Write-Host "  ⚠ Value mismatch but path exists" -ForegroundColor Yellow
    }
} else {
    Write-Host "  ✗ Registration failed!" -ForegroundColor Red
    exit 1
}

Write-Host "`n=========================================" -ForegroundColor Green
Write-Host "Cache cleared and re-registered!" -ForegroundColor Green
Write-Host "=========================================" -ForegroundColor Green
Write-Host ""
Write-Host "Now restart LogonUI:" -ForegroundColor Cyan
Write-Host "- Shutdown or restart the computer" -ForegroundColor White
Write-Host "- OR just logout and login again" -ForegroundColor White
Write-Host ""
Write-Host "The dual-account tile should now appear on the login screen." -ForegroundColor Yellow
