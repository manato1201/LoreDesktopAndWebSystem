; Inno Setup script for LoreForge Client.
; Source of truth for what gets packaged: build-release/bin/, the same
; windeployqt-populated output directory CPack's install() rule uses (see
; CMakeLists.txt) - LoreForgeClientTests.exe and debug symbols are excluded
; the same way, so this and the NSIS/CPack path never drift out of sync on
; "what ships".
;
; Build with: ISCC.exe installer.iss  (from this directory, after
; `cmake --preset release && cmake --build --preset release`)

#define AppVersion "0.1.0"
#define SourceBin "build-release\bin"

[Setup]
AppId={{B4B6E3B0-6C2E-4F1A-9E3A-6C6E1B9B7C31}
AppName=LoreForge Client
AppVersion={#AppVersion}
AppPublisher=Nebula Studios
DefaultDirName={autopf}\LoreForge Client
DefaultGroupName=LoreForge Client
UninstallDisplayIcon={app}\LoreForgeClient.exe
OutputDir=installer-output
OutputBaseFilename=LoreForgeClient-Setup-{#AppVersion}
Compression=lzma2
SolidCompression=yes
SetupIconFile=resources\icon.ico
WizardStyle=modern
DisableProgramGroupPage=yes
ArchitecturesAllowed=x64compatible
ArchitecturesInstallIn64BitMode=x64compatible

[Languages]
Name: "english"; MessagesFile: "compiler:Default.isl"

[Files]
; recursesubdirs pulls in the Qt runtime/QML/plugin directories windeployqt
; populated; the three Excludes mirror CPack's PATTERN EXCLUDE list exactly.
Source: "{#SourceBin}\*"; DestDir: "{app}"; Excludes: "LoreForgeClientTests.exe,*.pdb,*.ilk"; Flags: recursesubdirs ignoreversion

[Icons]
Name: "{group}\LoreForge Client"; Filename: "{app}\LoreForgeClient.exe"
Name: "{group}\Uninstall LoreForge Client"; Filename: "{uninstallexe}"

[Run]
Filename: "{app}\LoreForgeClient.exe"; Description: "Launch LoreForge Client"; Flags: nowait postinstall skipifsilent
