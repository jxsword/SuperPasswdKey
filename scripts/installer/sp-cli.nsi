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
  File "${SRC}\add-path.ps1"
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

; PATH 修改经 add-path.ps1（.NET RegistryKey 无 1024 长度上限、保类型、幂等、
; 防覆盖守卫）。禁用 NSIS 原生 ReadRegStr/WriteRegStr 方案——ReadRegStr 对
; 超过 NSIS_MAX_STRLEN(1024) 的注册表值返回空串，旧逻辑据此把整个用户 PATH
; 覆盖为仅安装目录（2026-10-30 事故根因，见 docs/reviews/ 事件记录）。
Section "Add install dir to user PATH (recommended)" SEC_PATH
  nsExec::ExecToLog 'powershell -NoProfile -ExecutionPolicy Bypass -File "$INSTDIR\add-path.ps1" -InstallDir "$INSTDIR"'
  Pop $0
  DetailPrint "add-path.ps1 exit code: $0"
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
