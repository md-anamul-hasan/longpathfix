; LongPathFix installer. Build with scripts\build-installer.ps1 (publishes a self-contained build to dist\stage first).
#define AppName "LongPathFix"
#define AppFullName "LongPathFix - Long Path & Illegal Filename Finder and Fixer"
#ifndef AppVersion
  #define AppVersion "1.1.0"
#endif
#define StageDir "..\dist\stage"

[Setup]
AppId={{25FFAF8A-4D8A-5C88-B27E-FD990DE9FB1B}
AppName={#AppFullName}
AppVersion={#AppVersion}
AppPublisher=Md Anamul Hasan
AppPublisherURL=https://github.com/md-anamul-hasan/longpathfix
AppSupportURL=https://github.com/md-anamul-hasan/longpathfix/issues
DefaultDirName={autopf}\LongPathFix
DefaultGroupName=LongPathFix
DisableProgramGroupPage=yes
UninstallDisplayIcon={app}\LongPathFix.exe
UninstallDisplayName={#AppFullName}
SetupIconFile=..\Assets/icon.ico
LicenseFile=..\LICENSE
OutputDir=..\dist\installer
OutputBaseFilename=LongPathFix-Setup-{#AppVersion}
Compression=lzma2/ultra
SolidCompression=yes
WizardStyle=modern
ArchitecturesAllowed=x64compatible
ArchitecturesInstallIn64BitMode=x64compatible
PrivilegesRequired=lowest
PrivilegesRequiredOverridesAllowed=dialog
CloseApplications=yes
RestartApplications=no

[Languages]
Name: "english"; MessagesFile: "compiler:Default.isl"

[Tasks]
Name: "desktopicon"; Description: "Create a &desktop shortcut"; GroupDescription: "Shortcuts:"

[Files]
Source: "{#StageDir}\*"; DestDir: "{app}"; Flags: ignoreversion recursesubdirs createallsubdirs; Excludes: "*.pdb"

[Icons]
Name: "{autoprograms}\LongPathFix"; Filename: "{app}\LongPathFix.exe"
Name: "{autodesktop}\LongPathFix"; Filename: "{app}\LongPathFix.exe"; Tasks: desktopicon

[Run]
Filename: "{app}\LongPathFix.exe"; Description: "Launch LongPathFix"; Flags: nowait postinstall skipifsilent
