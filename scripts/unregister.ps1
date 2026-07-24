# WinSLA Uninstallation Script (PowerShell)
# Requires Administrator privileges

param(
    [string]$InstallPath = "$env:SystemRoot\System32\winsla"
)

# Check for admin privileges
$isAdmin = ([Security.Principal.WindowsPrincipal] `
    [Security.Principal.WindowsIdentity]::GetCurrent()).IsInRole(
    [Security.Principal.WindowsBuiltInRole]::Administrator)

if (-not $isAdmin) {
    Write-Error "This script must be run as Administrator. Please right-click and 'Run as administrator'"
    exit 1
}

Write-Host "WinSLA Uninstallation Script" -ForegroundColor Green
Write-Host "=============================" -ForegroundColor Green

$service_name = "WinSLA Service"
$provider_guid = "{E4D9F6E7-8A2B-4C3D-9E5F-1A2B3C4D5E6F}"

# Stop service
Write-Host "`n Stopping Windows Service..." -ForegroundColor Yellow
try {
    Get-Service -Name $service_name -ErrorAction SilentlyContinue | Stop-Service -Force
    Write-Host "Service stopped successfully" -ForegroundColor Green
} catch {
    Write-Warning "Could not stop service: $_"
}

# Delete service
Write-Host "`n Removing Windows Service..." -ForegroundColor Yellow
& sc.exe delete $service_name

if ($LASTEXITCODE -eq 0) {
    Write-Host "Service deleted successfully" -ForegroundColor Green
} else {
    Write-Warning "Failed to delete service"
}

# Remove Credential Provider registry entry
Write-Host "`n Removing Credential Provider from registry..." -ForegroundColor Yellow
$provider_reg_path = "HKLM:\SOFTWARE\Microsoft\Windows\CurrentVersion\Auth\LogonProviders\$provider_guid"

if (Test-Path $provider_reg_path) {
    Remove-Item -Path $provider_reg_path -Force
    Write-Host "Credential Provider registry entry removed" -ForegroundColor Green
} else {
    Write-Warning "Registry entry not found at $provider_reg_path"
}

# Remove installation directory
Write-Host "`n Removing installation directory..." -ForegroundColor Yellow
if (Test-Path $InstallPath) {
    Remove-Item -Path $InstallPath -Recurse -Force
    Write-Host "Installation directory removed" -ForegroundColor Green
} else {
    Write-Warning "Installation directory not found at $InstallPath"
}

Write-Host "`n Uninstallation completed!" -ForegroundColor Green
Write-Host "Please reboot the system to ensure all changes take effect." -ForegroundColor Yellow
