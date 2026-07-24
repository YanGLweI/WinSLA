; WinSLA - Windows Dual-Account Authentication System
; NSIS Installer Script v0.0.1
; Requires: NSIS 3.x

!include "MUI2.nsh"
!include "LogicLib.nsh"
!include "x64.nsh"

; ─── 基本信息 ───────────────────────────────────────────────
Name "WinSLA v0.0.1"
OutFile "WinSLA-v0.0.1-Setup.exe"
InstallDir "$SYSDIR\winsla"
InstallDirRegKey HKLM "Software\WinSLA" "InstallDir"
RequestExecutionLevel admin
Unicode true

; ─── 版本信息 ───────────────────────────────────────────────
VIProductVersion "0.0.1.0"
VIAddVersionKey "ProductName" "WinSLA"
VIAddVersionKey "FileVersion" "0.0.1"
VIAddVersionKey "FileDescription" "Windows Dual-Account Authentication System"
VIAddVersionKey "LegalCopyright" "MIT License - 2026 WinSLA Contributors"

; ─── 界面配置 ───────────────────────────────────────────────
!define MUI_ICON "${NSISDIR}\Contrib\Graphics\Icons\modern-install.ico"
!define MUI_UNICON "${NSISDIR}\Contrib\Graphics\Icons\modern-uninstall.ico"
!define MUI_WELCOMEPAGE_TITLE "WinSLA 双账号认证系统 安装向导"
!define MUI_WELCOMEPAGE_TEXT "本向导将安装 WinSLA Windows 双账号协同登录代理。$\r$\n$\r$\nWinSLA 实现'金库双人原则'，要求两个独立 AD 域账号同时验证通过方可登录。$\r$\n$\r$\n⚠️ 警告：安装后会影响系统登录流程，请确保在测试环境中操作。$\r$\n$\r$\n点击'下一步'继续。"
!define MUI_FINISHPAGE_TITLE "安装完成"
!define MUI_FINISHPAGE_TEXT "WinSLA 已成功安装。$\r$\n$\r$\n已注册 Credential Provider 并启动认证服务。$\r$\n下次登录时将显示双账号认证界面。$\r$\n$\r$\n如需卸载，请通过控制面板或运行卸载程序。"

; ─── 页面 ───────────────────────────────────────────────────
!insertmacro MUI_PAGE_WELCOME
!insertmacro MUI_PAGE_LICENSE "..\LICENSE"
!insertmacro MUI_PAGE_DIRECTORY
!insertmacro MUI_PAGE_INSTFILES
!insertmacro MUI_PAGE_FINISH

!insertmacro MUI_UNPAGE_CONFIRM
!insertmacro MUI_UNPAGE_INSTFILES

; ─── 语言 ───────────────────────────────────────────────────
!insertmacro MUI_LANGUAGE "SimpChinese"
!insertmacro MUI_LANGUAGE "English"

; ─── 常量 ───────────────────────────────────────────────────
!define CP_CLSID "{A5A5A5A5-B6B6-C7C7-D8D8-E9E9E9E9E9E9}"
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

    ; 复制脚本
    SetOutPath "$INSTDIR\scripts"
    File "..\scripts\install.ps1"
    File "..\scripts\unregister.ps1"

    ; 写入安装路径到注册表
    WriteRegStr HKLM "Software\WinSLA" "InstallDir" "$INSTDIR"
    WriteRegStr HKLM "Software\WinSLA" "Version" "0.0.1"

    ; 创建卸载程序
    WriteUninstaller "$INSTDIR\uninstall.exe"
    WriteRegStr HKLM "Software\Microsoft\Windows\CurrentVersion\Uninstall\WinSLA" \
        "DisplayName" "WinSLA - Dual-Account Authentication"
    WriteRegStr HKLM "Software\Microsoft\Windows\CurrentVersion\Uninstall\WinSLA" \
        "UninstallString" "$INSTDIR\uninstall.exe"
    WriteRegStr HKLM "Software\Microsoft\Windows\CurrentVersion\Uninstall\WinSLA" \
        "DisplayVersion" "0.0.1"
    WriteRegStr HKLM "Software\Microsoft\Windows\CurrentVersion\Uninstall\WinSLA" \
        "Publisher" "WinSLA Contributors"
    WriteRegDWORD HKLM "Software\Microsoft\Windows\CurrentVersion\Uninstall\WinSLA" \
        "NoModify" 1
    WriteRegDWORD HKLM "Software\Microsoft\Windows\CurrentVersion\Uninstall\WinSLA" \
        "NoRepair" 1
SectionEnd

Section "Register Credential Provider" SecCP
    ; 注册 CP CLSID
    WriteRegStr HKLM "SOFTWARE\Classes\CLSID\${CP_CLSID}" "" "WinSLA Dual-Auth Credential Provider"
    WriteRegStr HKLM "SOFTWARE\Classes\CLSID\${CP_CLSID}\InprocServer32" "" "$INSTDIR\DualAuthCP.dll"
    WriteRegStr HKLM "SOFTWARE\Classes\CLSID\${CP_CLSID}\InprocServer32" "ThreadingModel" "Apartment"

    ; 注册到 Credential Providers
    WriteRegStr HKLM "SOFTWARE\Microsoft\Windows\CurrentVersion\Authentication\Credential Providers\${CP_CLSID}" "" "WinSLA Dual-Auth"
    WriteRegDWORD HKLM "SOFTWARE\Microsoft\Windows\CurrentVersion\Authentication\Credential Providers\${CP_CLSID}" "Disabled" 0

    DetailPrint "Credential Provider 已注册: ${CP_CLSID}"
