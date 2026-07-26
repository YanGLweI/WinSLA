锘? WinSLA - Windows Dual-Account Authentication System
; NSIS Installer Script v0.0.1
; Requires: NSIS 3.x

!include "MUI2.nsh"
!include "LogicLib.nsh"
!include "x64.nsh"

; 鈹€鈹€鈹€ 鍩烘湰淇℃伅 鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€
Name "WinSLA v0.0.1"
OutFile "WinSLA-v0.0.1-Setup.exe"
InstallDir "$SYSDIR\winsla"
InstallDirRegKey HKLM "Software\WinSLA" "InstallDir"
RequestExecutionLevel admin
Unicode true

; 鈹€鈹€鈹€ 鐗堟湰淇℃伅 鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€
VIProductVersion "0.0.1.0"
VIAddVersionKey "ProductName" "WinSLA"
VIAddVersionKey "FileVersion" "0.0.1"
VIAddVersionKey "FileDescription" "Windows Dual-Account Authentication System"
VIAddVersionKey "LegalCopyright" "MIT License - 2026 WinSLA Contributors"

; 鈹€鈹€鈹€ 鐣岄潰閰嶇疆 鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€
!define MUI_ICON "${NSISDIR}\Contrib\Graphics\Icons\modern-install.ico"
!define MUI_UNICON "${NSISDIR}\Contrib\Graphics\Icons\modern-uninstall.ico"
!define MUI_WELCOMEPAGE_TITLE "WinSLA 鍙岃处鍙疯璇佺郴缁?瀹夎鍚戝"
!define MUI_WELCOMEPAGE_TEXT "鏈悜瀵煎皢瀹夎 WinSLA Windows 鍙岃处鍙峰崗鍚岀櫥褰曚唬鐞嗐€?\r$\n$\r$\nWinSLA 瀹炵幇'閲戝簱鍙屼汉鍘熷垯'锛岃姹備袱涓嫭绔?AD 鍩熻处鍙峰悓鏃堕獙璇侀€氳繃鏂瑰彲鐧诲綍銆?\r$\n$\r$\n鈿狅笍 璀﹀憡锛氬畨瑁呭悗浼氬奖鍝嶇郴缁熺櫥褰曟祦绋嬶紝璇风‘淇濆湪娴嬭瘯鐜涓搷浣溿€?\r$\n$\r$\n鐐瑰嚮'涓嬩竴姝?缁х画銆?
!define MUI_FINISHPAGE_TITLE "瀹夎瀹屾垚"
!define MUI_FINISHPAGE_TEXT "WinSLA 宸叉垚鍔熷畨瑁呫€?\r$\n$\r$\n宸叉敞鍐?Credential Provider 骞跺惎鍔ㄨ璇佹湇鍔°€?\r$\n涓嬫鐧诲綍鏃跺皢鏄剧ず鍙岃处鍙疯璇佺晫闈€?\r$\n$\r$\n濡傞渶鍗歌浇锛岃閫氳繃鎺у埗闈㈡澘鎴栬繍琛屽嵏杞界▼搴忋€?

; 鈹€鈹€鈹€ 椤甸潰 鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€
!insertmacro MUI_PAGE_WELCOME
!insertmacro MUI_PAGE_LICENSE "..\LICENSE"
!insertmacro MUI_PAGE_DIRECTORY
!insertmacro MUI_PAGE_INSTFILES
!insertmacro MUI_PAGE_FINISH

!insertmacro MUI_UNPAGE_CONFIRM
!insertmacro MUI_UNPAGE_INSTFILES

; 鈹€鈹€鈹€ 璇█ 鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€
!insertmacro MUI_LANGUAGE "SimpChinese"
!insertmacro MUI_LANGUAGE "English"

; 鈹€鈹€鈹€ 甯搁噺 鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€
!define CP_CLSID "{A5A5A5A5-B6B6-C7C7-D8D8-E9E9E9E9E9E9}"
!define SERVICE_NAME "WinSLA Service"
!define PIPE_NAME "winsla-auth-pipe"

