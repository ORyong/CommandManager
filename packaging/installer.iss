#ifndef AppVersion
  #define AppVersion "0.3.1"
#endif
#ifndef BuildRoot
  #define BuildRoot "..\target\release"
#endif

[Setup]
#ifdef AcceptanceFixture
AppId={{B45E97F8-8CC7-4CE9-A3A0-999104310B87}
#else
AppId={{A455CABA-A492-4430-A1D9-702B85F82455}
#endif
AppName=命令管理器
AppVersion={#AppVersion}
AppPublisher=Command Manager contributors
DefaultDirName={localappdata}\Programs\CommandManager
DefaultGroupName=命令管理器
DisableProgramGroupPage=yes
DisableDirPage=no
PrivilegesRequired=lowest
ArchitecturesAllowed=x64compatible
ArchitecturesInstallIn64BitMode=x64compatible
MinVersion=10.0.17763
OutputDir=..\dist
#ifdef AcceptanceFixture
OutputBaseFilename=CommandManager-acceptance-setup
#else
OutputBaseFilename=CommandManager-{#AppVersion}-windows-x64-setup
#endif
Compression=lzma2
SolidCompression=yes
WizardStyle=modern
LicenseFile=..\LICENSE
UninstallDisplayIcon={app}\command-manager.exe
SetupIconFile=..\assets\app.ico
CloseApplications=yes
RestartApplications=no
AppMutex=Local\StandaloneCommandManagerMutex-v1
SetupLogging=yes

[Languages]
Name: "chinesesimplified"; MessagesFile: "ChineseSimplified.isl"
Name: "english"; MessagesFile: "compiler:Default.isl"

[Tasks]
Name: "desktopicon"; Description: "{cm:CreateDesktopIcon}"; Flags: unchecked

[Files]
Source: "{#BuildRoot}\command-manager.exe"; DestDir: "{app}"; Flags: ignoreversion
Source: "..\LICENSE"; DestDir: "{app}"; Flags: ignoreversion
Source: "..\README.md"; DestDir: "{app}"; Flags: ignoreversion
Source: "..\dist\THIRD-PARTY-NOTICES.txt"; DestDir: "{app}"; Flags: ignoreversion

[Dirs]
Name: "{app}\data"; Flags: uninsneveruninstall

[Icons]
Name: "{group}\命令管理器"; Filename: "{app}\command-manager.exe"
Name: "{autodesktop}\命令管理器"; Filename: "{app}\command-manager.exe"; Tasks: desktopicon

[Run]
Filename: "{app}\command-manager.exe"; Description: "{cm:LaunchProgram,命令管理器}"; Flags: nowait postinstall skipifsilent

[Registry]
#ifdef AcceptanceFixture
Root: HKCU; Subkey: "Software\Microsoft\Windows\CurrentVersion\Run"; ValueName: "StandaloneCommandManagerAcceptance"; Flags: uninsdeletevalue
#else
Root: HKCU; Subkey: "Software\Microsoft\Windows\CurrentVersion\Run"; ValueName: "StandaloneCommandManager"; Flags: uninsdeletevalue
#endif

; No data files are included in [Files] or [UninstallDelete]. Upgrades and
; uninstall preserve the entire data directory, including settings and exports.
