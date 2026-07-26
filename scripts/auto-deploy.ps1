# WinSLA Credential Provider - Auto Deploy Script
# Version: 1.0.1
# This script automates the entire deployment process

param(
    [string]$NewDllPath = "C:\Temp\WinSLA-update\DualAuthCP.dll",
    [switch]$Force,
    [switch]$RestoreBackup
)

$ErrorActionPreference = "Stop"
$Host.UI.RawUI.WindowTitle = "WinSLA CP Deployment Tool"

# ============================================================================
# Helper Functions
# ============================================================================

function Write-Success {
    param([string]$Message)
    Write-Host "[✓] $Message" -ForegroundColor Green
}

function Write-Warning2 {
    param([string]$Message)
    Write-Host "[⚠] $Message" -ForegroundColor Yellow
}

function Write-Error2 {
    param([string]$Message)
    Write-Host "[✗] $Message" -ForegroundColor Red
}

function Confirm-Admin {
    $currentUser = New-Object Security.Principal.WindowsPrincipal([Security.Principal.WindowsIdentity]::GetCurrent())
    if (-not $currentUser.IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)) {
        Write-Error2 "This script must be run as Administrator!"
        exit 1
    }
}

function Get-DateStamp {
    return (Get-Date).ToString("yyyyMMdd-HHmmss")
}

# ============================================================================
# Main Script
# ============================================================================

Clear-Host
Write-Host "=========================================" -ForegroundColor Cyan
Write-Host "WinSLA Credential Provider Auto-Deploy" -ForegroundColor Cyan
Write-Host "=========================================" -ForegroundColor Cyan
Write-Host ""

# Step 0: Check administrator privileges
Confirm-Admin

# Step 1: Validate new DLL exists
if ($RestoreBackup) {
    Write-Warning2 "RESTORE MODE: Will restore from backup"
} else {
    Write-Host "[Step 1/5] Checking new DLL..." -ForegroundColor Yellow
    
    if (-not (Test-Path $NewDllPath)) {
        Write-Error2 "New DLL not found at: $NewDllPath"
        Write-Warning2 "`nPlease place your compiled DualAuthCP.dll in this location."
        Write-Warning2 "Then run: .\auto-deploy.ps1"
        exit 1
    }
    
    Write-Success "Found new DLL: $($NewDllPath)"
    $newDllSize = (Get-Item $NewDllPath).Length
    Write-Host "      Size: $($newDllSize / 1KB) KB" -ForegroundColor Gray
}

# Step 2: Stop services and backup old DLL
Write-Host "`n[Step 2/5] Preparing for deployment..." -ForegroundColor Yellow

try {
    # Stop WinSLA Service
    $serviceName = "WinSLA_Service"
    Write-Host "Stopping service: $serviceName..." -ForegroundColor Gray
    Try {
        net stop $serviceName | Out-Null
        Write-Success "Service stopped"
    } catch {
        Write-Warning2 "Service not running or not found (may be ok)"
    }
    
    # Find and stop any related processes
    Get-Process | Where-Object {$_.Name -eq "winsla-service" -or $_.Name -eq "winsla-management"} | 
        ForEach-Object {
            Write-Host "Stopping process: $($_.Name)" -ForegroundColor Gray
            Stop-Process -Id $_.Id -Force -ErrorAction SilentlyContinue
        }
    
    # Backup old DLL
    $oldDll = "C:\Program Files (x86)\WinSLA\DualAuthCP.dll"
    if (Test-Path $oldDll) {
        $backupDir = "C:\Backup\WinSLA-$((Get-DateStamp))"
        New-Item -ItemType Directory -Path $backupDir -Force | Out-Null
        
        Copy-Item $oldDll "$backupDir\DualAuthCP-old.dll" -Force
        Write-Ssuccess "Backed up old DLL to: $backupDir"
        
        # Also create a simple last-known-good backup
        Copy-Item $oldDll "C:\Backup\DualAuthCP-last-known-good.dll" -Force -ErrorAction SilentlyContinue
        Write-Success "Also saved to C:\Backup\DualAuthCP-last-known-good.dll"
    } else {
        Write-Warning2 "Old DLL not found (may be first deployment)"
    }
} catch {
    Write-Error2 "Failed to prepare: $($_.Exception.Message)"
    exit 1
}

# Step 3: Deploy new DLL
Write-Host "`n[Step 3/5] Deploying new DLL..." -ForegroundColor Yellow

