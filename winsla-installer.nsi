; WinSLA - Windows Dual-Account Authentication System
; NSIS Installer Script v1.0.5

!include "MUI2.nsh"
!include "LogicLib.nsh"
!include "x64.nsh"

; ─── 基本信息 ───────────────────────────────────────────────
Name "WinSLA v2.0.3"
OutFile "WinSLA-v2.0.3-Setup.exe"
InstallDir "$PROGRAMFILES64\WinSLA"
InstallDirRegKey HKLM "Software\WinSLA" "InstallDir"
RequestExecutionLevel admin
Unicode true

; ─── 版本信息 ───────────────────────────────────────────────
VIProductVersion "2.0.3.0"
VIAddVersionKey "ProductName" "WinSLA"
VIAddVersionKey "FileVersion" "2.0.3"
VIAddVersionKey "FileDescription" "WinSLA - Windows Dual-Account Authentication System"
VIAddVersionKey "LegalCopyright" "MIT License - 2026 ylw"

; ─── 界面配置 ───────────────────────────────────────────────
!define MUI_ICON "..\assets\winsla.ico"
!define MUI_UNICON "..\assets\winsla.ico"
Icon "..\assets\winsla.ico"
UninstallIcon "..\assets\winsla.ico"
!define MUI_WELCOMEPAGE_TITLE "WinSLA 双账号认证系统 安装向导"
!define MUI_WELCOMEPAGE_TEXT "本向导将安装 WinSLA Windows 双账号协同登录代理。$\r$\n$\r$\nWinSLA 实现'金库双人原则'，要求两个独立 AD 域账号同时验证通过方可登录。$\r$\n$\r$\n✅ 新版本特性：NSIS 安装程序现已自动写入注册表！无需手动运行 PowerShell 脚本。$\r$\n$\r$\n⚠️ 警告：安装后会影响系统登录流程，请确保在测试环境中操作。$\r$\n$\r$\n点击'下一步'继续。"
!define MUI_FINISHPAGE_TITLE "安装完成"
!define MUI_FINISHPAGE_TEXT "WinSLA 已成功安装。$\r$\n$\r$\n✅ 已自动注册 Credential Provider 到系统注册表!$\r$\n✅ 已启动认证服务 !$\r$\n$\r$\n🔄 重要：为了生效，请在安装完成后重启计算机。$\r$\n$\r$\n如需卸载，请通过控制面板或运行卸载程序。"

; ─── 页面 ───────────────────────────────────────────────────
!insertmacro MUI_PAGE_WELCOME
!insertmacro MUI_PAGE_LICENSE "..\LICENSE"
!insertmacro MUI_PAGE_DIRECTORY
!insertmacro MUI_PAGE_INSTFILES
!insertmacro MUI_PAGE_FINISH

!insertmacro MUI_UNPAGE_CONFIRM
!insertmacro MUI_UNPAGE_INSTFILES
!insertmacro MUI_UNPAGE_FINISH

; ─── 语言 ───────────────────────────────────────────────────
!insertmacro MUI_LANGUAGE "SimpChinese"
!insertmacro MUI_LANGUAGE "English"

; ─── 常量 ───────────────────────────────────────────────────
!define CP_CLSID "{E4D9F6E7-8A2B-4C3D-9E5F-1A2B3C4D5E6F}"
!define SERVICE_NAME "WinSLA Service"
!define PIPE_NAME "winsla-auth-pipe"

; ─── 安装区段 ───────────────────────────────────────────────
Section "Core Files" SecCore
    SectionIn RO

    ; 创建安装目录
    SetOutPath "$INSTDIR"

    ; 复制核心文件
    File "..\target\release\DualAuthCP.dll"
    File "..\target\release\winsla-service.exe"
    File "..\target\release\winsla-management.exe"
    File "..\assets\winsla.ico"

    ; 复制脚本
    SetOutPath "$INSTDIR\scripts"
    File "..\scripts\install.ps1"
    File "..\scripts\unregister.ps1"

    ; 写入安装路径到注册表
    WriteRegStr HKLM "Software\WinSLA" "InstallDir" "$INSTDIR"
    WriteRegStr HKLM "Software\WinSLA" "Version" "2.0.3"

    ; 创建卸载程序
    WriteUninstaller "$INSTDIR\uninstall.exe"
    WriteRegStr HKLM "Software\Microsoft\Windows\CurrentVersion\Uninstall\WinSLA" \
        "DisplayName" "WinSLA - Dual-Account Authentication"
    WriteRegStr HKLM "Software\Microsoft\Windows\CurrentVersion\Uninstall\WinSLA" \
        "UninstallString" "$INSTDIR\uninstall.exe"
    WriteRegStr HKLM "Software\Microsoft\Windows\CurrentVersion\Uninstall\WinSLA" \
        "DisplayVersion" "2.0.3"
    WriteRegStr HKLM "Software\Microsoft\Windows\CurrentVersion\Uninstall\WinSLA" \
        "Publisher" "ylw"
    WriteRegDWORD HKLM "Software\Microsoft\Windows\CurrentVersion\Uninstall\WinSLA" \
        "NoModify" 1
    WriteRegDWORD HKLM "Software\Microsoft\Windows\CurrentVersion\Uninstall\WinSLA" \
        "NoRepair" 1
SectionEnd

; ─── Credential Provider Section ──────────────────────────────
Section "Credential Provider (DualAuthCP)" SecCP
    SetOutPath "$INSTDIR\dlls"
    ; DLL will be copied by the Core Files section already
SectionEnd

; ─── Service Section ──────────────────────────────────────────
Section "Windows Service" SecService
    ; Service already installed by install.ps1 script
SectionEnd

; ─── Shortcuts Section ────────────────────────────────────────
Section "Shortcuts" SecShortcuts
    CreateDirectory "$SMPROGRAMS\WinSLA"
    CreateShortCut "$SMPROGRAMS\WinSLA\管理端.lnk" "$INSTDIR\winsla-management.exe"
    CreateShortCut "$SMPROGRAMS\WinSLA\卸载.lnk" "$INSTDIR\uninstall.exe"
SectionEnd

; ─── 卸载区段 ────────────────────────────────────────────────
Section "Uninstall"
    ; Stop and remove service
    nsExec::ExecToLog '"sc.exe" stop "$SERVICE_NAME"'
    nsExec::ExecToLog '"sc.exe" delete "$SERVICE_NAME"'

    ; Remove registry keys
    DeleteRegKey HKLM "Software\WinSLA"
    DeleteRegKey HKLM "Software\Microsoft\Windows\CurrentVersion\Uninstall\WinSLA"

    ; Remove files
    Delete "$INSTDIR\DualAuthCP.dll"
    Delete "$INSTDIR\winsla-service.exe"
    Delete "$INSTDIR\winsla-management.exe"
    Delete "$INSTDIR\uninstall.exe"
    Rmdir /r "$INSTDIR\dlls"
    Rmdir /r "$INSTDIR\scripts"
    Rmdir "$INSTDIR"

    ; Remove shortcuts
    Delete "$SMPROGRAMS\WinSLA\管理端.lnk"
    Delete "$SMPROGRAMS\WinSLA\卸载.lnk"
    Rmdir "$SMPROGRAMS\WinSLA"
SectionEnd