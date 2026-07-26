@echo off
chcp 65001 >nul
title WinSLA v1.0.2 Installer - Run as Administrator!
cls

echo ================================================
echo   WinSLA v1.0.2 Dual-Account Authentication
echo ================================================
echo.
echo IMPORTANT: This installer MUST be run as Administrator!
echo.
echo Why? Because it needs to write registry keys to HKLM.
echo Without admin rights, Credential Provider registration will FAIL.
echo.
echo You will see a UAC prompt right after clicking OK.
echo Click "Yes" to allow the installer to run with admin privileges.
echo.
echo -----------------------------------------------
echo THE BIG NEWS: NSIS INSTALLER NOW WORKS AUTOMATICALLY!
echo -----------------------------------------------
echo No more need to manually run PowerShell scripts!
echo The installer writes all registry entries itself.
echo.
pause

echo.
echo Starting installation...
echo.

start /wait "" "%~dp0WinSLA-v1.0.2-Setup.exe" /S

if %ERRORLEVEL% NEQ 0 (
    echo.
    echo !!! Installation failed with error code: %ERRORLEVEL% !!!
    echo.
    pause
    exit /b %ERRORLEVEL%
)

echo.
echo ================================================
echo Installation completed successfully!
echo ================================================
echo.
echo Next steps:
echo.
echo 1. RESTART your computer
echo    (or press Ctrl+Alt+Delete and log out)
echo.
echo 2. Look for the WinSLA dual-account authentication tile
echo    on the login screen
echo.
echo 3. If you don't see the tile, run:
echo    .\scripts\troubleshoot-tile.ps1
echo.
echo To validate registry was written automatically:
echo    reg query "HKLM\SOFTWARE\Microsoft\Windows\CurrentVersion\Authentication\Credential Providers\{E4D9F6E7-8A2B-4C3D-9E5F-1A2B3C4D5E6F}"
echo.
echo Should show: Disabled    REG_DWORD    0
echo.

pause
