; Inno Setup script for Moonglow Toolset's Windows installer.
;
;   iscc /DVersion=0.1.0 /DSource=..\..\target\dist packaging\windows\moonglow.iss
;
; Source holds moonglow.exe, mg.exe and THIRD-PARTY-LICENSES.txt
; (packaging/windows/build-installer.ps1 puts them there). Installs per user
; by default (no administrator rights), per machine when chosen.

#ifndef Version
  #define Version "0.0.0"
#endif
#ifndef Source
  #define Source "..\..\target\dist"
#endif

[Setup]
AppId={{6C1F3A52-8E0B-4E59-9B7A-4D2C7E1A9F03}
AppName=Moonglow Toolset
AppVersion={#Version}
AppVerName=Moonglow Toolset {#Version}
AppPublisher=The Moonglow Toolset contributors
DefaultDirName={autopf}\Moonglow Toolset
DefaultGroupName=Moonglow Toolset
DisableProgramGroupPage=yes
PrivilegesRequired=lowest
PrivilegesRequiredOverridesAllowed=dialog
ArchitecturesAllowed=x64compatible
ArchitecturesInstallIn64BitMode=x64compatible
LicenseFile=..\..\LICENSE
SetupIconFile=..\icons\moonglow.ico
UninstallDisplayIcon={app}\moonglow.exe
OutputDir={#Source}
OutputBaseFilename=Moonglow-{#Version}-windows-x64-setup
Compression=lzma2/max
SolidCompression=yes
WizardStyle=modern
ChangesAssociations=yes

[Tasks]
Name: desktopicon; Description: "{cm:CreateDesktopIcon}"; GroupDescription: "{cm:AdditionalIcons}"; Flags: unchecked
Name: modassoc; Description: "Open modules (.mod) with Moonglow Toolset"; GroupDescription: "File associations:"; Flags: unchecked

[Files]
Source: "{#Source}\moonglow.exe"; DestDir: "{app}"; Flags: ignoreversion
Source: "{#Source}\mg.exe"; DestDir: "{app}"; Flags: ignoreversion
Source: "..\..\LICENSE"; DestDir: "{app}"; DestName: "LICENSE.txt"
Source: "{#Source}\THIRD-PARTY-LICENSES.txt"; DestDir: "{app}"
Source: "..\..\docs\manual\*"; DestDir: "{app}\manual"; Flags: recursesubdirs skipifsourcedoesntexist

[Icons]
Name: "{autoprograms}\Moonglow Toolset"; Filename: "{app}\moonglow.exe"
Name: "{autodesktop}\Moonglow Toolset"; Filename: "{app}\moonglow.exe"; Tasks: desktopicon

[Registry]
Root: HKA; Subkey: "Software\Classes\.mod\OpenWithProgids"; ValueType: string; ValueName: "MoonglowToolset.Module"; ValueData: ""; Flags: uninsdeletevalue; Tasks: modassoc
Root: HKA; Subkey: "Software\Classes\MoonglowToolset.Module"; ValueType: string; ValueName: ""; ValueData: "Neverwinter Nights module"; Flags: uninsdeletekey; Tasks: modassoc
Root: HKA; Subkey: "Software\Classes\MoonglowToolset.Module\DefaultIcon"; ValueType: string; ValueName: ""; ValueData: "{app}\moonglow.exe,0"; Tasks: modassoc
Root: HKA; Subkey: "Software\Classes\MoonglowToolset.Module\shell\open\command"; ValueType: string; ValueName: ""; ValueData: """{app}\moonglow.exe"" ""%1"""; Tasks: modassoc

[Run]
Filename: "{app}\moonglow.exe"; Description: "{cm:LaunchProgram,Moonglow Toolset}"; Flags: nowait postinstall skipifsilent
