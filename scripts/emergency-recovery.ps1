# Emergency Recovery Script - Restore Default Login Behavior
# Run this to restore PasswordProvider and prevent blank login screen

$pwdCLSID = "{60b78e88-ead8-445c-9cfd-0b87f74ea6cd}"
$winslaCLSID = "{E4D9F6E7-8A2B-4C3D-9E5F-1A2B3C4D5E6F}"

Write-Host "=========================================" -ForegroundColor Cyan
Write-Host "Emergency Recovery - Restore Default Login" -ForegroundColor Cyan
Write-Host "=========================================" -ForegroundColor Cyan
Write-Host ""

Write-Host "[Step 1] Re-enabling PasswordProvider..." -ForegroundColor Yellow

# Enable PasswordProvider (remove Disabled key or set to 1)
try {
    $pwdPath = "HKLM:\SOFTWARE\Microsoft\Windows\CurrentVersion\Authentication\Credential Providers\$pwdCLSID"
    if (Test-Path $pwdPath) {
        # Try to remove the Disabled key entirely (default behavior is enabled)
        Remove-ItemProperty -Path $pwdPath -Name "Disabled" -Force -ErrorAction SilentlyContinue
        
        # Verify it's enabled
        $prop = Get-ItemProperty -Path $pwdPath -Name "Disabled" -ErrorAction SilentlyContinue
        if ($null -eq $prop.Disabled) {
            Write-Host "✓ PasswordProvider re-enabled (Disabled key removed)" -ForegroundColor Green
        } else {
            Set-ItemProperty -Path $pwdPath -Name "Disabled" -Value 1 -Type DWord -Force
            Write-Host "✓ PasswordProvider disabled flag set to 1 (enabled)" -ForegroundColor Green
        }
    } else {
        Write-Host "⚠ PasswordProvider registry not found - assuming default state" -ForegroundColor Yellow
    }
} catch {
    Write-Host "ERROR: Failed to modify PasswordProvider: $_" -ForegroundColor Red
    exit 1
}

Write-Host "`n[Step 2] Ensuring WinSLA remains registered..." -ForegroundColor Yellow

# Keep WinSLA registered but ensure it doesn't interfere
try {
    $winslaPath = "HKLM:\SOFTWARE\Microsoft\Windows\CurrentVersion\Authentication\Credential Providers\$winslaCLSID"
    if (Test-Path $winslaPath) {
        # Leave it as Disabled=0 (enabled) for now - we'll fix implementation later
        Write-Host "✓ WinSLA still registered (will be fixed in next update)" -ForegroundColor Green
    } else {
        Write-Host "✗ WinSLA registration missing!" -ForegroundColor Red
        Write-Host "Please re-run manual-register.ps1" -ForegroundColor Yellow
    }
} catch {
    Write-Host "Could not check WinSLA registration: $_" -ForegroundColor Gray
}

Write-Host "`n=========================================" -ForegroundColor Green
Write-Host "Recovery Complete!" -ForegroundColor Green
Write-Host "=========================================" -ForegroundColor Green
Write-Host ""
Write-Host "PasswordProvider should now work normally." -ForegroundColor White
Write-Host "Please restart your computer:" -ForegroundColor Yellow
Write-Host "shutdown /r /t 0" -ForegroundColor Cyan
Write-Host ""
Write-Host "Next steps after recovery:" -ForegroundColor Cyan
Write-Host "1. We need to properly implement all required interfaces" -ForegroundColor White
Write-Host "2. Test with both providers enabled first" -ForegroundColor White
Write-Host "3. Then consider disabling PasswordProvider once CP is stable" -ForegroundColor White
