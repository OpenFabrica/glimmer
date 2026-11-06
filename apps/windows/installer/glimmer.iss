; 微明 Windows 输入法安装脚本（Inno Setup）。
;
; 装到 Program Files\Glimmer（64 位），把 TSF DLL（64 位与 32 位各一份，见 README「安装布局」）、Server、设置程序与随包数据装在一起，
; 然后：① 给安装目录加 ALL APPLICATION PACKAGES 读+执行权限（UWP/AppContainer 应用——任务栏搜索、
; 设置——才能加载 DLL）；② regsvr32 注册文本服务，64 位与 32 位各注册一次（图标落到 %ProgramData%\Glimmer）；
; ③ 在「启动」文件夹放 Server 快捷方式（登录时由 Explorer 走 ShellExecute 拉起，uiAccess 才生效——
;    计划任务直接拉起拿不到 uiAccess）；④ 装完点 Finish 立即以原用户 ShellExecute 起一次 Server，免得先注销。
; 卸载反向：删旧任务（若有）、杀 Server、反注册 DLL，再删文件（用户数据 %APPDATA%\Glimmer 保留；启动快捷方式 Inno 自动删）。
;
; 升级：DLL 被加载进每个应用进程，文件锁着覆盖不了，所以 DLL 按版本起名（glimmer_tsf-<版本>.dll）并排装，
; 注册新的，旧的装完后删（删不掉的登记成重启后删）；已开着的应用继续用旧 DLL 直到重启，Server 两个版本都服务。
; Inno 的 CloseApplications 会用 Restart Manager 找出所有加载了 *.dll 的进程要求关闭——对输入法 DLL 就是关一切，故关掉；
; 只有 Server / 设置程序两个 exe 要覆盖，安装前自己 taskkill。
;
; 版本号由打包脚本用 /DAppVersion=... 传入，缺省 0.1.0。用法见本目录 README.md。

#ifndef AppVersion
  #define AppVersion "0.1.0"
#endif
; VersionInfoVersion 只认 a.b.c.d 数字；版本带预发布后缀（0.1.0-alpha.1）时由打包脚本传去掉后缀的数字版本。
#ifndef AppVersionNumeric
  #define AppVersionNumeric AppVersion
#endif
#define AppName "微明"
#define Publisher "微明"
#define WebsiteUrl "https://glimmer.im"
; 脚本相对仓库根（ime/）：installer → windows → apps → ime
#define Repo "..\..\.."
; 按版本起名的 TSF DLL（见文件头「升级」）。
#define TsfDll "glimmer_tsf-" + AppVersion + ".dll"
#define TsfDll32 "glimmer_tsf-" + AppVersion + "-x86.dll"

