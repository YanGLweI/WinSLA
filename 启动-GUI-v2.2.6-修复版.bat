@echo off
chcp 65001 >nul

REM 检查可执行文件是否存在
if not exist "target\release\winsla-management.exe" (
    echo [错误] 找不到可执行文件！正在重新编译...
    
    cd management_app\src-tauri
    cargo build --release
    
    if %ERRORLEVEL% NEQ 0 (
        echo [失败] 编译失败!
        pause
        exit /b 1
    )
    
    echo ✓ 编译成功
) else (
    echo ✓ 找到已编译的可执行文件
)

echo.
echo ================================
echo  启动 WinSLA v2.2.6 管理端
echo  (Bug #1 & Bug #2 已修复)
echo ================================
echo.

REM 使用 PowerShell 以管理员身份运行
powershell -Command "Start-Process 'C:\Users\YLW\Documents\PJ\WinSLA\target\release\winsla-management.exe' -Wait"

echo.
pause
