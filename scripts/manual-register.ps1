# Manual Credential Provider Registration Script
# Run as Administrator to register CP if NSIS installation failed to write registry

$clsid = "{E4D9F6E7-8A2B-4C3D-9E5F-1A2B3C4D5E6F}"
$dllPath = "C:\Program Files\WinSLA\DualAuthCP.dll"
$baseRegPath = "HKLM:\SOFTWARE\Classes\CLSID\$clsid"
$cpRegPath = "HKLM:\SOFTWARE\Microsoft\Windows\CurrentVersion\Authentication\Credential Providers\$clsid"

Write-Host "=========================================" -ForegroundColor Cyan
Write-Host "Manual CP Registration Script" -ForegroundColor Cyan
Write-Host "=========================================" -ForegroundColor Cyan
Write-Host ""

# Check DLL exists
Write-Host "[Step 1] Verifying DLL..." -ForegroundColor Yellow
if (-not (Test-Path $dllPath)) {
    Write-Host "ERROR: DLL not found at $dllPath" -ForegroundColor Red
    Write-Host "Please ensure you installed via NSIS installer first." -ForegroundColor Yellow
    exit 1
}
Write-Host "✓ DLL found: $dllPath" -ForegroundColor Green

# Create base CLSID key
Write-Host "`n[Step 2] Creating CLSID registry keys..." -ForegroundColor Yellow
try {
    # Create the main CLSID key
    New-Item -ItemType RegistryKey -Path "HKLM:\SOFTWARE\Classes\CLSID\$clsid" -Force | Out-Null
    Set-ItemProperty -Path "HKLM:\SOFTWARE\Classes\CLSID\$clsid" -Name "(default)" -Value "WinSLA Dual-Auth Credential Provider" -Force
    Write-Host "  ✓ Created CLSID key" -ForegroundColor Green
    
    # Create InprocServer32 subkey
    New-Item -ItemType RegistryKey -Path "HKLM:\SOFTWARE\Classes\CLSID\$clsid\InprocServer32" -Force | Out-Null
    Set-ItemProperty -Path "HKLM:\SOFTWARE\Classes\CLSID\$clsid\InprocServer32" -Name "(default)" -Value $dllPath -Force
    Set-ItemProperty -Path "HKLM:\SOFTWARE\Classes\CLSID\$clsid\InprocServer32" -Name "ThreadingModel" -Value "Apartment" -Force
    Write-Host "  ✓ Created InprocServer32 with ThreadingModel" -ForegroundColor Green
} catch {
    Write-Host "ERROR: Failed to create CLSID registry keys" -ForegroundColor Red
    Write-Host "Error: $($_.Exception.Message)" -ForegroundColor White
    exit 1
}

# Register to Credential Providers
Write-Host "`n[Step 3] Registering to Credential Providers..." -ForegroundColor Yellow
try {
    New-Item -ItemType RegistryKey -Path "HKLM:\SOFTWARE\Microsoft\Windows\CurrentVersion\Authentication\Credential Providers\$clsid" -Force | Out-Null
    Set-ItemProperty -Path "HKLM:\SOFTWARE\Microsoft\Windows\CurrentVersion\Authentication\Credential Providers\$clsid" -Name "(default)" -Value "WinSLA Dual-Auth" -Force
    Set-ItemProperty -Path "HKLM:\SOFTWARE\Microsoft\Windows\CurrentVersion\Authentication\Credential Providers\$clsid" -Name "Disabled" -Value 0 -Type DWord -Force
    Write-Host "  ✓ Registered to Credential Providers" -ForegroundColor Green
} catch {
    Write-Host "ERROR: Failed to register CP" -ForegroundColor Red
    Write-Host "Error: $($_.Exception.Message)" -ForegroundColor White
    exit 1
}

# Verify registration
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
    Write-Host "  ✗ Verification FAILED! Path does not exist" -ForegroundColor Red
    exit 1
}

Write-Host "`n=========================================" -ForegroundColor Green
Write-Host "Registration completed successfully!" -ForegroundColor Green
Write-Host "=========================================" -ForegroundColor Green
Write-Host ""
Write-Host "Next step: Restart computer and check login screen for dual-account credential tile" -ForegroundColor Yellow
Write-Host ""

$response = Read-Host "Restart now? (Y/N)"
if ($response -eq "Y" -or $response -eq "y") {
    Write-Host "Shutting down..." -ForegroundColor Cyan
    Start-Sleep -Seconds 2
    shutdown /r /t 0
    exit 0
} else {
    Write-Host "Please restart manually to apply changes." -ForegroundColor Cyan
}
