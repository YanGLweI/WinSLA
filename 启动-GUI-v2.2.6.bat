@echo off
chcp 65001 >nul

REM 检查可执行文件是否存在
if not exist "management_app\src-tauri\target\release\winsla-management.exe" (
    echo [错误] 找不到可执行文件！正在编译...
    
    cd management_app\src-tauri
    cargo build --release
    
    if %ERRORLEVEL% NEQ 0 (
        echo [失败] 编译失败!
        pause
        exit /b 1
    )
    
    echo ✓ 编译成功
)

echo.
echo ================================
echo  启动 WinSLA v2.2.6 管理端
echo ================================
echo.

REM 使用 Start-Process 以管理员身份运行
powershell -Command "Start-Process 'management_app\src-tauri\target\release\winsla-management.exe' -Wait"

echo.
pause
