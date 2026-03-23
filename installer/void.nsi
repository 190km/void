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
OutFile "void-${VERSION}-windows-x64.exe"
InstallDir "$LOCALAPPDATA\Programs\Void"
InstallDirRegKey HKCU "Software\Void" "InstallDir"
RequestExecutionLevel user
Unicode True

; ── UI Config ───────────────────────────────────────────────────────
!define MUI_ABORTWARNING
!define MUI_ICON "..\assets\icon.ico"
!define MUI_UNICON "..\assets\icon.ico"

; ── Finish page: launch app + shortcut checkboxes ───────────────────
!define MUI_FINISHPAGE_RUN "$INSTDIR\${APP_EXE}"
!define MUI_FINISHPAGE_RUN_TEXT "Launch Void Terminal"
!define MUI_FINISHPAGE_SHOWREADME ""
!define MUI_FINISHPAGE_SHOWREADME_TEXT "Create Desktop Shortcut"
!define MUI_FINISHPAGE_SHOWREADME_FUNCTION CreateDesktopShortcut

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

  ; Main binary + icon
  File "${APP_EXE}"
  File /oname=void.ico "..\assets\icon.ico"

  ; Create uninstaller
  WriteUninstaller "$INSTDIR\uninstall.exe"

  ; Start Menu
  CreateDirectory "$SMPROGRAMS\Void"
  CreateShortcut "$SMPROGRAMS\Void\Void.lnk" "$INSTDIR\${APP_EXE}" "" "$INSTDIR\void.ico" 0
  CreateShortcut "$SMPROGRAMS\Void\Uninstall.lnk" "$INSTDIR\uninstall.exe"

  ; Registry — install path + Add/Remove Programs
  WriteRegStr HKCU "Software\Void" "InstallDir" "$INSTDIR"
  WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\Void" \
    "DisplayName" "${APP_NAME}"
  WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\Void" \
    "UninstallString" '"$INSTDIR\uninstall.exe"'
  WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\Void" \
    "DisplayIcon" '"$INSTDIR\void.ico"'
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

; ── Desktop shortcut function (called from finish page checkbox) ────
Function CreateDesktopShortcut
  CreateShortcut "$DESKTOP\Void.lnk" "$INSTDIR\${APP_EXE}" "" "$INSTDIR\void.ico" 0
FunctionEnd

; ── Uninstall Section ───────────────────────────────────────────────
Section "Uninstall"
  Delete "$INSTDIR\${APP_EXE}"
  Delete "$INSTDIR\void.ico"
  Delete "$INSTDIR\uninstall.exe"
  RMDir "$INSTDIR"

  Delete "$DESKTOP\Void.lnk"
  Delete "$SMPROGRAMS\Void\Void.lnk"
  Delete "$SMPROGRAMS\Void\Uninstall.lnk"
  RMDir "$SMPROGRAMS\Void"

  DeleteRegKey HKCU "Software\Void"
  DeleteRegKey HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\Void"
SectionEnd