SectionEnd

Section "Install Windows Service" SecService
    ; 使用 sc.exe 注册服务 (标准 Windows 方式)
    nsExec::ExecToLog 'sc.exe create "${SERVICE_NAME}" binPath= "$INSTDIR\winsla-service.exe --service" start= auto'
    Pop $0

    ${If} $0 == 0
        DetailPrint "WinSLA Service 已注册"
        ; 启动服务
        nsExec::ExecToLog 'net start "${SERVICE_NAME}"'
        Pop $0
        ${If} $0 == 0
            DetailPrint "WinSLA Service 已启动"
        ${Else}
            DetailPrint "服务已注册但启动失败 (错误码: $0)，可稍后手动启动"
        ${EndIf}
    ${Else}
        DetailPrint "服务注册失败 (错误码: $0)，可手动运行: sc.exe create ..."
    ${EndIf}
SectionEnd

Section "Start Menu Shortcuts" SecShortcuts
    CreateDirectory "$SMPROGRAMS\WinSLA"
    CreateShortcut "$SMPROGRAMS\WinSLA\WinSLA Management.lnk" "$INSTDIR\winsla-management.exe"
    CreateShortcut "$SMPROGRAMS\WinSLA\Uninstall WinSLA.lnk" "$INSTDIR\uninstall.exe"

    ; 桌面快捷方式
    CreateShortcut "$DESKTOP\WinSLA Management.lnk" "$INSTDIR\winsla-management.exe" "" "$INSTDIR\winsla-management.exe" 0
SectionEnd

; ─── 卸载区段 ───────────────────────────────────────────────
Section "Uninstall"
    ; 停止并删除服务
    nsExec::ExecToLog 'net stop "${SERVICE_NAME}"'
    nsExec::ExecToLog 'sc delete "${SERVICE_NAME}"'

    ; 注销 Credential Provider
    DeleteRegKey HKLM "SOFTWARE\Microsoft\Windows\CurrentVersion\Authentication\Credential Providers\${CP_CLSID}"
    DeleteRegKey HKLM "SOFTWARE\Classes\CLSID\${CP_CLSID}"

    ; 删除注册表
    DeleteRegKey HKLM "Software\WinSLA"
    DeleteRegKey HKLM "Software\Microsoft\Windows\CurrentVersion\Uninstall\WinSLA"

    ; 删除开始菜单和桌面快捷方式
    Delete "$DESKTOP\WinSLA Management.lnk"
    Delete "$SMPROGRAMS\WinSLA\WinSLA Management.lnk"
    Delete "$SMPROGRAMS\WinSLA\Uninstall WinSLA.lnk"
    RMDir "$SMPROGRAMS\WinSLA"

    ; 删除文件
    Delete "$INSTDIR\DualAuthCP.dll"
    Delete "$INSTDIR\winsla-service.exe"
    Delete "$INSTDIR\winsla-management.exe"
    Delete "$INSTDIR\scripts\install.ps1"
    Delete "$INSTDIR\scripts\unregister.ps1"
    Delete "$INSTDIR\uninstall.exe"
    RMDir "$INSTDIR\scripts"
    RMDir "$INSTDIR"

    DetailPrint "WinSLA 已完全卸载"
SectionEnd

; ─── 描述 ───────────────────────────────────────────────────
LangString DESC_SecCore ${LANG_SIMPCHINESE} "核心文件 (DLL、服务、管理端、脚本)"
LangString DESC_SecCP ${LANG_SIMPCHINESE} "注册 Credential Provider 到系统 (影响登录界面)"
LangString DESC_SecService ${LANG_SIMPCHINESE} "安装并启动 WinSLA Windows 认证服务"
LangString DESC_SecShortcuts ${LANG_SIMPCHINESE} "创建开始菜单快捷方式"

!insertmacro MUI_FUNCTION_DESCRIPTION_BEGIN
    !insertmacro MUI_DESCRIPTION_TEXT ${SecCore} $(DESC_SecCore)
    !insertmacro MUI_DESCRIPTION_TEXT ${SecCP} $(DESC_SecCP)
    !insertmacro MUI_DESCRIPTION_TEXT ${SecService} $(DESC_SecService)
    !insertmacro MUI_DESCRIPTION_TEXT ${SecShortcuts} $(DESC_SecShortcuts)
!insertmacro MUI_FUNCTION_DESCRIPTION_END

; ─── 初始化检查 ─────────────────────────────────────────────
Function .onInit
    ; 检查管理员权限
    UserInfo::GetAccountType
    Pop $0
    ${If} $0 != "admin"
        MessageBox MB_ICONSTOP "此安装程序需要管理员权限运行。$\r$\n请右键选择'以管理员身份运行'。"
        Abort
    ${EndIf}

    ; 检查 64 位系统
    ${IfNot} ${RunningX64}
        MessageBox MB_ICONSTOP "WinSLA 仅支持 64 位 Windows 系统。"
        Abort
    ${EndIf}

    ; 安全警告
    MessageBox MB_YESNO|MB_ICONEXCLAMATION \
        "⚠️ 安全警告$\r$\n$\r$\n安装 Credential Provider 将修改系统登录流程。$\r$\n如果配置错误，可能导致无法正常登录。$\r$\n$\r$\n强烈建议在虚拟机中测试。$\r$\n$\r$\n确认继续安装？" \
        IDYES continue
    Abort
    continue:
FunctionEnd
