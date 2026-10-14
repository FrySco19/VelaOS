#ifndef Stage
#error Pass /DStage=verified-package-directory
#endif
#ifndef Output
#define Output "dist"
#endif
[Setup]
AppId=VelaSoft
AppName=Vela OS
AppVersion=1.0.0-preview
DefaultDirName={localappdata}\Programs\Vela
PrivilegesRequired=lowest
ArchitecturesAllowed=x64compatible
OutputDir={#Output}
OutputBaseFilename=Vela-Soft-Setup
Compression=lzma2
SolidCompression=yes
[Files]
Source: "{#Stage}\*"; DestDir: "{app}"; Flags: recursesubdirs createallsubdirs ignoreversion
[Icons]
Name: "{autodesktop}\Vela OS"; Filename: "{sys}\WindowsPowerShell\v1.0\powershell.exe"; Parameters: "-NoProfile -ExecutionPolicy RemoteSigned -File ""{app}\vela-native.ps1"""; WorkingDir: "{app}"
Name: "{autoprograms}\Vela OS"; Filename: "{sys}\WindowsPowerShell\v1.0\powershell.exe"; Parameters: "-NoProfile -ExecutionPolicy RemoteSigned -File ""{app}\vela-native.ps1"""; WorkingDir: "{app}"