[Setup]
AppId={{2AF2FE03-7FAF-40D5-BFEB-82AE52C48C25}
AppName={#AppName}
AppVersion={#AppVersion}
AppPublisher={#Publisher}
AppSupportURL={#WebsiteUrl}
VersionInfoVersion={#AppVersionNumeric}
DefaultDirName={autopf}\Glimmer
DefaultGroupName={#AppName}
DisableProgramGroupPage=yes
ArchitecturesAllowed=x64compatible
ArchitecturesInstallIn64BitMode=x64compatible
; Windows 10 1809 起（Windows App Runtime 的下限，见 docs\notes\windows-win10.md）。
MinVersion=10.0.17763
PrivilegesRequired=admin
; 别让 Restart Manager 去关所有加载了 DLL 的应用（那是每一个有文本框的应用）。
CloseApplications=no
OutputDir={#Repo}\target\installer
OutputBaseFilename=Glimmer-{#AppVersion}-Setup
SetupIconFile={#Repo}\apps\windows\tsf\resources\glimmer.ico
UninstallDisplayIcon={app}\glimmer.ico
Compression=lzma2
SolidCompression=yes
WizardStyle=modern

[Languages]
Name: "chs"; MessagesFile: "compiler:Languages\ChineseSimplified.isl"

[Messages]
; 完成页：DLL 会装进每个应用进程，装之前就开着的应用要用新版必须重启（见文件头「升级」）。
; 这是最容易被误解成「设置/新版没生效」的一点，所以放在完成页明说。
FinishedLabel=安装完成。请注销后重新登录（或重启电脑），微明才会在所有应用里生效。%n%n不方便注销的话，先关掉再重新打开要打字的应用也可以。

[Files]
; —— 二进制 ——
; DLL 按版本起名并排装；卸载时若仍被占用，登记成重启后删。
Source: "{#Repo}\target\release\glimmer_tsf.dll";      DestDir: "{app}"; DestName: "{#TsfDll}"; Flags: ignoreversion uninsrestartdelete
Source: "{#Repo}\target\i686-pc-windows-msvc\release\glimmer_tsf.dll"; DestDir: "{app}"; DestName: "{#TsfDll32}"; Flags: ignoreversion uninsrestartdelete
Source: "{#Repo}\target\release\glimmer-server.exe";   DestDir: "{app}"; Flags: ignoreversion
Source: "{#Repo}\target\release\glimmer-settings.exe"; DestDir: "{app}"; Flags: ignoreversion
; 设置程序自带一份 Windows App Runtime（自包含部署：Windows 10 上机器装的框架包用不了，见 docs\notes\windows-win10.md）；
; 文件由 build.ps1 按 settings-runtime.txt 从 target\release 挑进 target\installer\settings-runtime，必须与 exe 同级。
Source: "{#Repo}\target\installer\settings-runtime\*"; DestDir: "{app}"; Flags: ignoreversion recursesubdirs createallsubdirs
Source: "{#Repo}\apps\windows\tsf\resources\glimmer.ico"; DestDir: "{app}"; Flags: ignoreversion
; —— 随包生成数据（只装运行时要的 .qj / .tsv，不装 dev 中间产物）——
Source: "{#Repo}\data\generated\dict.qj";        DestDir: "{app}\data\generated";       Flags: ignoreversion
Source: "{#Repo}\data\generated\lm.qj";          DestDir: "{app}\data\generated";       Flags: ignoreversion
Source: "{#Repo}\data\generated\glossary-en.qj"; DestDir: "{app}\data\generated";       Flags: ignoreversion
Source: "{#Repo}\data\generated\glossary-ja.qj"; DestDir: "{app}\data\generated";       Flags: ignoreversion
Source: "{#Repo}\data\generated\glossary-zh.qj"; DestDir: "{app}\data\generated";       Flags: ignoreversion
Source: "{#Repo}\data\generated\glossary-es.qj"; DestDir: "{app}\data\generated";       Flags: ignoreversion
Source: "{#Repo}\data\generated\english.tsv";    DestDir: "{app}\data\generated";       Flags: ignoreversion
; 英文语言模型与误拼表可选：没有就不装，英文候选按词频排、少一档纠错
Source: "{#Repo}\data\generated\lm-en.qj";       DestDir: "{app}\data\generated";       Flags: ignoreversion skipifsourcedoesntexist
Source: "{#Repo}\data\generated\english-misspellings.tsv"; DestDir: "{app}\data\generated"; Flags: ignoreversion skipifsourcedoesntexist
Source: "{#Repo}\data\generated\dicts\*.qj";     DestDir: "{app}\data\generated\dicts";  Flags: ignoreversion
; AI 领域词库的署名、来源和许可证。
Source: "{#Repo}\assets\lexicon\ai\LICENSE.*"; DestDir: "{app}\assets\lexicon\ai"; Flags: ignoreversion
Source: "{#Repo}\assets\lexicon\ai\sources.json"; DestDir: "{app}\assets\lexicon\ai"; Flags: ignoreversion
Source: "{#Repo}\assets\lexicon\ai\README.md"; DestDir: "{app}\assets\lexicon\ai"; Flags: ignoreversion
; 五笔码表三版（dict-convert 打成的 .qj，各自的许可与署名随包）；缺哪版就不装哪版，[general] wubi 指到缺的版本时 Server 记 warn 当没开
Source: "{#Repo}\data\generated\wubi86.qj";               DestDir: "{app}\data\generated";     Flags: ignoreversion skipifsourcedoesntexist
Source: "{#Repo}\assets\wubi\wubi86\LICENSE.LGPL-3.0";    DestDir: "{app}\assets\wubi\wubi86";  Flags: ignoreversion skipifsourcedoesntexist
Source: "{#Repo}\assets\wubi\wubi86\AUTHORS";             DestDir: "{app}\assets\wubi\wubi86";  Flags: ignoreversion skipifsourcedoesntexist
Source: "{#Repo}\data\generated\wubi98.qj";               DestDir: "{app}\data\generated";     Flags: ignoreversion skipifsourcedoesntexist
Source: "{#Repo}\assets\wubi\wubi98\LICENSE.LGPL-3.0";    DestDir: "{app}\assets\wubi\wubi98";  Flags: ignoreversion skipifsourcedoesntexist
Source: "{#Repo}\data\generated\wubixsj.qj";              DestDir: "{app}\data\generated";     Flags: ignoreversion skipifsourcedoesntexist
; 新世纪上游没带 LICENSE 文件，AUTHORS 里声明 LGPL-3.0，全文用 wubi86 随包的那份
Source: "{#Repo}\assets\wubi\wubixsj\AUTHORS";            DestDir: "{app}\assets\wubi\wubixsj"; Flags: ignoreversion skipifsourcedoesntexist
; —— 本地整句模型（tools/release/pack-model.sh 打成的单文件 data\model\model.qjm；没有就不装，Server 不重排）——
Source: "{#Repo}\data\model\model.qjm"; DestDir: "{app}\data\model"; Flags: ignoreversion skipifsourcedoesntexist
; —— 随 git 的资源 ——
Source: "{#Repo}\assets\emoji\emoji-zh.tsv";     DestDir: "{app}\assets\emoji";  Flags: ignoreversion
Source: "{#Repo}\assets\emoji\emoji-en.tsv";     DestDir: "{app}\assets\emoji";  Flags: ignoreversion
Source: "{#Repo}\assets\levels\levels-en.tsv";   DestDir: "{app}\assets\levels"; Flags: ignoreversion
Source: "{#Repo}\assets\levels\levels-ja.tsv";   DestDir: "{app}\assets\levels"; Flags: ignoreversion
Source: "{#Repo}\assets\sample\dict.tsv";        DestDir: "{app}\assets\sample"; Flags: ignoreversion

[Icons]
Name: "{group}\微明设置"; Filename: "{app}\glimmer-settings.exe"; IconFilename: "{app}\glimmer.ico"
Name: "{group}\卸载微明"; Filename: "{uninstallexe}"
; 登录自启：登录时 Explorer 走 ShellExecute 拉起本快捷方式 → AppInfo 授予 uiAccess，候选窗才能盖过商店 / 任务栏搜索。
; 用 {commonstartup}（所有用户「启动」文件夹）而非 {userstartup}：本安装器是 admin 机器级安装，
; admin 模式下写每用户区会落到「谁提权就写谁」的 profile（Inno 会告警且可能不是目标用户）；
; 机器级「启动」项对每个登录用户都在其会话里由该用户的 Explorer 拉起，仍是 per-user 运行、仍授予 uiAccess。
; （计划任务直接拉起拿不到 uiAccess，故不用 schtasks。）
Name: "{commonstartup}\微明 Server"; Filename: "{app}\glimmer-server.exe"; WorkingDir: "{app}"; IconFilename: "{app}\glimmer.ico"

[Run]
; ① UWP/AppContainer 应用要能读安装目录才能加载 DLL（*S-1-15-2-1 = ALL APPLICATION PACKAGES，按 SID 与语言无关）。
Filename: "{sys}\icacls.exe"; Parameters: """{app}"" /grant *S-1-15-2-1:(OI)(CI)RX /T /C /Q"; \
  Flags: runhidden waituntilterminated; StatusMsg: "配置应用容器权限…"
; ② 注册文本服务（写 HKCR + 图标到 %ProgramData%\Glimmer\glimmer.ico）。注册的是本版本的 DLL，
;    InprocServer32 指向新文件；旧版本的 DLL **不能** regsvr32 /u（那会把整个 CLSID 注销掉）。
Filename: "{sys}\regsvr32.exe"; Parameters: "/s ""{app}\{#TsfDll}"""; \
  Flags: runhidden waituntilterminated; StatusMsg: "注册输入法…"
; 32 位那份用 SysWOW64 里的 32 位 regsvr32 注册，InprocServer32 才落到 WOW6432Node 下给 32 位进程用。
Filename: "{syswow64}\regsvr32.exe"; Parameters: "/s ""{app}\{#TsfDll32}"""; \
  Flags: runhidden waituntilterminated; StatusMsg: "注册输入法（32 位）…"
; ④ 装完立即起一次 Server 见 [Code] 的 NextButtonClick：uiAccess=true 的 exe 不能用
;    CreateProcess / runasoriginaluser 拉起（报 740），必须走 ShellExecute（等同双击）。

[UninstallRun]
; 反向：先删登录任务、杀 Server / 设置程序、反注册 DLL，Inno 再删文件（DLL 若仍被占用，重启后删）。
Filename: "{sys}\schtasks.exe"; Parameters: "/delete /tn ""Glimmer Server"" /f"; \
  Flags: runhidden; RunOnceId: "DelLogonTask"
Filename: "{sys}\taskkill.exe"; Parameters: "/im glimmer-server.exe /f"; \
  Flags: runhidden; RunOnceId: "KillServer"
Filename: "{sys}\taskkill.exe"; Parameters: "/im glimmer-settings.exe /f"; \
  Flags: runhidden; RunOnceId: "KillSettings"
Filename: "{sys}\regsvr32.exe"; Parameters: "/u /s ""{app}\{#TsfDll}"""; \
  Flags: runhidden; RunOnceId: "UnregDll"
Filename: "{syswow64}\regsvr32.exe"; Parameters: "/u /s ""{app}\{#TsfDll32}"""; \
  Flags: runhidden; RunOnceId: "UnregDll32"

[InstallDelete]
; 更早版本装在当前用户「启动」文件夹里的自启快捷方式：与机器级那份并存会起两个 Server（两条状态条）。
Type: files; Name: "{userstartup}\Glimmer Server.lnk"
; 更早版本装的模型三件套（现在只带 model.qjm）：留着白占 56 MB。
Type: files; Name: "{app}\data\model\model.safetensors"
Type: files; Name: "{app}\data\model\config.json"
Type: files; Name: "{app}\data\model\vocab.json"

[UninstallDelete]
; 历次升级留下的旧版本 DLL（正常在升级时就删了；仍被占用的会留到这里）。
Type: files; Name: "{app}\glimmer_tsf-*.dll"

[Code]
procedure KillProcess(const Image: String);
var
  ResultCode: Integer;
begin
  Exec(ExpandConstant('{sys}\taskkill.exe'), '/im ' + Image + ' /f', '',
    SW_HIDE, ewWaitUntilTerminated, ResultCode);
end;

{ 同版本重装（开发期反复装）：目标文件名与已加载的 DLL 撞名，覆盖不了但 Windows 允许改名，
  先把它改成 glimmer_tsf-<版本>.old-<随机>.dll 腾出名字，装完由 DeleteStaleDlls 删掉 / 登记重启后删。 }
procedure RetireLoadedDll(const Name: String);
var
  Path, Retired: String;
begin
  Path := ExpandConstant('{app}\') + Name;
  if FileExists(Path) then
  begin
    Retired := ExpandConstant('{app}\glimmer_tsf-{#AppVersion}.old-') + IntToStr(Random(1000000)) + '.dll';
    if not RenameFile(Path, Retired) then
      Log('改名旧 DLL 失败: ' + Path);
  end;
end;

{ 覆盖前先结束 Server 与设置程序（只有这两个 exe 要覆盖；DLL 按版本并排装，不用关应用）。
  没在跑时 taskkill 返回非 0，忽略。 }
function PrepareToInstall(var NeedsRestart: Boolean): String;
begin
  KillProcess('glimmer-server.exe');
  KillProcess('glimmer-settings.exe');
  RetireLoadedDll('{#TsfDll}');
  RetireLoadedDll('{#TsfDll32}');
  Result := '';
end;

{ 清掉旧版本建的「登录自启」计划任务（现在改用「启动」文件夹快捷方式，见 [Icons]）。
  计划任务直接拉起 Server 拿不到 uiAccess，升级安装时删掉它，免得它在登录时抢先以非 uiAccess 方式
  起 Server 并占住命名管道，让快捷方式那份起不来。没有旧任务时 schtasks 返回非 0，忽略即可。 }
procedure DeleteLegacyLogonTask;
var
  ResultCode: Integer;
begin
  Exec('schtasks.exe', '/delete /tn "Glimmer Server" /f', '',
    SW_HIDE, ewWaitUntilTerminated, ResultCode);
end;

{ 删掉旧版本的 DLL（含没带版本号的最早那份）。仍被某个应用加载着的删不掉，登记成重启后删：
  那些应用重启前继续用旧 DLL，Server 两个版本都服务。 }
procedure DeleteStaleDlls;
var
  Dir, Path: String;
  Found: TFindRec;
begin
  Dir := ExpandConstant('{app}');
  if FindFirst(Dir + '\glimmer_tsf*.dll', Found) then
  begin
    try
      repeat
        if (CompareText(Found.Name, '{#TsfDll}') <> 0) and (CompareText(Found.Name, '{#TsfDll32}') <> 0) then
        begin
          Path := Dir + '\' + Found.Name;
          if not DeleteFile(Path) then
            RestartReplace(Path, '');
        end;
      until not FindNext(Found);
    finally
      FindClose(Found);
    end;
  end;
end;

procedure CurStepChanged(CurStep: TSetupStep);
begin
  if CurStep = ssPostInstall then
  begin
    DeleteLegacyLogonTask;
    DeleteStaleDlls;
  end;
end;

{ 装完在完成页点 Finish 后立即起一次 Server。
  uiAccess=true 的 exe 不能用 CreateProcess / runasoriginaluser 拉起（报 740），
  必须以原（非提升）用户身份 ShellExecute（等同双击），AppInfo 才会授予 uiAccess 高 z-band 权限。 }
function NextButtonClick(CurPageID: Integer): Boolean;
var
  ErrorCode: Integer;
begin
  Result := True;
  if (CurPageID = wpFinished) and (not WizardSilent) then
    ShellExecAsOriginalUser(
      '', ExpandConstant('{app}\glimmer-server.exe'), '', '',
      SW_SHOWNORMAL, ewNoWait, ErrorCode);
end;
