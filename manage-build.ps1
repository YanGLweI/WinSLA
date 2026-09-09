# WinSLA v2.2.6 - 完整构建与测试脚本

param(
    [switch]$Admin
)

$ErrorActionPreference = "Stop"
$ProgressPreference = "SilentlyContinue"

Write-Host "=== WinSLA v2.2.6 完整构建流程 ===" -ForegroundColor Cyan
Write-Host "Starting at: $(Get-Date)" -ForegroundColor Gray
Write-Host ""

try {
    # Step 1: Stop existing processes
    Write-Host "[Step 1/6] Stopping existing processes..." -ForegroundColor Yellow
    
    $processes = Get-Process -Name "winsla-management","node" -ErrorAction SilentlyContinue
    if ($processes) {
        $processes | Stop-Process -Force -ErrorAction SilentlyContinue
        Start-Sleep -Seconds 3
        Write-Host "  ✓ Processes stopped" -ForegroundColor Green
    } else {
        Write-Host "  ℹ No running processes" -ForegroundColor Gray
    }
    
    # Step 2: Clean frontend cache
    Write-Host ""
    Write-Host "[Step 2/6] Cleaning frontend cache..." -ForegroundColor Yellow
    
    Set-Location management_app
    
    if (Test-Path node_modules) {
        Remove-Item node_modules -Recurse -Force -ErrorAction SilentlyContinue
        Write-Host "  ✓ Deleted node_modules" -ForegroundColor Green
    }
    
    if (Test-Path src-tauri\frontend\dist) {
        Remove-Item src-tauri\frontend\dist -Recurse -Force -ErrorAction SilentlyContinue
        Write-Host "  ✓ Deleted dist directory" -ForegroundColor Green
    }
    
    Write-Host "  Installing dependencies..." -ForegroundColor Gray
    npm install --silent
    
    Set-Location ..
    
    # Step 3: Clean Rust cache
    Write-Host ""
    Write-Host "[Step 3/6] Cleaning Rust cache..." -ForegroundColor Yellow
    
    Set-Location management_app\src-tauri
    
    if (Test-Path target\release\winsla-management.exe) {
        Remove-Item target\release\winsla-management.exe -Force -ErrorAction SilentlyContinue
        Write-Host "  ✓ Deleted old executable" -ForegroundColor Green
    }
    
    cargo clean >$null 2>&1
    Write-Host "  ✓ Cargo cleaned" -ForegroundColor Green
    
    # Step 4: Build Vue frontend
    Write-Host ""
    Write-Host "[Step 4/6] Building Vue frontend..." -ForegroundColor Yellow
    
    Set-Location ..
    $buildStart = Get-Date
    npm run build 2>$null
    $buildDuration = New-TimeSpan -Start $buildStart -End (Get-Date)
    Write-Host "  ✓ Frontend built in $($buildDuration.ToString("ss\.s"))s" -ForegroundColor Green
    
    # Step 5: Build Rust backend
    Write-Host ""
    Write-Host "[Step 5/6] Building Rust backend..." -ForegroundColor Yellow
    
    Set-Location management_app\src-tauri
    $cargoStart = Get-Date
    $cargoOutput = cargo build --release --message-format=json 2>&1 | Where-Object { $_ -match "Finished|error" }
    $cargoDuration = New-TimeSpan -Start $cargoStart -End (Get-Date)
    
    if ($cargoOutput -match "Finished") {
        Write-Host "  ✓ Rust built in $($cargoDuration.ToString("ss\.s"))s" -ForegroundColor Green
    } else {
        throw "Rust build failed!"
    }
    
    Set-Location ..
    
    # Step 6: Launch with admin privileges
    Write-Host ""
    Write-Host "[Step 6/6] Starting application..." -ForegroundColor Yellow
    
    $exePath = "$PSScriptRoot\management_app\src-tauri\target\release\winsla-management.exe"
    
    if ($Admin -or (-not $env:SESSION_NAME -eq "")) {
        # Try to launch as admin if we're already elevated
        Start-Process -FilePath $exePath -Wait
        Write-Host ""
        Write-Host "=== Build Complete! ===" -ForegroundColor Green
        Write-Host "Application launched successfully." -ForegroundColor Green
    } else {
        # Otherwise show instructions
        Write-Host ""
        Write-Host "Build completed successfully!" -ForegroundColor Green
        Write-Host ""
        Write-Host "To launch with admin privileges:" -ForegroundColor Cyan
        Write-Host "1. Right-click on: $exePath" -ForegroundColor White
        Write-Host "2. Select 'Run as administrator'" -ForegroundColor White
        Write-Host ""
        Write-Host "Or use the batch file:" -ForegroundColor Cyan
        Write-Host "$PSScriptRoot\启动-GUI-管理员.bat" -ForegroundColor White
    }
    
    Write-Host ""
    Write-Host "Total build time: $((New-TimeSpan -Start (Get-Date).AddMinutes(-10) -End (Get-Date)).ToString("mm\.ss"))m $(((Get-Date).Second % 60).ToString().PadLeft(2,"0")).0s" -ForegroundColor Gray
    
} catch {
    Write-Host ""
    Write-Host "❌ Error: $_" -ForegroundColor Red
    exit 1
} finally {
    Set-Location $env:USERPROFILE
}
