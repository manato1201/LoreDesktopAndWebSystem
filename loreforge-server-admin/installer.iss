; Inno Setup script for LoreForge Server Admin.
; Source of truth for what gets packaged: build-release/bin/, the same
; windeployqt-populated output directory CPack's install() rule uses (see
; CMakeLists.txt) - LoreForgeServerAdminTests.exe and debug symbols are
; excluded the same way, so this and the NSIS/CPack path never drift out of
; sync on "what ships".
;
; Build with: ISCC.exe installer.iss  (from this directory, after
; `cmake --preset release && cmake --build --preset release`)

#define AppVersion "0.1.0"
#define SourceBin "build-release\bin"

[Setup]
AppId={{7E4E9A2C-1B3F-4C7D-9A5E-2F8B6D4A1E90}
AppName=LoreForge Server Admin
AppVersion={#AppVersion}
AppPublisher=Nebula Studios
DefaultDirName={autopf}\LoreForge Server Admin
DefaultGroupName=LoreForge Server Admin
UninstallDisplayIcon={app}\LoreForgeServerAdmin.exe
OutputDir=installer-output
OutputBaseFilename=LoreForgeServerAdmin-Setup-{#AppVersion}
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
Source: "{#SourceBin}\*"; DestDir: "{app}"; Excludes: "LoreForgeServerAdminTests.exe,*.pdb,*.ilk"; Flags: recursesubdirs ignoreversion

[Icons]
Name: "{group}\LoreForge Server Admin"; Filename: "{app}\LoreForgeServerAdmin.exe"
Name: "{group}\Uninstall LoreForge Server Admin"; Filename: "{uninstallexe}"

[Run]
Filename: "{app}\LoreForgeServerAdmin.exe"; Description: "Launch LoreForge Server Admin"; Flags: nowait postinstall skipifsilent
