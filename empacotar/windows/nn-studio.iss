; Instalador do NN Studio para Windows (Inno Setup 6).
; Gerado pelo workflow de release:
;   iscc /DAppVersion=0.2.0 empacotar\windows\nn-studio.iss
; Instala só para o usuário (sem pedir administrador), em
; %LOCALAPPDATA%\Programs\NN Studio, com atalho no menu Iniciar e, se quiser,
; na área de trabalho. Desinstala por "Aplicativos instalados".

#ifndef AppVersion
  #define AppVersion "0.0.0"
#endif

[Setup]
AppId={{6F2C0B7E-3D8A-4C5B-9E1F-7A2D4B6C8E10}
AppName=NN Studio
AppVersion={#AppVersion}
AppVerName=NN Studio {#AppVersion}
AppPublisher=nn-engine
AppPublisherURL=https://github.com/nereuneto/nn-engine
AppSupportURL=https://github.com/nereuneto/nn-engine/issues
AppUpdatesURL=https://github.com/nereuneto/nn-engine/releases
DefaultDirName={autopf}\NN Studio
DefaultGroupName=NN Studio
DisableProgramGroupPage=yes
PrivilegesRequired=lowest
ArchitecturesAllowed=x64compatible
ArchitecturesInstallIn64BitMode=x64compatible
LicenseFile=..\..\LICENSE-MIT
SetupIconFile=..\..\crates\nn-studio\assets\icone.ico
UninstallDisplayIcon={app}\nn-studio.exe
UninstallDisplayName=NN Studio
OutputDir=..\..\dist
OutputBaseFilename=NN-Studio-{#AppVersion}-setup
Compression=lzma2/max
SolidCompression=yes
WizardStyle=modern
CloseApplications=yes

[Languages]
Name: "ptbr"; MessagesFile: "compiler:Languages\BrazilianPortuguese.isl"

[Tasks]
Name: "desktopicon"; Description: "{cm:CreateDesktopIcon}"; GroupDescription: "{cm:AdditionalIcons}"; Flags: unchecked

[Files]
Source: "..\..\target\release\nn-studio.exe"; DestDir: "{app}"; Flags: ignoreversion
Source: "..\..\target\release\nn.exe"; DestDir: "{app}"; Flags: ignoreversion
Source: "..\..\target\release\nn-jogo.exe"; DestDir: "{app}"; Flags: ignoreversion
Source: "..\..\LICENSE-MIT"; DestDir: "{app}"; DestName: "LICENSE-MIT.txt"
Source: "..\..\LICENSE-APACHE"; DestDir: "{app}"; DestName: "LICENSE-APACHE.txt"
Source: "..\..\CHANGELOG.md"; DestDir: "{app}"

[Icons]
Name: "{autoprograms}\NN Studio"; Filename: "{app}\nn-studio.exe"; Comment: "Crie jogos 2D e 3D com a nn-engine"
Name: "{autodesktop}\NN Studio"; Filename: "{app}\nn-studio.exe"; Tasks: desktopicon

[Run]
Filename: "{app}\nn-studio.exe"; Description: "{cm:LaunchProgram,NN Studio}"; Flags: nowait postinstall skipifsilent
