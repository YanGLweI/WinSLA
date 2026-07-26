# Full Build and Install Package Creator
# This script creates a complete deployment package including installer

param(
    [string]$OutputDir = "C:\Temp\WinSLA-Full-Package",
    [switch]$CreateInstaller,
    [Switch]$AutoRunAfterBuild
)

$ErrorActionPreference = "Stop"

# Ensure output directory exists
if (-not (Test-Path $OutputDir)) {
    New-Item -ItemType Directory -Path $OutputDir | Out-Null
}

Write-Host "=========================================" -ForegroundColor Cyan
Write-Host "WinSLA - Complete Build & Package System" -ForegroundColor Cyan
Write-Host "=========================================" -ForegroundColor Cyan
Write-Host ""

# Step 1: Compile DLL and executables
Write-Host "[Step 1/4] Compiling Rust components..." -ForegroundColor Yellow
Write-Host "Running: cargo build --release --workspace" -ForegroundColor Gray

cd "$PSScriptRoot"

try {
    cargo build --release --workspace
    
    Write-Host "✓ Compilation completed successfully!" -ForegroundColor Green
    
    # Copy built files to temp folder
    $tempFolder = Join-Path $OutputDir "InstallFiles"
    Remove-Item $tempFolder -Recurse -Force -ErrorAction SilentlyContinue
    New-Item -ItemType Directory -Path $tempFolder | Out-Null
} catch {
    Write-Error "Compilation failed: $($_.Exception.Message)"
    exit 1
}

# Step 2: Copy built binaries
Write-Host "`n[Step 2/4] Preparing installation files..." -ForegroundColor Yellow
Write-Host "Copying binaries from target/release/" -ForegroundColor Gray

$filesToCopy = @("DualAuthCP.dll", "winsla-service.exe", "winsla-management.exe")
foreach ($file in $filesToCopy) {
    if (Test-Path "target\release\$file") {
        Copy-Item "target\release\$file" "$tempFolder\" -Force
        Write-Host "  ✓ Copied: $file" -ForegroundColor Green
    } else {
        Write-Error "File not found: $file"
        exit 1
    }
}

# Copy assets
if (Test-Path "..\assets\winsla.ico") {
    Copy-Item "..\assets\winsla.ico" "$tempFolder\" -Force
    Write-Host "  ✓ Copied: winsla.ico" -ForegroundColor Green
}

# Copy scripts
Set-Location "$PSScriptRoot\scripts"
$scriptList = Get-ChildItem "*.ps1" | Where-Object { $_.Name -match '^(auto-deploy|emergency-recovery|manual-register|troubleshoot)' }
Set-Location "$PSScriptRoot"

foreach ($script in $scriptList) {
    $destScript = Join-Path "$tempFolder\scripts" $script.Name
    New-Item -ItemType Directory -Path "$tempFolder\scripts" -Force | Out-Null
    Copy-Item $script.FullName $destScript -Force
    Write-Host "  ✓ Copied script: $($script.Name)" -ForegroundColor Gray
}

# Step 3: Create NSIS installer
Write-Host "`n[Step 3/4] Creating NSIS installer..." -ForegroundColor Yellow
nsisInstallerPath = "installer\winsla-installer.nsi"
if (-not (Test-Path "$nsisInstallerPath")) {
    Write-Error "NSIS script not found at $nsisInstallerPath"
    exit 1
}

# Run makensis with proper paths
Set-Location "installer"

# Generate fresh NSIS installer
$makensisExe = "${env:ProgramFiles(x86)}\NSIS\makensis.exe"
if (-not (Test-Path $makensisExe)) {
    $makensisExe = "makensis.exe" ; Use PATH
}

Write-Host "Running: $makensisExe winsla-installer.nsi" -ForegroundColor Gray

try {
    if ([System.Diagnostics.Process]::Start($makensisExe, "winsla-installer.nsi").WaitForExit()) {
        $installerName = Get-ChildItem "WinSLA-v*Setup.exe" | Select-Object -First 1
        
        if ($null -ne $installerName) {
            Copy-Item $installerName.FullName "$OutputDir\" -Force
            Write-Host "✓ Installer created: WinSLA-v1.0.1-Setup.exe" -ForegroundColor Green
            
            $installerSize = [math]::Round((Get-Item "WinSLA-v1.0.1-Setup.exe").Length / 1MB, 2)
            Write-Host "  Size: ${installerSize} MB" -ForegroundColor Gray
        }
        
        Set-Location $PSScriptRoot
    }
} catch {
    Write-Warning "NSIS compiler not found or error occurred."
    Write-Warning "Please manually run: cd installer && makensis winsla-installer.nsi"
}

# Step 4: Package everything for VM
Write-Host "`n[Step 4/4] Packaging complete installation for VM..." -ForegroundColor Yellow

$zipFolder = Join-Path $OutputDir "Complete-VM-Package.zip"
if (Test-Path $zipFolder) {
    Remove-Item $zipFolder -Force
}

