# WinSLA Installation Script (PowerShell)
# Requires Administrator privileges

param(
    [Parameter(Mandatory=$false)]
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

Write-Host "WinSLA Installation Script" -ForegroundColor Green
Write-Host "=========================" -ForegroundColor Green

$cp_dll_path = Join-Path $InstallPath "DualAuthCP.dll"
$service_exe_path = Join-Path $InstallPath "winsla-service.exe"

# Create installation directories
Write-Host "`n Creating directories..." -ForegroundColor Yellow
New-Item -ItemType Directory -Force -Path $InstallPath | Out-Null

# Copy files (assumes they've been built)
$bin_dir = Join-Path $PSScriptRoot "target\release"
if (Test-Path $bin_dir) {
    Write-Host " Building project first..." -ForegroundColor Yellow
    cargo build --release
    $bin_dir = Join-Path $PSScriptRoot "target\release"
} else {
    Write-Host " Using existing binaries in: $bin_dir" -ForegroundColor Gray
}

# Copy DLL
if (Test-Path (Join-Path $bin_dir "DualAuthCP.dll")) {
    Copy-Item (Join-Path $bin_dir "DualAuthCP.dll") $cp_dll_path -Force
    Write-Host "Copied DualAuthCP.dll to $cp_dll_path" -ForegroundColor Green
} else {
    Write-Warning "DualAuthCP.dll not found in $bin_dir"
}

# Copy Service EXE
if (Test-Path (Join-Path $bin_dir "winsla-service.exe")) {
    Copy-Item (Join-Path $bin_dir "winsla-service.exe") $service_exe_path -Force
    Write-Host "Copied winsla-service.exe to $service_exe_path" -ForegroundColor Green
} else {
    Write-Warning "winsla-service.exe not found in $bin_dir"
}

# Define service name constant
$service_name = "WinSLA Service"

# Register Credential Provider in Registry
Write-Host "`n Registering Credential Provider..." -ForegroundColor Yellow
$provider_reg_path = "HKLM:\SOFTWARE\Microsoft\Windows\CurrentVersion\Auth\LogonProviders"

# This is simplified - real implementation needs proper CLSID registration
# The CLSID should match CLSID_DUAL_AUTH_PROVIDER in lib.rs
$provider_guid = "{E4D9F6E7-8A2B-4C3D-9E5F-1A2B3C4D5E6F}"

try {
    if (-not (Test-Path $provider_reg_path)) {
        New-Item -Path $provider_reg_path -Force | Out-Null
    }
    
    Set-ItemProperty -Path "$provider_reg_path\$provider_guid" -Name "DisplayName" -Value "WinSLA Dual-Account Authentication" -Force
    Set-ItemProperty -Path "$provider_reg_path\$provider_guid" -Name "DllPath" -Value $cp_dll_path -Force
    Set-ItemProperty -Path "$provider_reg_path\$provider_guid" -Name "ThumbnailsDisabled" -Value 0 -Force
    
    Write-Host "Registered Credential Provider: $provider_guid" -ForegroundColor Green
} catch {
    Write-Error "Failed to register Credential Provider: $_"
}

# Install Windows Service
Write-Host "`n Installing Windows Service..." -ForegroundColor Yellow
& sc.exe create "$service_name" binPath="$service_exe_path" start=auto

if ($LASTEXITCODE -eq 0) {
    Write-Host "Service '$service_name' installed successfully" -ForegroundColor Green
    
    Start-Service -Name $service_name -ErrorAction SilentlyContinue
    Write-Host "Started service '$service_name'" -ForegroundColor Green
} else {
    Write-Error "Failed to install service. Exit code: $LASTEXITCODE"
}

Write-Host "`n Installation completed!" -ForegroundColor Green
Write-Host "Please reboot the system or restart logonui.exe to activate the new credential provider." -ForegroundColor Yellow
