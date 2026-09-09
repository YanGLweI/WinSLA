@echo off
title WinSLA v2.2.6 - 管理员启动
chcp 65001 >nul
cls

echo ================================
echo  WinSLA v2.2.6 管理端
echo  (管理员权限)
echo ================================
echo.

REM 检查文件是否存在
if not exist "target\release\winsla-management.exe" (
    echo [错误] 找不到可执行文件！
    echo 路径：%~dp0target\release\winsla-management.exe
    echo.
    pause
    exit /b 1
)

REM 使用 PowerShell 以管理员身份运行
powershell -Command "Start-Process -FilePath '%~dp0target\release\winsla-management.exe' -Verb RunAs -Wait"

echo.
echo [完成] 应用已关闭或您取消了授权
pause
