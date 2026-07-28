; WinSLA - Windows Dual-Account Authentication System
; NSIS Installer Script v2.0.9
; Fixed: Audit log table field names (account_sid/approver_sid) and UI terminology update

!include "MUI2.nsh"
!include "LogicLib.nsh"
!include "x64.nsh"

; ─── 基本信息 ───────────────────────────────────────────────
Name "WinSLA v2.0.9"
OutFile "WinSLA-v2.0.9-Setup.exe"
InstallDir "$PROGRAMFILES64\WinSLA"
InstallDirRegKey HKLM "Software\WinSLA" "InstallDir"
RequestExecutionLevel admin
Unicode true

; ─── 版本信息 ───────────────────────────────────────────────
VIProductVersion "2.0.9.0"
VIAddVersionKey "ProductName" "WinSLA"
VIAddVersionKey "FileVersion" "2.0.9"
VIAddVersionKey "FileDescription" "WinSLA - Windows Dual-Account Authentication System"
VIAddVersionKey "LegalCopyright" "MIT License - 2026 ylw"

; ─── 界面配置 ───────────────────────────────────────────────
!define MUI_ICON "..\assets\winsla.ico"
!define MUI_UNICON "..\assets\winsla.ico"
Icon "..\assets\winsla.ico"
UninstallIcon "..\assets\winsla.ico"
!define MUI_WELCOMEPAGE_TITLE "WinSLA 双账号认证系统 安装向导"
!define MUI_WELCOMEPAGE_TEXT "本向导将安装 WinSLA Windows 双账号协同登录代理。$\r$\n$\r$\nWinSLA 实现'金库双人原则'，要求两个独立 AD 域账号同时验证通过方可登录。$\r$\n$\r$\n✅ 新版本特性：严格配对规则验证！主账号与审批人必须按顺序匹配，B-A 顺序将被拒绝。$\r$\n$\r$\n✅ 已修复数据库字段名不一致问题（v2.0.5 bug）!$\r$\n⚠️ 警告：安装后会影响系统登录流程，请确保在测试环境中操作。$\r$\n$\r$\n点击'下一步'继续。"
!define MUI_FINISHPAGE_TITLE "安装完成"
!define MUI_FINISHPAGE_TEXT "WinSLA 已成功安装。$\r$\n$\r$\n✅ 已自动注册 Credential Provider 到系统注册表!$\r$\n✅ 已启动认证服务 !$\r$\n✅ 已启用严格配对规则验证功能（主账号 + 审批人必须顺序匹配）！$\r$\n$\r$\n下次登录时将显示双账号认证界面。$\r$\n$\r$\n如需卸载，请通过控制面板或运行卸载程序。"

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
    WriteRegStr HKLM "Software\WinSLA" "Version" "2.0.9"

    ; 创建卸载程序
    WriteUninstaller "$INSTDIR\uninstall.exe"
    WriteRegStr HKLM "Software\Microsoft\Windows\CurrentVersion\Uninstall\WinSLA" \
        "DisplayName" "WinSLA - Dual-Account Authentication"
    WriteRegStr HKLM "Software\Microsoft\Windows\CurrentVersion\Uninstall\WinSLA" \
        "UninstallString" "$INSTDIR\uninstall.exe"
    WriteRegStr HKLM "Software\Microsoft\Windows\CurrentVersion\Uninstall\WinSLA" \\
        "DisplayVersion" "2.0.9"
    WriteRegStr HKLM "Software\Microsoft\Windows\CurrentVersion\Uninstall\WinSLA" \
        "Publisher" "ylw"
    WriteRegDWORD HKLM "Software\Microsoft\Windows\CurrentVersion\Uninstall\WinSLA" \
        "NoModify" 1
    WriteRegDWORD HKLM "Software\Microsoft\Windows\CurrentVersion\Uninstall\WinSLA" \
        "NoRepair" 1
SectionEnd

Section "Register Credential Provider" SecCP
    DetailPrint "Creating Credential Provider registry entries..."
    
    ; Step 1: Create CLSID base key for DualAuthCredentialProvider class
    WriteRegStr HKLM "SOFTWARE\Classes\CLSID\${CP_CLSID}" "" "WinSLA Dual-Auth Credential Provider"
    WriteRegStr HKLM "SOFTWARE\Classes\CLSID\${CP_CLSID}" "Version" "1.0.24"
    DetailPrint "Creating CLSID key: ${CP_CLSID}..."
    
    ; Create InprocServer32 subkey (standard COM registration)
    WriteRegStr HKLM "SOFTWARE\Classes\CLSID\${CP_CLSID}\InprocServer32" "" "$INSTDIR\DualAuthCP.dll"
    WriteRegStr HKLM "SOFTWARE\Classes\CLSID\${CP_CLSID}\InprocServer32" "ThreadingModel" "Apartment"
    DetailPrint "  ✓ Created InprocServer32 subkey"
    DetailPrint "  ✓ DLL Path: $INSTDIR\DualAuthCP.dll"
    DetailPrint "  ✓ ThreadingModel: Apartment (required by LogonUI)"
    
    ; Step 2: Register this CP with Windows Authentication system (THE KEY STEP!)
    ; This is where WinSLA tells LogonUI about the new credential provider
    DetailPrint "Registering Credential Provider with Windows Authentication..."
    
    WriteRegStr HKLM "SOFTWARE\Microsoft\Windows\CurrentVersion\Authentication\Credential Providers\${CP_CLSID}" "" "WinSLA Dual-Account Auth"
    WriteRegStr HKLM "SOFTWARE\Microsoft\Windows\CurrentVersion\Authentication\Credential Providers\${CP_CLSID}" "DllPath" "$INSTDIR\DualAuthCP.dll"
    WriteRegStr HKLM "SOFTWARE\Microsoft\Windows\CurrentVersion\Authentication\Credential Providers\${CP_CLSID}" "Description" "WinSLA Dual-Account Authentication System v2.0.4"
    
    ; Disabled=0 means ENABLED (this is how Windows works - 0 = active, 1 = disabled)
    WriteRegDWORD HKLM "SOFTWARE\Microsoft\Windows\CurrentVersion\Authentication\Credential Providers\${CP_CLSID}" "Disabled" 0
    
    DetailPrint "  ✓ Added to Credential Providers list"
    DetailPrint "  ✓ Set Disabled=0 (ENABLED/active)"
    DetailPrint "✓ All Credential Provider registrations completed successfully!"
