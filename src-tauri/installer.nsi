; Wexo Optimizer - NSIS Installer Script
; Generated for Tauri v2

!include "MUI2.nsh"
!include "FileFunc.nsh"
!include "x64.nsh"
!include "LogicLib.nsh"

; --- Metadata ---
!define APP_NAME "Wexo Optimizer"
!define APP_VERSION "0.1.0"
!define APP_PUBLISHER "Wexo"
!define APP_WEBSITE "https://wexo.app"
!define APP_SUPPORT "https://github.com/wexo/optimizer/issues"
!define APP_EXE "wexo-app.exe"
!define MAIN_ICON "icons/icon.ico"
!define UNINSTALLER_NAME "uninstall.exe"

; --- General Settings ---
Name "${APP_NAME} ${APP_VERSION}"
OutFile "WexoOptimizer_${APP_VERSION}_Setup_x64.exe"
InstallDir "$PROGRAMFILES64\Wexo Optimizer"
InstallDirRegKey HKLM "Software\Wexo Optimizer" ""
RequestExecutionLevel admin
ShowInstDetails show
ShowUninstDetails show

; --- Modern UI ---
!define MUI_ICON "${MAIN_ICON}"
!define MUI_UNICON "${MAIN_ICON}"
!define MUI_ABORTWARNING
!define MUI_FINISHPAGE_RUN "$INSTDIR\${APP_EXE}"
!define MUI_FINISHPAGE_RUN_NOTCHECKED
!define MUI_WELCOMEPAGE_TITLE "Bienvenido al instalador de ${APP_NAME}"
!define MUI_WELCOMEPAGE_TITLE_3LINES
!define MUI_FINISHPAGE_TITLE "Instalación completada"
!define MUI_FINISHPAGE_TITLE_3LINES

; --- Pages ---
!insertmacro MUI_PAGE_WELCOME
!insertmacro MUI_PAGE_LICENSE "LICENSE.txt"
!insertmacro MUI_PAGE_DIRECTORY
!insertmacro MUI_PAGE_INSTFILES
!insertmacro MUI_PAGE_FINISH

!insertmacro MUI_UNPAGE_WELCOME
!insertmacro MUI_UNPAGE_CONFIRM
!insertmacro MUI_UNPAGE_INSTFILES
!insertmacro MUI_UNPAGE_FINISH

; --- Languages ---
!insertmacro MUI_LANGUAGE "Spanish"
!insertmacro MUI_LANGUAGE "English"

; --- Reserve files ---
ReserveFile "${MAIN_ICON}"
ReserveFile "LICENSE.txt"

; --- Installation Sections ---
Section "MainSection" SEC_MAIN
  SectionIn RO

  ; Set output path
  SetOutPath "$INSTDIR"

  ; Copy all files from Tauri bundle
  File /r "bundle\*"

  ; Create uninstaller
  WriteUninstaller "$INSTDIR\${UNINSTALLER_NAME}"

  ; Registry entries for uninstaller
  WriteRegStr HKLM "Software\Microsoft\Windows\CurrentVersion\Uninstall\${APP_NAME}" \
    "DisplayName" "${APP_NAME}"
  WriteRegStr HKLM "Software\Microsoft\Windows\CurrentVersion\Uninstall\${APP_NAME}" \
    "UninstallString" "$INSTDIR\${UNINSTALLER_NAME}"
  WriteRegStr HKLM "Software\Microsoft\Windows\CurrentVersion\Uninstall\${APP_NAME}" \
    "QuietUninstallString" "$INSTDIR\${UNINSTALLER_NAME} /S"
  WriteRegStr HKLM "Software\Microsoft\Windows\CurrentVersion\Uninstall\${APP_NAME}" \
    "DisplayVersion" "${APP_VERSION}"
  WriteRegStr HKLM "Software\Microsoft\Windows\CurrentVersion\Uninstall\${APP_NAME}" \
    "Publisher" "${APP_PUBLISHER}"
  WriteRegStr HKLM "Software\Microsoft\Windows\CurrentVersion\Uninstall\${APP_NAME}" \
    "URLInfoAbout" "${APP_WEBSITE}"
  WriteRegStr HKLM "Software\Microsoft\Windows\CurrentVersion\Uninstall\${APP_NAME}" \
    "URLUpdateInfo" "${APP_WEBSITE}"
  WriteRegStr HKLM "Software\Microsoft\Windows\CurrentVersion\Uninstall\${APP_NAME}" \
    "HelpLink" "${APP_SUPPORT}"
  WriteRegStr HKLM "Software\Microsoft\Windows\CurrentVersion\Uninstall\${APP_NAME}" \
    "InstallLocation" "$INSTDIR"
  WriteRegStr HKLM "Software\Microsoft\Windows\CurrentVersion\Uninstall\${APP_NAME}" \
    "DisplayIcon" "$INSTDIR\${APP_EXE}"
  WriteRegDWORD HKLM "Software\Microsoft\Windows\CurrentVersion\Uninstall\${APP_NAME}" \
    "NoModify" 1
  WriteRegDWORD HKLM "Software\Microsoft\Windows\CurrentVersion\Uninstall\${APP_NAME}" \
    "NoRepair" 1
  WriteRegDWORD HKLM "Software\Microsoft\Windows\CurrentVersion\Uninstall\${APP_NAME}" \
    "EstimatedSize" 0
  WriteRegStr HKLM "Software\Microsoft\Windows\CurrentVersion\Uninstall\${APP_NAME}" \
    "Language" "1033"

  ; App-specific registry
  WriteRegStr HKLM "Software\Wexo Optimizer" "" "$INSTDIR"
  WriteRegStr HKLM "Software\Wexo Optimizer" "Version" "${APP_VERSION}"
  WriteRegStr HKLM "Software\Wexo Optimizer" "InstallPath" "$INSTDIR"

  ; Start Menu shortcuts
  CreateDirectory "$SMPROGRAMS\Wexo Optimizer"
  CreateShortcut "$SMPROGRAMS\Wexo Optimizer\${APP_NAME}.lnk" "$INSTDIR\${APP_EXE}" "" "$INSTDIR\${MAIN_ICON}" 0
  CreateShortcut "$SMPROGRAMS\Wexo Optimizer\Desinstalar ${APP_NAME}.lnk" "$INSTDIR\${UNINSTALLER_NAME}" "" "$INSTDIR\${MAIN_ICON}" 0

  ; Desktop shortcut (optional - user can choose)
  CreateShortcut "$DESKTOP\${APP_NAME}.lnk" "$INSTDIR\${APP_EXE}" "" "$INSTDIR\${MAIN_ICON}" 0

  ; Register as capable of handling wexo:// protocol (optional)
  WriteRegStr HKLM "Software\Classes\wexo" "" "URL:Wexo Optimizer Protocol"
  WriteRegStr HKLM "Software\Classes\wexo" "URL Protocol" ""
  WriteRegStr HKLM "Software\Classes\wexo\shell\open\command" "" "$INSTDIR\${APP_EXE} %1"
  WriteRegStr HKLM "Software\Classes\wexo\DefaultIcon" "" "$INSTDIR\${APP_EXE},0"

  ; Firewall rule for the app (if it needs network)
  ; ExecWait '"$SYSDIR\netsh.exe" advfirewall firewall add rule name="${APP_NAME}" dir=in action=allow program="$INSTDIR\${APP_EXE}" enable=yes'

