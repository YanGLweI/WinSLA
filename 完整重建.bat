@echo off
title WinSLA v2.2.6 - 完整重建与启动 (管理员)
chcp 65001 >nul
cls

echo ========================================
echo   WinSLA v2.2.6 - 完整重建流程
echo ========================================
echo.

REM Step 1: 停止所有相关进程
echo [Step 1/5] 停止现有进程...
taskkill /F /IM winsla-management.exe 2>nul
timeout /t 2 /nobreak >nul
echo     ✓ 完成
echo.

REM Step 2: 清理前端构建
echo [Step 2/5] 清理前端构建缓存...
cd management_app
if exist node_modules (
    echo     删除 node_modules...
    rmdir /s /q node_modules 2>nul
)
if exist src-tauri\frontend\dist (
    echo     删除 dist 目录...
    rmdir /s /q src-tauri\frontend\dist 2>nul
)
call npm install >nul 2>&1
echo     ✓ 完成
cd ..
echo.

REM Step 3: 清理 Rust 编译
echo [Step 3/5] 清理 Rust 编译缓存...
cd management_app\src-tauri
if exist target\release\winsla-management.exe (
    echo     删除旧版本...
    del target\release\winsla-management.exe 2>nul
)
cargo clean >nul 2>&1
echo     ✓ 完成
echo.

REM Step 4: 重新构建
echo [Step 4/5] 重新构建...
echo     编译 Vue 前端...
cd ..
npm run build >nul 2>&1
echo     编译 Rust 后端...
cd src-tauri
cargo build --release 2>&1 | findstr "Finished"
if %ERRORLEVEL% == 0 (
    echo     ✓ 编译成功!
) else (
    echo     ✗ 编译失败!
    pause
    exit /b 1
)
cd ..\..
echo.

REM Step 5: 以管理员身份启动
echo [Step 5/5] 准备启动应用...
start "" /wait "%~dp0management_app\src-tauri\target\release\winsla-management.exe"

echo.
echo ========================================
echo   启动完成!
echo ========================================
pause