SectionEnd

Section "Install Windows Service" SecService
    DetailPrint "Installing WinSLA Windows Service..."
    
    ; Stop any existing service first (from previous installation)
    nsExec::ExecToLog 'sc.exe stop "${SERVICE_NAME}"'
    Pop $0
    ${If} $0 != 1060 ; 错误码 1060 = 服务未安装是正常的
        DetailPrint "Stopped existing service (or no service was running)"
    ${EndIf}
    
    ; Remove service if it exists (sometimes registry entry remains)
    nsExec::ExecToLog 'sc.exe delete "${SERVICE_NAME}"'
    Pop $0
    ${If} $0 != 1060 ; 正常或不存在都是 ok 的
        DetailPrint "Removed old service entry"
    ${ElseIf} $0 == 1060
        DetailPrint "Service does not exist yet (normal on fresh install)"
    ${EndIf}
    
    ; Create new service with sc.exe (standard Windows method)
    nsExec::ExecToLog 'sc.exe create "${SERVICE_NAME}" binPath= "$INSTDIR\winsla-service.exe --service" start= auto' 2>&1
    Pop $0
    StrCpy $0 $0 1 ; 只取第一个字符（成功/失败）
    
    ${If} $0 == "0"
        DetailPrint "✓ Service created successfully"
        
        ; Wait a moment for registry to sync
        Sleep 1000
        
        ; Start the service
        nsExec::ExecToLog 'net start "${SERVICE_NAME}"' 2>&1
        Pop $0
        
        ${If} $0 == "0"
            DetailPrint "✓ Service started and is running"
        ${ElseIf} $0 == "1060" ; 错误码 1060 = 服务已存在但未运行
            DetailPrint "⚠ Service already registered, starting..."
            Sleep 500
            nsExec::ExecToLog 'net start "${SERVICE_NAME}"' 2>&1
            Pop $0
            ${If} $0 == "0"
                DetailPrint "✓ Service started"
            ${Else}
                DetailPrint "✗ Failed to start service (error: $0), but registration completed"
            ${EndIf}
        ${Else}
            DetailPrint "✗ Service start failed (error code: $0), but registration is complete"
        ${EndIf}
    ${Else}
        DetailPrint "✗ Service creation failed (error: $0)"
        DetailPrint "You may need to manually run: sc.exe create ..."
        MessageBox MB_ICONSTOP "Service installation failed!\n\nError code: $0\n\nPlease check error log above." IDOK
    ${EndIf}
    
    ; Verify service exists
    DetailPrint "Verifying service installation..."
    nsExec::ExecToLog 'sc.exe query "${SERVICE_NAME}" type= SERVICE'
    Pop $0
    ${If} $0 != "0"
        DetailPrint "✗ Query service failed - checking service status in Control Panel"
    ${Else}
        DetailPrint "✓ Service verification passed"
    ${EndIf}
SectionEnd

Section "Start Menu Shortcuts" SecShortcuts
    CreateDirectory "$SMPROGRAMS\WinSLA"
    CreateShortcut "$SMPROGRAMS\WinSLA\WinSLA Management.lnk" "$INSTDIR\winsla-management.exe" "" "$INSTDIR\winsla.ico" 0
    CreateShortcut "$SMPROGRAMS\WinSLA\Uninstall WinSLA.lnk" "$INSTDIR\uninstall.exe"

    ; 桌面快捷方式 (先删除旧的，再创建新的)
    Delete "$DESKTOP\WinSLA Management.lnk"
    CreateShortcut "$DESKTOP\WinSLA Management.lnk" "$INSTDIR\winsla-management.exe" "" "$INSTDIR\winsla.ico" 0

    ; 刷新图标缓存
    nsExec::ExecToLog 'ie4uinit.exe -show'
    Pop $0
    
    ; 创建安装完成标记（用于后续刷新）
    WriteRegStr HKLM "Software\WinSLA" "InstallComplete" "1"
    DetailPrint "安装配置已完成，请重启计算机应用更改"
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
    Delete "$INSTDIR\winsla.ico"
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
    ; 关键修复: NSIS 安装器是 32 位进程，默认写 HKLM\SOFTWARE 会被 WOW64
    ; 重定向到 WOW6432Node。必须切换到 64 位注册表视图，
    ; 否则 64 位 LogonUI 读不到 Credential Provider 注册。
    SetRegView 64

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

Function un.onInit
    ; 卸载时同样使用 64 位注册表视图，确保能删除 64 位视图下的键
    SetRegView 64
FunctionEnd
