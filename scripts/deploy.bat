@echo off
chcp 65001 >nul
title WinSLA CP Auto-Deploy Tool
echo =========================================
echo WinSLA Credential Provider Auto-Deploy
echo =========================================
echo.
echo Starting deployment tool...
echo.
powershell -ExecutionPolicy Bypass -File "%~dp0auto-deploy.ps1" %*
if %ERRORLEVEL% NEQ 0 (
    echo.
    echo Deployment encountered an error. Please check the output above.
    pause
)