; 鈹€鈹€鈹€ 瀹夎鍖烘 鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€
Section "Core Files" SecCore
    SectionIn RO

    ; 鍒涘缓瀹夎鐩綍
    SetOutPath "$INSTDIR"

    ; 澶嶅埗鏍稿績鏂囦欢
    File "..\target\release\DualAuthCP.dll"
    File "..\target\release\winsla-service.exe"
    File "..\target\release\winsla-management.exe"

    ; 澶嶅埗鑴氭湰
    SetOutPath "$INSTDIR\scripts"
    File "..\scripts\install.ps1"
    File "..\scripts\unregister.ps1"

    ; 鍐欏叆瀹夎璺緞鍒版敞鍐岃〃
    WriteRegStr HKLM "Software\WinSLA" "InstallDir" "$INSTDIR"
    WriteRegStr HKLM "Software\WinSLA" "Version" "0.0.1"

    ; 鍒涘缓鍗歌浇绋嬪簭
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
    ; 娉ㄥ唽 CP CLSID
    WriteRegStr HKLM "SOFTWARE\Classes\CLSID\${CP_CLSID}" "" "WinSLA Dual-Auth Credential Provider"
    WriteRegStr HKLM "SOFTWARE\Classes\CLSID\${CP_CLSID}\InprocServer32" "" "$INSTDIR\DualAuthCP.dll"
    WriteRegStr HKLM "SOFTWARE\Classes\CLSID\${CP_CLSID}\InprocServer32" "ThreadingModel" "Apartment"

    ; 娉ㄥ唽鍒?Credential Providers
    WriteRegStr HKLM "SOFTWARE\Microsoft\Windows\CurrentVersion\Authentication\Credential Providers\${CP_CLSID}" "" "WinSLA Dual-Auth"
    WriteRegDWORD HKLM "SOFTWARE\Microsoft\Windows\CurrentVersion\Authentication\Credential Providers\${CP_CLSID}" "Disabled" 0

    DetailPrint "Credential Provider 宸叉敞鍐? ${CP_CLSID}"
SectionEnd

Section "Install Windows Service" SecService
    ; 浣跨敤 sc.exe 娉ㄥ唽鏈嶅姟 (鏍囧噯 Windows 鏂瑰紡)
    nsExec::ExecToLog 'sc.exe create "${SERVICE_NAME}" binPath= "$INSTDIR\winsla-service.exe --service" start= auto'
    Pop $0

    ${If} $0 == 0
        DetailPrint "WinSLA Service 宸叉敞鍐?
        ; 鍚姩鏈嶅姟
        nsExec::ExecToLog 'net start "${SERVICE_NAME}"'
        Pop $0
        ${If} $0 == 0
            DetailPrint "WinSLA Service 宸插惎鍔?
        ${Else}
            DetailPrint "鏈嶅姟宸叉敞鍐屼絾鍚姩澶辫触 (閿欒鐮? $0)锛屽彲绋嶅悗鎵嬪姩鍚姩"
        ${EndIf}
    ${Else}
        DetailPrint "鏈嶅姟娉ㄥ唽澶辫触 (閿欒鐮? $0)锛屽彲鎵嬪姩杩愯: sc.exe create ..."
    ${EndIf}
SectionEnd

Section "Start Menu Shortcuts" SecShortcuts
    CreateDirectory "$SMPROGRAMS\WinSLA"
    CreateShortcut "$SMPROGRAMS\WinSLA\WinSLA Management.lnk" "$INSTDIR\winsla-management.exe"
    CreateShortcut "$SMPROGRAMS\WinSLA\Uninstall WinSLA.lnk" "$INSTDIR\uninstall.exe"

    ; 妗岄潰蹇嵎鏂瑰紡
    CreateShortcut "$DESKTOP\WinSLA Management.lnk" "$INSTDIR\winsla-management.exe" "" "$INSTDIR\winsla-management.exe" 0
SectionEnd

