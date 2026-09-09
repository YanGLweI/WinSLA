# WinSLA v2.2.6 Quick Start Script

param(
    [switch]$Dev,
    [switch]$Preview,
    [switch]$Build
)

Push-Location "C:\Users\YLW\Documents\PJ\WinSLA\management_app"

if ($Dev.IsPresent) {
    Write-Host "Starting Vite development server..." -ForegroundColor Cyan
    npm run dev
} elseif ($Preview.IsPresent) {
    Write-Host "Starting preview server..." -ForegroundColor Cyan
    npm run preview
} elseif ($Build.IsPresent) {
    Write-Host "Building production version..." -ForegroundColor Cyan
    npm run build
    
    Pop-Location
    cd ..
    
    Write-Host "Compiling Rust management application..." -ForegroundColor Cyan
    cargo build --release -p winsla-management
    
    if ($?) {
        Write-Host "`n✓ Build completed successfully!" -ForegroundColor Green
        Write-Host "Executable: .\target\release\winsla-management.exe" -ForegroundColor White
    } else {
        Write-Host "`n✗ Build failed!" -ForegroundColor Red
        exit 1
    }
} else {
    Write-Host "Usage:" -ForegroundColor Yellow
    Write-Host "  .\quick-start.ps1 -Dev         # Start development environment" -ForegroundColor White
    Write-Host "  .\quick-start.ps1 -Preview    # Start preview environment" -ForegroundColor White
    Write-Host "  .\quick-start.ps1 -Build      # Build production version" -ForegroundColor White
}