SectionEnd

; --- Optional: Auto-updater section ---
Section "AutoUpdater" SEC_UPDATER
  SectionIn RO

  ; If you have a separate updater executable, copy it here
  ; File "updater.exe"
  ; WriteRegStr HKLM "Software\Wexo Optimizer" "UpdaterPath" "$INSTDIR\updater.exe"

SectionEnd

; --- Uninstaller ---
Section "Uninstall"
  ; Remove Start Menu folder
  RMDir /r "$SMPROGRAMS\Wexo Optimizer"

  ; Remove desktop shortcut
  Delete "$DESKTOP\${APP_NAME}.lnk"

  ; Remove protocol registration
  DeleteRegKey HKLM "Software\Classes\wexo"

  ; Remove firewall rule
  ; ExecWait '"$SYSDIR\netsh.exe" advfirewall firewall delete rule name="${APP_NAME}"'

  ; Remove app registry
  DeleteRegKey HKLM "Software\Wexo Optimizer"
  DeleteRegKey HKLM "Software\Microsoft\Windows\CurrentVersion\Uninstall\${APP_NAME}"

  ; Remove installed files
  RMDir /r "$INSTDIR"

SectionEnd

; --- Functions ---
Function .onInit
  ; Check for previous installation
  ReadRegStr $0 HKLM "Software\Microsoft\Windows\CurrentVersion\Uninstall\${APP_NAME}" "UninstallString"
  StrCmp $0 "" 0 ShowUninstallConfirm
  Goto Done

  ShowUninstallConfirm:
    MessageBox MB_YESNO|MB_ICONQUESTION \
      "Se ha detectado una instalación previa de ${APP_NAME}.\n$\n¿Desea desinstalarla antes de continuar?" \
      IDYES UninstallPrevious
    Abort

  UninstallPrevious:
    ClearErrors
    ExecWait '"$0" /S'
    IfErrors 0 Done
    MessageBox MB_OK|MB_ICONSTOP "Error al desinstalar la versión anterior. Cierre la aplicación e intente de nuevo."
    Abort

  Done:
FunctionEnd

Function .onInstSuccess
  ; Hide the "Run" checkbox on finish page if app requires admin
  ; (Wexo Optimizer needs admin for system optimizations)
FunctionEnd

Function un.onUninstSuccess
  ; Hide "Run" checkbox on uninstaller finish
FunctionEnd

