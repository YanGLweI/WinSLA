# Emergency Uninstall Script - Credential Provider Recovery
# Run this in Safe Mode or Windows Recovery Environment to restore login functionality

$clsid = "{E4D9F6E7-8A2B-4C3D-9E5F-1A2B3C4D5E6F}"

Write-Host "=========================================" -ForegroundColor Red
Write-Host "EMERGENCY UNINSTALL SCRIPT" -ForegroundColor Red
Write-Host "Credential Provider Recovery Tool" -ForegroundColor Red
Write-Host "=========================================" -ForegroundColor Red
Write-Host ""

Write-Host "This script will remove WinSLA's Credential Provider registration." -ForegroundColor Yellow
Write-Host "You must be able to boot into Windows (Safe Mode) to run this!" -ForegroundColor Yellow
Write-Host ""

$response = Read-Host "Run emergency uninstall? (Y/N)"
if ($response -ne "Y" -and $response -ne "y") {
    Write-Host "Aborted. Please reboot and try again." -ForegroundColor Yellow
    exit 0
}

# Delete Credential Providers registry key
Write-Host "`n[Step 1] Removing from Credential Providers..." -ForegroundColor Yellow
try {
    $cpPath = "HKLM:\SOFTWARE\Microsoft\Windows\CurrentVersion\Authentication\Credential Providers\$clsid"
    if (Test-Path $cpPath) {
        Remove-Item -Path $cpPath -Force -Recurse
        Write-Host "✓ Removed CP registry key" -ForegroundColor Green
    } else {
        Write-Host "⚠ CP registry key not found" -ForegroundColor Yellow
    }
} catch {
    Write-Host "ERROR: Failed to delete CP registry key: $_" -ForegroundColor Red
    exit 1
}

# Delete CLSID registry key
Write-Host "`n[Step 2] Removing CLSID base keys..." -ForegroundColor Yellow
try {
    $clsidPath = "HKLM:\SOFTWARE\Classes\CLSID\$clsid"
    if (Test-Path $clsidPath) {
        Remove-Item -Path $clsidPath -Force -Recurse
        Write-Host "✓ Removed CLSID registry keys" -ForegroundColor Green
    } else {
        Write-Host "⚠ CLSID registry key not found" -ForegroundColor Yellow
    }
} catch {
    Write-Host "ERROR: Failed to delete CLSID registry keys: $_" -ForegroundColor Red
    exit 1
}

# Verify removal
Write-Host "`n[Step 3] Verifying removal..." -ForegroundColor Yellow
$verifyPath = "HKLM:\SOFTWARE\Microsoft\Windows\CurrentVersion\Authentication\Credential Providers\$clsid"
if (-not (Test-Path $verifyPath)) {
    Write-Host "✓ Verification passed! WinSLA CP has been removed." -ForegroundColor Green
    Write-Host ""
    Write-Host "Please restart your computer now:" -ForegroundColor Cyan
    Write-Host "shutdown /r /t 0" -ForegroundColor White
} else {
    Write-Host "✗ Verification failed! Registry key still exists." -ForegroundColor Red
    Write-Host "Please check manually using regedit.exe" -ForegroundColor Yellow
    exit 1
}
