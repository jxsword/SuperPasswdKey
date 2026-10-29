; SuperPasswdKey CLI 单安装包（SP-C-104；用户级安装，免管理员）
; 编译（仓库根）：makensis /DVER=0.3.0 /DSRC=dist scripts\installer\sp-cli.nsi
;   SRC 目录须含：sp.exe / sp-nmhost.exe / README.md / docs\manual\sp_cli_manual.md /
;                 native-host\{register.ps1,unregister.ps1,README.md}
; 功能：核心文件（必装）+ “添加到用户 PATH”（默认勾选，幂等）+ 控制面板卸载器
; 纪律：仅写 HKCU；PATH 保留用户其余条目；无网络；卸载不动用户库/配置

Unicode true
ManifestDPIAware true
RequestExecutionLevel user
SetCompressor /SOLID lzma

!ifndef VER
  !define VER "0.0.0-dev"
!endif
!ifndef SRC
  !define SRC "dist"
!endif

!define APPNAME "SuperPasswdKey CLI"
!define UNINSTKEY "SuperPasswdKeyCLI"

!include "MUI2.nsh"
!include "StrFunc.nsh"
${Using:StrFunc} StrLoc

Name "${APPNAME} ${VER}"
OutFile "sp-cli-${VER}-x64-setup.exe"
InstallDir "$LOCALAPPDATA\SuperPasswdKey\bin"
InstallDirRegKey HKCU "Software\SuperPasswdKeyCLI" "InstallDir"
ShowInstDetails show
ShowUninstDetails show

!define MUI_ABORTWARNING
!insertmacro MUI_PAGE_COMPONENTS
!insertmacro MUI_PAGE_DIRECTORY
!insertmacro MUI_PAGE_INSTFILES
!insertmacro MUI_UNPAGE_CONFIRM
!insertmacro MUI_UNPAGE_INSTFILES
!insertmacro MUI_LANGUAGE "English"

Section "!Core files" SEC_CORE
  SectionIn RO
  SetOutPath "$INSTDIR"
  File "${SRC}\sp.exe"
  File "${SRC}\sp-nmhost.exe"
  File "${SRC}\README.md"
  SetOutPath "$INSTDIR\docs"
  File "${SRC}\docs\manual\sp_cli_manual.md"
  SetOutPath "$INSTDIR\native-host"
  File "${SRC}\native-host\register.ps1"
  File "${SRC}\native-host\unregister.ps1"
  File "${SRC}\native-host\README.md"
  WriteUninstaller "$INSTDIR\uninstall.exe"
  WriteRegStr HKCU "Software\SuperPasswdKeyCLI" "InstallDir" "$INSTDIR"
  WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\${UNINSTKEY}" "DisplayName" "${APPNAME}"
  WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\${UNINSTKEY}" "DisplayVersion" "${VER}"
  WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\${UNINSTKEY}" "UninstallString" "$INSTDIR\uninstall.exe"
  WriteRegDWORD HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\${UNINSTKEY}" "NoModify" 1
  WriteRegDWORD HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\${UNINSTKEY}" "NoRepair" 1
  WriteRegDWORD HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\${UNINSTKEY}" "EstimatedSize" 12000
SectionEnd

Section "Add install dir to user PATH (recommended)" SEC_PATH
  ReadRegStr $R0 HKCU "Environment" "Path"
  ${StrLoc} $R1 "$R0" "$INSTDIR" 0
  ${If} $R1 != ""
    DetailPrint "PATH already contains $INSTDIR (skip)"
    Goto path_done
  ${EndIf}
  ${If} $R0 == ""
    StrCpy $R0 "$INSTDIR"
  ${Else}
    StrCpy $R0 "$R0;$INSTDIR"
  ${EndIf}
  ; 保留可展开条目语义：原 PATH 含 %VAR% 时写 REG_EXPAND_SZ
  ${StrLoc} $R2 "$R0" "%" 0
  ${If} $R2 != ""
    WriteRegExpandStr HKCU "Environment" "Path" "$R0"
  ${Else}
    WriteRegStr HKCU "Environment" "Path" "$R0"
  ${EndIf}
  SendMessage ${HWND_BROADCAST} ${WM_SETTINGCHANGE} 0 "STR:Environment" /TIMEOUT=5000
  DetailPrint "User PATH updated: $INSTDIR (new terminals only)"
path_done:
SectionEnd

Section "Uninstall"
  Delete "$INSTDIR\sp.exe"
  Delete "$INSTDIR\sp-nmhost.exe"
  Delete "$INSTDIR\README.md"
  Delete "$INSTDIR\docs\sp_cli_manual.md"
  Delete "$INSTDIR\native-host\register.ps1"
  Delete "$INSTDIR\native-host\unregister.ps1"
  Delete "$INSTDIR\native-host\README.md"
  Delete "$INSTDIR\uninstall.exe"
  RMDir "$INSTDIR\docs"
  RMDir "$INSTDIR\native-host"
  RMDir "$INSTDIR"
  DeleteRegKey HKCU "Software\SuperPasswdKeyCLI"
  DeleteRegKey HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\${UNINSTKEY}"
  ; PATH 条目保留（避免误删用户其他 PATH 项）；需要时手动编辑“编辑账户的环境变量”
  DetailPrint "Note: the user PATH entry for $INSTDIR was kept (remove manually if unwanted)."
SectionEnd
