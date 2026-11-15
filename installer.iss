#define AppName "Aireluma"
#define AppVersion "0.8.2"
[Setup]
AppId={{0C1E7E3A-75E5-4FA0-B6A1-30B0757E4111}
AppName={#AppName}
AppVersion={#AppVersion}
DefaultDirName={localappdata}\Programs\Aireluma
PrivilegesRequired=lowest
DefaultGroupName={#AppName}
OutputDir=installer
OutputBaseFilename=Aireluma-Setup
Compression=lzma2/fast
SolidCompression=yes
WizardStyle=modern
SetupIconFile=assets\aireluma.ico
WizardImageFile=assets\wizard.bmp
WizardSmallImageFile=assets\wizard-small.bmp
UninstallDisplayIcon={app}\airplay-windows.exe
[Files]
Source: "package\*"; DestDir: "{app}"; Flags: ignoreversion recursesubdirs createallsubdirs
[Icons]
Name: "{autoprograms}\Удалить Aireluma"; Filename: "{uninstallexe}"
Name: "{autoprograms}\Aireluma"; Filename: "{app}\airplay-windows.exe"; AppUserModelID: "Aireluma.Desktop"
Name: "{autodesktop}\Aireluma"; Filename: "{app}\airplay-windows.exe"; AppUserModelID: "Aireluma.Desktop"; Tasks: desktopicon
[Tasks]
Name: "desktopicon"; Description: "Создать ярлык на рабочем столе"; Flags: unchecked
[Run]
Filename: "{app}\airplay-windows.exe"; Description: "Запустить Aireluma"; Flags: nowait postinstall skipifsilent

[Code]
procedure StopAireluma;
var H: HWND;
begin
  H := FindWindowByClassName('AirPlayWindow');
  if H <> 0 then begin
    PostMessage(H, $8010, 0, 0);
    Sleep(1000);
  end;
end;

function InitializeSetup: Boolean;
begin
  StopAireluma;
  Result := True;
end;

function InitializeUninstall: Boolean;
begin
  StopAireluma;
  Result := True;
end;
