; Void Terminal — NSIS Installer
; Next > Next > Install > Finish

!include "MUI2.nsh"

; ── Metadata ────────────────────────────────────────────────────────
!define APP_NAME "Void Terminal"
!define APP_EXE "void.exe"
!define APP_PUBLISHER "190km"
!define APP_URL "https://void.sh"

; VERSION is passed from the command line: makensis -DVERSION=0.0.1
!ifndef VERSION
  !define VERSION "0.0.0"
!endif

Name "${APP_NAME}"
OutFile "void-${VERSION}-x86_64-setup.exe"
InstallDir "$LOCALAPPDATA\Programs\Void"
InstallDirRegKey HKCU "Software\Void" "InstallDir"
RequestExecutionLevel user
Unicode True

; ── UI Config ───────────────────────────────────────────────────────
!define MUI_ABORTWARNING
!define MUI_ICON "..\assets\icon.ico"
!define MUI_UNICON "..\assets\icon.ico"

; ── Pages ───────────────────────────────────────────────────────────
!insertmacro MUI_PAGE_WELCOME
!insertmacro MUI_PAGE_DIRECTORY
!insertmacro MUI_PAGE_INSTFILES
!insertmacro MUI_PAGE_FINISH

; Uninstaller pages
!insertmacro MUI_UNPAGE_CONFIRM
!insertmacro MUI_UNPAGE_INSTFILES

!insertmacro MUI_LANGUAGE "English"

; ── Install Section ─────────────────────────────────────────────────
Section "Install"
  SetOutPath "$INSTDIR"

  ; Main binary
  File "${APP_EXE}"

  ; Create uninstaller
  WriteUninstaller "$INSTDIR\uninstall.exe"

  ; Desktop shortcut
  CreateShortcut "$DESKTOP\Void.lnk" "$INSTDIR\${APP_EXE}" "" "$INSTDIR\${APP_EXE}" 0

  ; Start Menu
  CreateDirectory "$SMPROGRAMS\Void"
  CreateShortcut "$SMPROGRAMS\Void\Void.lnk" "$INSTDIR\${APP_EXE}" "" "$INSTDIR\${APP_EXE}" 0
  CreateShortcut "$SMPROGRAMS\Void\Uninstall.lnk" "$INSTDIR\uninstall.exe"

  ; Registry — install path + Add/Remove Programs
  WriteRegStr HKCU "Software\Void" "InstallDir" "$INSTDIR"
  WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\Void" \
    "DisplayName" "${APP_NAME}"
  WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\Void" \
    "UninstallString" '"$INSTDIR\uninstall.exe"'
  WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\Void" \
    "DisplayVersion" "${VERSION}"
  WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\Void" \
    "Publisher" "${APP_PUBLISHER}"
  WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\Void" \
    "URLInfoAbout" "${APP_URL}"
  WriteRegDWORD HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\Void" \
    "NoModify" 1
  WriteRegDWORD HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\Void" \
    "NoRepair" 1
SectionEnd

; ── Uninstall Section ───────────────────────────────────────────────
Section "Uninstall"
  Delete "$INSTDIR\${APP_EXE}"
  Delete "$INSTDIR\uninstall.exe"
  RMDir "$INSTDIR"

  Delete "$DESKTOP\Void.lnk"
  Delete "$SMPROGRAMS\Void\Void.lnk"
  Delete "$SMPROGRAMS\Void\Uninstall.lnk"
  RMDir "$SMPROGRAMS\Void"

  DeleteRegKey HKCU "Software\Void"
  DeleteRegKey HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\Void"
SectionEnd
