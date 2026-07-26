# Package Creator Script - Creates deployment ZIP file
# Run this on development machine after compiling DLL

param(
    [string]$SourceDll = "target\release\DualAuthCP.dll",
    [string]$OutputDir = "C:\Temp",
    [switch]$CreateZip,
    [Switch]$OnlyPackage
)

$ErrorActionPreference = "Stop"

# Ensure output directory exists
if (-not (Test-Path $OutputDir)) {
    New-Item -ItemType Directory -Path $OutputDir | Out-Null
}

# Create temporary folder for package
$dateStamp = Get-Date -Format "yyyyMMdd-HHmmss"
$packageName = "WinSLA-Deploy-Package-$dateStamp"
$tempFolder = Join-Path $OutputDir "$packageName"
Remove-Item $tempFolder -Recurse -Force -ErrorAction SilentlyContinue
New-Item -ItemType Directory -Path $tempFolder | Out-Null

Write-Host "Creating deployment package..." -ForegroundColor Cyan

# Step 1: Copy new DLL
Write-Host "[1/4] Copying compiled DLL..." -ForegroundColor Yellow
if (-not (Test-Path $SourceDll)) {
    Write-Host "ERROR: DLL not found at $SourceDll" -ForegroundColor Red
    Write-Host "Please compile first: cargo build --release --package cp_provider" -ForegroundColor Gray
    exit 1
}

Copy-Item $SourceDll (Join-Path $tempFolder "DualAuthCP.dll") -Force
$dllSize = [math]::Round((Get-Item $SourceDll).Length / 1KB, 2)
Write-Host "  ✓ Copied: $SourceSql (${dllSize} KB)" -ForegroundColor Green

# Step 2: Copy scripts
Write-Host "`n[2/4] Copying scripts..." -ForegroundColor Yellow
$scriptsToCopy = @("auto-deploy.ps1", "deploy.bat", "manual-register.ps1", 
                   "emergency-recovery.ps1", "troubleshoot-tile.ps1", "test-cp-diagnostics.ps1")

foreach ($script in $scriptsToCopy) {
    $source = "scripts\$script"
    if (Test-Path $source) {
        Copy-Item $source (Join-Path $tempFolder $script) -Force
        Write-Host "  ✓ $script" -ForegroundColor Gray
    } else {
        Write-Host "  ⚠ $script (skipped - not found)" -ForegroundColor DarkGray
    }
}

# Copy README and documentation
Write-Host "`n[3/4] Copying documentation..." -ForegroundColor Yellow
Copy-Item "DEPLOYMENT-PACKAGE.md" (Join-Path $tempFolder "README.md") -Force
Write-Host "  ✓ README.md created" -ForegroundColor Green

# Add quick-start notes
$quickStart = @"
# WinSLA Credential Provider - Quick Deploy Instructions

## To Deploy on Test VM:

1. Transfer entire `.$packageName` folder to test VM

2. On VM: Place your NEW DualAuthCP.dll in the same folder as deploy.bat

3. Right-click "deploy.bat" and select "Run as Administrator"

4. Follow prompts (or press Enter to log out and test)

## Files Included:
- DualAuthCP.dll          : Your compiled DLL (replace this one)
- deploy.bat              : Double-click to run deployment
- auto-deploy.ps1         : Main automation script
- emergency-recovery.ps1  : Restore old version if needed

## Troubleshooting:
If you get "file in use" errors:
- Stop Windows Explorer temporarily
- Or boot into Safe Mode
- See full docs: DEPLOYMENT-PACKAGE.md

"@
Set-Content -Path (Join-Path $tempFolder "QUICK-START.txt") -Value $quickStart

# Step 4: Create ZIP (if requested or if only packaging)
if ($CreateZip -or ($null -eq $CreateZip -and $null -ne $env:ZIP_CREATE)) {
    Write-Host "`n[4/4] Creating ZIP archive..." -ForegroundColor Yellow
    
    $zipFileName = "$PackageName.zip"
    $zipPath = Join-Path $OutputDir $zipFileName
    
    # Create ZIP using PowerShell
    Add-Type -AssemblyName System.IO.Compression.FileSystem
    [System.IO.Compression.ZipFile]::CreateFromDirectory($tempFolder, $zipPath)
    
    Write-Host "  ✓ Created: $zipPath" -ForegroundColor Green
    $zipSize = [math]::Round((Get-Item $zipPath).Length / 1024, 2)
    Write-Host "  Size: ${zipSize} KB" -ForegroundColor Gray
    
    # Optional: Clean up temp folder
    Remove-Item $tempFolder -Recurse -Force
    
    Write-Host "`n=========================================" -ForegroundColor Green
    Write-Host "Deployment Package Ready!" -ForegroundColor Green
    Write-Host "=========================================" -ForegroundColor Green
    Write-Host "Location: $zipPath" -ForegroundColor Cyan
    Write-Host "Transfer this to your test VM and extract it." -ForegroundColor White
} elseif ($null -eq $CreateZip) {
    # Just create folder without zipping (for debugging/verification)
    Write-Host "`nPackage created at:" -ForegroundColor Green
    Write-Host $tempFolder -ForegroundColor Cyan
    Write-Host "`nTo create ZIP manually:" -ForegroundColor Yellow
    Write-Host "Compress-Archive -Path $tempFolder -DestinationPath $(Join-Path $OutputDir '$packageName.zip') -Force" -ForegroundColor Gray
}