# Copy the installer if it was created
if (Test-Path "WinSLA-v1.0.1-Setup.exe") {
    Copy-Item "WinSLA-v1.0.1-Setup.exe" "$OutputDir\" -Force
}

# Copy README with instructions
$readmeContent = @"
================================================================================
                    WinSLA Windows Dual-Account Authentication
================================================================================

INSTALLATION GUIDE FOR TEST VM:
===============================

This package contains ALL required files for installing WinSLA on your test VM.

OPTION 1: Automatic Installation (Recommended)
----------------------------------------------
1. Extract this ZIP to any location on your VM (e.g., Desktop or C:\Temp\)
2. Navigate to the extracted folder
3. Double-click "WinSLA-v1.0.1-Setup.exe" 
4. Follow the installation wizard
5. ✅ DONE! The installer will:
   - Copy all files to C:\Program Files (x86)\WinSLA
   - Register Credential Provider automatically (no manual registry needed!)
   - Install and start the Windows Service
   - Create shortcuts in Start Menu and Desktop
6. Restart your VM and check login screen

OPTION 2: Manual Deployment (Advanced)
--------------------------------------
If you already have files installed and only want to update the DLL:
1. Extract zip
2. Place new DLL in same folder as deploy.bat
3. Right-click deploy.bat → "Run as Administrator"
4. Follow prompts

FILES INCLUDED:
---------------
- WinSLA-v1.0.1-Setup.exe        : Automatic installer (preferred method)
- Auto-update tools:
  - deploy.bat                   : One-click deployment (run as admin)
  - auto-deploy.ps1              : Advanced automation script  
  - emergency-recovery.ps1       : Restore PasswordProvider if needed
  - troubleshoot-tile.ps1        : Diagnostic tool
  - manual-register.ps1          : Registry helper

IMPORTANT NOTES:
----------------
✅ INSTALLER DOES EVERYTHING AUTOMATICALLY:
   - No need to run PowerShell scripts manually
   - Registry entries are written during installation
   - Service is registered and started automatically
   - All shortcuts are created

⚠️ IMPORTANT TESTING STEPS:
1. After installation, restart the VM first
2. Then log out (or press Ctrl+Alt+Delete) to see login screen
3. Look for dual-account authentication tile
4. If tile doesn't appear, run troubleshoot-tile.ps1

TROUBLESHOOTING:
----------------
If something goes wrong after installation:
- Run emergency-recovery.ps1 to restore previous state
- Use VM snapshot to revert to clean state
- Check install.log file for detailed errors

For full documentation, see DEPLOYMENT-SIMPLE.md in the original repo.

================================================================================
Installation Date: $(Get-Date -Format "yyyy-MM-dd HH:mm:ss")
Package Version: 1.0.1
================================================================================
"@

Set-Content -Path "$OutputDir\README-INSTALL.txt" -Value $readmeContent

# Create simple batch file for easy installation
$batchContent = @"
@echo off
chcp 65001 >nul
title WinSLA Installation Wizard
cls
echo ================================================
echo WinSLA - Dual Account Authentication Setup
echo ================================================
echo.
echo Starting automatic installation...
echo.
echo ⚡ IMPORTANT: Please right-click and choose
echo    "Run as Administrator" before continuing.
echo.
pause
start "" "%~dp0WinSLA-v1.0.1-Setup.exe"
if %ERRORLEVEL% NEQ 0 (
    echo.
    echo Installation failed!
    echo Please check that you ran this as Administrator.
    pause
)
"@

Set-Content -Path "$OutputDir\SETUP-BATCH.bat" -Value $batchContent

# Final summary
Write-Host "`n=========================================" -ForegroundColor Green
Write-Host "Package Creation Complete!" -ForegroundColor Green
Write-Host "=========================================" -ForegroundColor Green
Write-Host "`nYour complete installation package is ready:" -ForegroundColor Cyan
Write-Host "Location: $OutputDir" -ForegroundColor White
Write-Host "`nContents:" -ForegroundColor Yellow
Write-Host "  - WinSLA-v1.0.1-Setup.exe         : Main installer" -ForegroundColor Green
Write-Host "  - SETUP-BATCH.bat                 : Easy launcher" -ForegroundColor Green
Write-Host "  - README-INSTALL.txt              : Instructions" -ForegroundColor Green
Write-Host "  - InstallFiles\                   : Source files" -ForegroundColor Gray
Write-Host ""
Write-Host "Next steps on TEST VM:" -ForegroundColor Cyan
Write-Host "  1. Transfer this entire folder to VM" -ForegroundColor White
Write-Host "  2. Extract and run WINSLA-INSTALLER directly" -ForegroundColor White
Write-Host "  3. Or use SETUP-BATCH.bat for quick launch" -ForegroundColor White
Write-Host "  4. Restart VM and logout to test" -ForegroundColor White
Write-Host ""
Write-Host "The installer handles all registry operations automatically!" -ForegroundColor Magenta
Write-Host ""

if ($AutoRun) {
    explorer.exe "$OutputDir"
}