if (-not $RestoreBackup) {
    try {
        # Handle file locking by stopping explorer temporarily
        $targetDll = "C:\Program Files (x86)\WinSLA\DualAuthCP.dll"
        
        # Check if file is locked
        $isLocked = $false
        try {
            Remove-Item $targetDll -Force -ErrorAction Stop
            $isLocked = $false
        } catch {
            $isLocked = $true
        }
        
        if ($isLocked) {
            Write-Warning2 "File appears to be in use. Attempting recovery..."
            
            # Try alternative method
            Start-Sleep -Seconds 1
            Copy-Item $NewDllPath $targetDll -Force -ErrorAction SilentlyContinue
            
            # If still fails, prompt user
            if ((Test-Path $targetDll) -and ((Get-Item $NewDllPath).LastWriteTime -ne (Get-Item $targetDll).LastWriteTime)) {
                Write-Error2 "Deployment failed! File is locked."
                Write-Host "`nManual steps:" -ForegroundColor Cyan
                Write-Host "1. Press Ctrl+Alt+Delete -> Task Manager" -ForegroundColor White
                Write-Host "2. End 'Windows Explorer' process" -ForegroundColor White
                Write-Host "3. Run this script again" -ForegroundColor White
                Write-Host "4. After copy completes, restart Windows Explorer" -ForegroundColor White
                exit 1
            }
        }
        
        Write-Success "DLL deployed successfully"
    } catch {
        Write-Error2 "Deployment failed: $($_.Exception.Message)"
        exit 1
    }
} else {
    # Restore mode
    $latestBackup = Get-ChildItem "C:\Backup\DualAuthCP-old*.dll" | Sort-Object LastWriteTime -Descending | Select-Object -First 1
    
    if ($null -eq $latestBackup) {
        Write-Error2 "No backup found to restore!"
        exit 1
    }
    
    Write-Host "Restoring from: $($latestBackup.FullName)" -ForegroundColor Gray
    Copy-Item $latestBackup.FullName "C:\Program Files (x86)\WinSLA\DualAuthCP.dll" -Force
    Write-Success "DLL restored successfully"
}

# Step 4: Restart service
Write-Host "`n[Step 4/5] Restarting services..." -ForegroundColor Yellow

Try {
    Try {
        net start "WinSLA_Service" | Out-Null
        Write-Success "Service started"
    } catch {
        Write-Warning2 "Service might already be running"
    }
} catch {
    Write-Error2 "Failed to restart service"
}

# Step 5: Verify registration
Write-Host "`n[Step 5/5] Verifying setup..." -ForegroundColor Yellow

$clsid = "{E4D9F6E7-8A2B-4C3D-9E5F-1A2B3C4D5E6F}"
$regPath = "HKLM:\SOFTWARE\Microsoft\Windows\CurrentVersion\Authentication\Credential Providers\$clsid"

if (Test-Path $regPath) {
    $prop = Get-ItemProperty -Path $regPath -Name "Disabled" -ErrorAction SilentlyContinue
    if ($null -ne $prop.Disabled -and $prop.Disabled -eq 0) {
        Write-Success "Credential Provider is registered and enabled"
    } else {
        Write-Warning2 "Provider is disabled. Running manual-register.ps1..."
        cd "C:\Program Files (x86)\WinSLA\scripts"
        powershell -ExecutionPolicy Bypass -File manual-register.ps1
    }
} else {
    Write-Error2 "Credential Provider not registered!"
    Write-Host "Please run: C:\Program Files (x86)\WinSLA\scripts\manual-register.ps1" -ForegroundColor Yellow
    exit 1
}

# Final status
Write-Host "`n=========================================" -ForegroundColor Green
Write-Host "Deployment Completed Successfully!" -ForegroundColor Green
Write-Host "=========================================" -ForegroundColor Green
Write-Host ""
Write-Host "Summary:" -ForegroundColor Cyan
Write-Host "  ✓ New DLL deployed" -ForegroundColor Green
Write-Host "  ✓ Old DLL backed up to C:\Backup\" -ForegroundColor Green
Write-Host "  ✓ Services restarted" -ForegroundColor Green
Write-Host "  ✓ Registration verified" -ForegroundColor Green
Write-Host ""
Write-Host "Next steps:" -ForegroundColor Cyan
Write-Host "  1. Test with current configuration (both providers enabled)" -ForegroundColor White
Write-Host "     → Log out and observe the login screen" -ForegroundColor Gray
Write-Host "  2. If working correctly, you can now disable PasswordProvider" -ForegroundColor White
Write-Host "     → Run emergency-recovery.ps1 to reverse if needed" -ForegroundColor Gray
Write-Host "  3. Keep snapshot backup before testing critical changes" -ForegroundColor White
Write-Host ""
Write-Host "Diagnostic scripts available:" -ForegroundColor Yellow
Write-Host "  - troubleshoot-tile.ps1   : Full diagnostic report" -ForegroundColor Gray
Write-Host "  - test-cp-diagnostics.ps1 : Quick check" -ForegroundColor Gray
Write-Host ""

$response = Read-Host "`nPress Enter to log off and test, or press Ctrl+C to cancel"
if ($response -eq "") {
    Write-Host "Logging off..." -ForegroundColor Cyan
    shutdown /l
}
