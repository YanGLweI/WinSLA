# WinSLA Credential Provider Registration Script
# Run this script as Administrator

$clsid = "{E4D9F6E7-8A2B-4C3D-9E5F-1A2B3C4D5E6F}"
$instDir = "C:\Program Files (x86)\WinSLA"
$dllPath = "$instDir\DualAuthCP.dll"

Write-Host "========================================" -ForegroundColor Cyan
Write-Host "WinSLA Credential Provider Registration Script" -ForegroundColor Cyan
Write-Host "========================================" -ForegroundColor Cyan
Write-Host ""

# Verify DLL exists
Write-Host "[1/5] Verifying DLL file..." -ForegroundColor Yellow
if (-not (Test-Path $dllPath)) {
    Write-Host "ERROR: DLL file not found at $dllPath" -ForegroundColor Red
    Write-Host "Please check if DLL is correctly copied to installation directory" -ForegroundColor Red
    exit 1
}
Write-Host "OK: DLL file found" -ForegroundColor Green

# Get timestamp for logging
$dateStamp = Get-Date -Format "yyyy-MM-dd HH:mm:ss"
Write-Host ""
Write-Host "[2/5] Creating registry entries ($dateStamp)..." -ForegroundColor Yellow

try {
    # 1. Create CLSID base key
    Write-Host "  [1/3] Creating CLSID base key..." -ForegroundColor Gray
    New-Item -ItemType RegistryKey -Path "HKLM:\SOFTWARE\Classes\CLSID\$clsid" -Force | Out-Null
    
    # Set description
    Set-ItemProperty -Path "HKLM:\SOFTWARE\Classes\CLSID\$clsid" -Name "(default)" -Value "WinSLA Dual-Auth Credential Provider" -Force
    Write-Host "      OK: CLSID base key created" -ForegroundColor Green
    
    # 2. Create InprocServer32 subkey
    Write-Host "  [2/3] Configuring InprocServer32..." -ForegroundColor Gray
    New-Item -ItemType RegistryKey -Path "HKLM:\SOFTWARE\Classes\CLSID\$clsid\InprocServer32" -Force | Out-Null
    Set-ItemProperty -Path "HKLM:\SOFTWARE\Classes\CLSID\$clsid\InprocServer32" -Name "(default)" -Value $dllPath -Force
    Set-ItemProperty -Path "HKLM:\SOFTWARE\Classes\CLSID\$clsid\InprocServer32" -Name "ThreadingModel" -Value "Apartment" -Force
    Write-Host "      OK: InprocServer32 configured" -ForegroundColor Green
    
    # 3. Register to Credential Providers
    Write-Host "  [3/3] Registering to Credential Providers..." -ForegroundColor Gray
    New-Item -ItemType RegistryKey -Path "HKLM:\SOFTWARE\Microsoft\Windows\CurrentVersion\Authentication\Credential Providers\$clsid" -Force | Out-Null
    Set-ItemProperty -Path "HKLM:\SOFTWARE\Microsoft\Windows\CurrentVersion\Authentication\Credential Providers\$clsid" -Name "(default)" -Value "WinSLA Dual-Auth" -Force
    Set-ItemProperty -Path "HKLM:\SOFTWARE\Microsoft\Windows\CurrentVersion\Authentication\Credential Providers\$clsid" -Name "Disabled" -Value 0 -Type DWord -Force
    Write-Host "      OK: Registered to Credential Providers" -ForegroundColor Green
    
    Write-Host ""
    Write-Host "========================================" -ForegroundColor Green
    Write-Host "Registration completed successfully!" -ForegroundColor Green
    Write-Host "========================================" -ForegroundColor Green
    Write-Host ""
    
    # Show final configuration
    Write-Host "Final Configuration:" -ForegroundColor Cyan
    Write-Host "  CLSID:       $clsid"
    Write-Host "  DLL Path:    $dllPath"
    Write-Host "  ThreadingModel: Apartment"
    Write-Host "  Disabled:     0 (enabled)"
    Write-Host ""
    
    Write-Host "Next Steps:" -ForegroundColor Yellow
    Write-Host "1. Restart computer or logout then login again" -ForegroundColor White
    Write-Host "2. Check if dual-account credential tile appears on login screen" -ForegroundColor White
    Write-Host ""
    
    # Offer quick restart option
    $response = Read-Host "`nRestart computer now? (Y/N)"
    if ($response -eq "Y" -or $response -eq "y") {
        Write-Host "Shutting down..." -ForegroundColor Cyan
        Start-Sleep -Seconds 2
        shutdown /r /t 0
        exit 0
    } else {
        Write-Host "Restart cancelled, please restart manually to apply changes." -ForegroundColor Cyan
    }
    
} catch {
    Write-Host ""
    Write-Host "ERROR: Registry operation failed!" -ForegroundColor Red
    Write-Host "Error message: $($_.Exception.Message)" -ForegroundColor Red
    Write-Host ""
    Write-Host "Please check:" -ForegroundColor Yellow
    Write-Host "- Whether running PowerShell as administrator" -ForegroundColor White
    Write-Host "- Whether registry permissions are correct" -ForegroundColor White
    exit 1
}