; 鈹€鈹€鈹€ 鍗歌浇鍖烘 鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€
Section "Uninstall"
    ; 鍋滄骞跺垹闄ゆ湇鍔?    nsExec::ExecToLog 'net stop "${SERVICE_NAME}"'
    nsExec::ExecToLog 'sc delete "${SERVICE_NAME}"'

    ; 娉ㄩ攢 Credential Provider
    DeleteRegKey HKLM "SOFTWARE\Microsoft\Windows\CurrentVersion\Authentication\Credential Providers\${CP_CLSID}"
    DeleteRegKey HKLM "SOFTWARE\Classes\CLSID\${CP_CLSID}"

    ; 鍒犻櫎娉ㄥ唽琛?    DeleteRegKey HKLM "Software\WinSLA"
    DeleteRegKey HKLM "Software\Microsoft\Windows\CurrentVersion\Uninstall\WinSLA"

    ; 鍒犻櫎寮€濮嬭彍鍗曞拰妗岄潰蹇嵎鏂瑰紡
    Delete "$DESKTOP\WinSLA Management.lnk"
    Delete "$SMPROGRAMS\WinSLA\WinSLA Management.lnk"
    Delete "$SMPROGRAMS\WinSLA\Uninstall WinSLA.lnk"
    RMDir "$SMPROGRAMS\WinSLA"

    ; 鍒犻櫎鏂囦欢
    Delete "$INSTDIR\DualAuthCP.dll"
    Delete "$INSTDIR\winsla-service.exe"
    Delete "$INSTDIR\winsla-management.exe"
    Delete "$INSTDIR\scripts\install.ps1"
    Delete "$INSTDIR\scripts\unregister.ps1"
    Delete "$INSTDIR\uninstall.exe"
    RMDir "$INSTDIR\scripts"
    RMDir "$INSTDIR"

    DetailPrint "WinSLA 宸插畬鍏ㄥ嵏杞?
SectionEnd

; 鈹€鈹€鈹€ 鎻忚堪 鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€
LangString DESC_SecCore ${LANG_SIMPCHINESE} "鏍稿績鏂囦欢 (DLL銆佹湇鍔°€佺鐞嗙銆佽剼鏈?"
LangString DESC_SecCP ${LANG_SIMPCHINESE} "娉ㄥ唽 Credential Provider 鍒扮郴缁?(褰卞搷鐧诲綍鐣岄潰)"
LangString DESC_SecService ${LANG_SIMPCHINESE} "瀹夎骞跺惎鍔?WinSLA Windows 璁よ瘉鏈嶅姟"
LangString DESC_SecShortcuts ${LANG_SIMPCHINESE} "鍒涘缓寮€濮嬭彍鍗曞揩鎹锋柟寮?

!insertmacro MUI_FUNCTION_DESCRIPTION_BEGIN
    !insertmacro MUI_DESCRIPTION_TEXT ${SecCore} $(DESC_SecCore)
    !insertmacro MUI_DESCRIPTION_TEXT ${SecCP} $(DESC_SecCP)
    !insertmacro MUI_DESCRIPTION_TEXT ${SecService} $(DESC_SecService)
    !insertmacro MUI_DESCRIPTION_TEXT ${SecShortcuts} $(DESC_SecShortcuts)
!insertmacro MUI_FUNCTION_DESCRIPTION_END

; 鈹€鈹€鈹€ 鍒濆鍖栨鏌?鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€
Function .onInit
    ; 妫€鏌ョ鐞嗗憳鏉冮檺
    UserInfo::GetAccountType
    Pop $0
    ${If} $0 != "admin"
        MessageBox MB_ICONSTOP "姝ゅ畨瑁呯▼搴忛渶瑕佺鐞嗗憳鏉冮檺杩愯銆?\r$\n璇峰彸閿€夋嫨'浠ョ鐞嗗憳韬唤杩愯'銆?
        Abort
    ${EndIf}

    ; 妫€鏌?64 浣嶇郴缁?    ${IfNot} ${RunningX64}
        MessageBox MB_ICONSTOP "WinSLA 浠呮敮鎸?64 浣?Windows 绯荤粺銆?
        Abort
    ${EndIf}

    ; 瀹夊叏璀﹀憡
    MessageBox MB_YESNO|MB_ICONEXCLAMATION \
        "鈿狅笍 瀹夊叏璀﹀憡$\r$\n$\r$\n瀹夎 Credential Provider 灏嗕慨鏀圭郴缁熺櫥褰曟祦绋嬨€?\r$\n濡傛灉閰嶇疆閿欒锛屽彲鑳藉鑷存棤娉曟甯哥櫥褰曘€?\r$\n$\r$\n寮虹儓寤鸿鍦ㄨ櫄鎷熸満涓祴璇曘€?\r$\n$\r$\n纭缁х画瀹夎锛? \
        IDYES continue
    Abort
    continue:
FunctionEnd