; --- Custom Modern UI Text (Spanish) ---
!ifdef MUI_LANGUAGE_Spanish
  !define MUI_TEXT_WELCOME_INFO_TITLE "Bienvenido al instalador de ${APP_NAME}"
  !define MUI_TEXT_WELCOME_INFO_TEXT "Este asistente le guiará a través de la instalación de ${APP_NAME} ${APP_VERSION}.\n$\nSe recomienda cerrar todas las demás aplicaciones antes de continuar."
  !define MUI_TEXT_LICENSE_TITLE "Acuerdo de licencia"
  !define MUI_TEXT_LICENSE_SUBTITLE "Por favor, lea el siguiente acuerdo de licencia. Debe aceptar los términos antes de continuar."
  !define MUI_TEXT_DIRECTORY_TITLE "Seleccionar carpeta de destino"
  !define MUI_TEXT_DIRECTORY_SUBTITLE "Seleccione la carpeta donde desea instalar ${APP_NAME}."
  !define MUI_TEXT_INSTALLING_TITLE "Instalando ${APP_NAME}"
  !define MUI_TEXT_INSTALLING_SUBTITLE "Por favor, espere mientras se instalan los archivos."
  !define MUI_TEXT_FINISH_TITLE "Instalación completada"
  !define MUI_TEXT_FINISH_SUBTITLE "La instalación se ha completado correctamente."
  !define MUI_TEXT_FINISH_RUN "Ejecutar ${APP_NAME}"
  !define MUI_TEXT_ABORT_TITLE "Instalación cancelada"
  !define MUI_TEXT_ABORT_SUBTITLE "La instalación no se completó."
  !define MUI_BUTTONTEXT_INSTALL "Instalar"
  !define MUI_BUTTONTEXT_UNINSTALL "Desinstalar"

  ; Uninstaller
  !define MUI_UNTEXT_WELCOME_INFO_TITLE "Desinstalador de ${APP_NAME}"
  !define MUI_UNTEXT_WELCOME_INFO_TEXT "Este asistente le guiará a través de la desinstalación de ${APP_NAME}.\n$\nAntes de continuar, asegúrese de que ${APP_NAME} no se esté ejecutando."
  !define MUI_UNTEXT_CONFIRM_TITLE "Confirmar desinstalación"
  !define MUI_UNTEXT_CONFIRM_SUBTITLE "¿Está seguro de que desea desinstalar ${APP_NAME}?"
  !define MUI_UNTEXT_UNINSTALLING_TITLE "Desinstalando ${APP_NAME}"
  !define MUI_UNTEXT_UNINSTALLING_SUBTITLE "Por favor, espere mientras se eliminan los archivos."
  !define MUI_UNTEXT_FINISH_TITLE "Desinstalación completada"
  !define MUI_UNTEXT_FINISH_SUBTITLE "${APP_NAME} se ha desinstalado correctamente."
!endif

; --- Custom Modern UI Text (English) ---
!ifdef MUI_LANGUAGE_English
  !define MUI_TEXT_WELCOME_INFO_TITLE "Welcome to the ${APP_NAME} Setup Wizard"
  !define MUI_TEXT_WELCOME_INFO_TEXT "This wizard will guide you through the installation of ${APP_NAME} ${APP_VERSION}.\n$\nIt is recommended that you close all other applications before continuing."
  !define MUI_TEXT_LICENSE_TITLE "License Agreement"
  !define MUI_TEXT_LICENSE_SUBTITLE "Please read the following license agreement. You must accept the terms before continuing."
  !define MUI_TEXT_DIRECTORY_TITLE "Choose Install Location"
  !define MUI_TEXT_DIRECTORY_SUBTITLE "Select the folder where you want to install ${APP_NAME}."
  !define MUI_TEXT_INSTALLING_TITLE "Installing ${APP_NAME}"
  !define MUI_TEXT_INSTALLING_SUBTITLE "Please wait while the files are being installed."
  !define MUI_TEXT_FINISH_TITLE "Installation Complete"
  !define MUI_TEXT_FINISH_SUBTITLE "The installation has completed successfully."
  !define MUI_TEXT_FINISH_RUN "Launch ${APP_NAME}"
  !define MUI_TEXT_ABORT_TITLE "Installation Cancelled"
  !define MUI_TEXT_ABORT_SUBTITLE "The installation was not completed."
  !define MUI_BUTTONTEXT_INSTALL "Install"
  !define MUI_BUTTONTEXT_UNINSTALL "Uninstall"

  ; Uninstaller
  !define MUI_UNTEXT_WELCOME_INFO_TITLE "${APP_NAME} Uninstaller"
  !define MUI_UNTEXT_WELCOME_INFO_TEXT "This wizard will guide you through the uninstallation of ${APP_NAME}.\n$\nBefore continuing, ensure that ${APP_NAME} is not running."
  !define MUI_UNTEXT_CONFIRM_TITLE "Confirm Uninstallation"
  !define MUI_UNTEXT_CONFIRM_SUBTITLE "Are you sure you want to uninstall ${APP_NAME}?"
  !define MUI_UNTEXT_UNINSTALLING_TITLE "Uninstalling ${APP_NAME}"
  !define MUI_UNTEXT_UNINSTALLING_SUBTITLE "Please wait while files are being removed."
  !define MUI_UNTEXT_FINISH_TITLE "Uninstallation Complete"
  !define MUI_UNTEXT_FINISH_SUBTITLE "${APP_NAME} has been uninstalled successfully."
!endif