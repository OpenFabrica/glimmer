| `.github/workflows/ci.yml` | push main、PR | `core`（Linux）fmt / clippy / 全 workspace 测试（排除 IMK 壳）；`macos` 编 IMK 壳并跑它的测试；`windows` 编 Server / TSF DLL / Settings 并跑测试。仓库公开，Actions 不计费 |
| `.github/workflows/audit.yml` | 每周一、Cargo.lock 变动 | `cargo audit`（RustSec 已知漏洞） |
| `.github/dependabot.yml` | 每周一 | Cargo 依赖与钉 commit 的 actions 的更新 PR |# 发版流程

2026-09-07 搭起来的：GitHub Actions 按标签打包、建 Release、生成官网下载页用的 `releases.json`。
这里记怎么发一版、各环节的依赖，以及官网怎么消费产物。

## 一次发版做什么

（下面以 macOS 为例；Windows 与 Linux 见各自一节，步骤同构。）

1. 改 `apps/macos/Cargo.toml` 的 `version`（`apps/macos` 的 Info.plist 版本号从这里取，pkg 文件名也是）：把 `0.1.2-dev` 改成 `0.1.2`。
   **发版之间版本号一直带 `-dev`**（Rust nightly / Firefox Nightly 那套）：Cargo.toml 写 `0.1.2-dev`，`bundle.sh` 打包时再接上 git 短哈希，
   本地装的、CI 中间构建的都显示 `0.1.2-dev-1a2b3c4`（工作区有改动加 `+`），测试时一眼知道装的是哪个提交；版本号干净的一定是线上包；
   带 `-dev` 的标签 CI 直接拒绝。pkg 的 `--version` 与 `distribution.xml` 只认数字点号，`bundle.sh` 去掉后缀再传，Info.plist 与 pkg 文件名保留完整版本。
   **各平台壳版本号独立**：macOS 的版本只在 `apps/macos/Cargo.toml`，跟 workspace 与其他壳无关（例：mac 到 `0.1.1`、win 还在 `0.1.0`）。
2. `CHANGELOG.md` 顶上加一节 `## <版本> · <日期> · <渠道>`（渠道是 `alpha` / `beta` / `rc` / `stable`），一行一条、面向用户的措辞。
   **更新日志手写，不由提交自动生成**：提交信息里有大量内部改动（拆模块、修 RefCell 重入），用户看不懂也不关心；
   做法是发版前按上个标签以来的 `git log` 起草几条，人审一遍再定稿。
3. 提交，打**带平台前缀**的注释标签并推：`git tag -a macos-v0.1.1 -m "微明 macOS 0.1.1" && git push origin main macos-v0.1.1`
   （标签按平台加前缀 `macos-v*` / 将来 `windows-v*`，因为各平台版本号独立、光靠 `v<版本>` 会撞车；旧的 `v*` 标签仍能被官网识别，向后兼容）。
3b. 标签推出去之后紧接一个普通提交把版本号改成下一个开发版（只是改 Cargo.toml，不打标签、不建 Release；-dev 版本永远没有标签与 Release）：`apps/macos/Cargo.toml` 改成 `0.1.3-dev`（Windows 同理 `0.1.0-alpha.3-dev`），本地从此打的包都带 `-dev`。
4. `release.yml` 跑完后 GitHub Release 上有 `Glimmer-<版本>-arm64.pkg`、`Glimmer-<版本>-x86_64.pkg`、`SHA256SUMS`、`build-info.json`（提交、构建时间、工具链）、`releases.json`。
5. 官网由 Cloudflare Workers Builds 按官网仓库的提交自动构建，没有可调用的构建钩子，所以主仓库靠**往官网仓库推一个小提交**来触发：
   `tools/release/bump-website.sh` 把版本标签与文档提交号写进官网的 `src/content/upstream.json` 并提交推送（提交者 glimmer-ci）。
   配了 `GLIMMER_WEB_TOKEN`（对 glimmer-web 有 Contents: read and write 的 fine-grained PAT）release.yml 末尾自动做；
   官网文档只随发版更新（`docs/user/` 平时改动不推官网，免得文档领先于用户装到的版本）。没配就在官网仓库随便提交一次（或本地跑这个脚本）。
   官网构建时才拉最新 Release 的 `releases.json` 与主仓库 `docs/user`，所以提交内容本身不重要，`upstream.json` 只是留个记录、
   顺便让文档按记下的提交号拉（版本对得上）。

workflow 会核对 `apps/macos/Cargo.toml` 版本号与标签（去掉 `macos-v` 前缀后）一致，不一致直接失败，避免打出版本号错的包。

Rust 工具链由 `rust-toolchain.toml` 钉版本（现在 1.96.0），两个 workflow 里 `dtolnay/rust-toolchain@master` 的 `toolchain:` 输入写同一个号；升级 Rust 时三处一起改。

## 一键发版（本机）

Apple 签名 secrets 没配进 CI，macOS 正式包在本机签名公证；Windows 仍由 CI 打。三个脚本把整套流程串起来：

- `tools/release/release.sh [--macos] [--windows] [--yes] [--skip-website]`：缺省两个平台一起发。
  从 Cargo.toml 的 `-dev` 版本推出发版号 → CHANGELOG 的 `## <版本> · 未发布 · <渠道>` 换成今天（没有该小节或没有条目就停）→
  `chore(release)` 提交并打 `macos-v*` / `windows-v*` 标签 → 确认后推送 → 取消 CI 的 macOS 无签名构建、跑 `macos-local.sh` →
  等 Windows CI 发完 → 两个平台都发时按全部 Release 重生成一次 `releases.json` → `deploy-website.sh` → 版本号推到下一个 `-dev` 并推送。
  推送前取消会撤回发版提交与标签。
- `tools/release/macos-local.sh [--no-publish] [--fetch-data]`：要求版本不带 `-dev`、工作区干净、标签指向 HEAD。
  先下载 `data.lock` 锁定的数据包逐文件比对本机 `data/generated/` 与模型（不符就停；`--fetch-data` 用锁定数据覆盖），
  打 arm64 与 x86_64 两个 pkg（Developer ID 签名），公证钉票据并用 `spctl` 核对，写 `SHA256SUMS` / `build-info.json`，
  建 Release 并跑 `publish-releases-json.sh`。公证凭据用 `GLIMMER_NOTARY_PROFILE`，没设就读仓库根 `.env` 的 `APPLE_ID` / `APPLE_PASSWORD` / `APPLE_TEAM_ID`。产物在 `target/pkg/macos-v<版本>/`。
- `tools/release/deploy-website.sh [标签…]`：官网仓库（缺省 `../glimmer-web`，`GLIMMER_WEB_DIR` 可改）记一笔 `upstream.json` 并提交，
  删掉 `src/content/releases.json` 缓存（一小时内不重拉，不删会部署出旧列表），`GLIMMER_DOCS_SOURCE=git npm run build`，`npx wrangler deploy`，
  最后逐个探测下载页上的安装包地址，有打不开的就报错。主仓库 HEAD 要已推到 origin/main（文档按提交号拉）。

Release 发出后别改回草稿：草稿的下载地址公开访问是 404，已生成的 `releases.json` 仍指着它（2026-09-17 的 macOS 0.1.5 就是这样挂在官网上的）；
官网的 `sync-releases.mjs` 现在会去掉 404 的安装包，但该删的草稿还是要删。

## Windows 发版

1. 改 `apps/windows/{server,tsf,settings}/Cargo.toml` 的 `version`（三个一起改；打包脚本与 workflow 读 `server` 那份）。
   同样带 `-dev`：发版之间是 `0.1.0-alpha.2-dev`，发版提交改成 `0.1.0-alpha.2`；Inno 的 `VersionInfoVersion` 只认数字，`build.ps1` 把整个预发布后缀去掉再传，安装包与 DLL 文件名保留完整版本。
   内测版用 semver 预发布号 `0.1.0-alpha.1`、`0.1.0-alpha.2`…：CHANGELOG 按版本号索引、官网按版本号列条目，
   与 macOS 的 `0.1.0` / `0.1.1` 不能同号；Inno 的 `VersionInfoVersion` 只认数字，`build.ps1` 会把后缀去掉再传。
2. `CHANGELOG.md` 加一节 `## 0.1.0-alpha.1 · 日期 · alpha`。
3. 打标签 `windows-v0.1.0-alpha.1` 推送。`release.yml` 的 `windows` job 在 `windows-latest` 上：核对版本 → 下载 `data` Release
   → 装 Inno Setup 7.1.0（与开发机同版本，钉死 GitHub Release 的安装程序）→ `build.ps1`→ 建 Release（`Glimmer-<版本>-Setup.exe` + `SHA256SUMS` + `build-info.json`）
   → `publish-releases-json.sh` 生成 `releases.json`，挂到本次发布并覆盖到 GitHub latest 那版上（官网只读 latest 的）。
4. **没有代码签名证书时** workflow 设 `GLIMMER_UIACCESS=0`：没签名的 exe 带 uiAccess=true 起不来。
   代价是候选窗在任务栏搜索 / 设置这类 UWP 宿主里可能被盖住，用户文档与 CHANGELOG 已列为已知问题。
   Certum 开源证书办下来后：在 `build.ps1` 加 signtool 一步（`sign-local.ps1` 是本机自签的参考），workflow 去掉那个环境变量。
   SmartScreen 对无签名安装包的拦截也一并消失。
5. 官网：`releases.json` 里 Windows 包由文件名 `-Setup.exe` 识别（`ASSET_KINDS`），下载页按访问者平台取「有该平台安装包的最新版本」
   （`latestFor`），所以 macOS 与 Windows 各自的最新版互不干扰。

## Linux 发版

1. 改 `apps/linux/Cargo.toml` 的 `version`（打包脚本与 workflow 都读它）：发版之间是 `0.1.0-linux.1-dev`，发版提交改成 `0.1.0-linux.1`，标签推出去后改成下一个 `-dev`。
   **版本号不能与其他平台撞号**：更新日志与官网都按版本号索引，Windows 已占用 `0.1.0-alpha.N`，所以 Linux 用 `0.1.0-linux.N` 这一串预发布号。
2. `CHANGELOG.md` 加一节 `## 0.1.0-linux.1 · 日期 · alpha`。
3. 打标签 `linux-v0.1.0-linux.1` 推送。`release.yml` 的 Linux 部分分四段，两半在不同系统上编（amd64 与 arm64 都在原生 runner 上，不交叉编译）：
   - `linux-check`（`ubuntu-24.04`）：门禁，版本 = 标签、不带 `-dev`、标签在 main 上；后面的 job 都 `needs` 它。
   - `linux-ibus`（matrix `ubuntu-22.04` / `ubuntu-22.04-arm`）：`cargo build --release --locked -p glimmer-linux`，传 artifact `ibus-<arch>`（`glimmer-ibus`）。
   - `linux-fcitx5`（matrix `ubuntu-24.04` / `ubuntu-24.04-arm`）：apt 装 Fcitx5 5.1 构建依赖，编 `glimmer-fcitx5` 静态库 + cmake，`DESTDIR=<目录> cmake --install`，传 artifact `fcitx5-<arch>`（整个安装目录）。
   - `linux-package`（每架构一个，matrix `ubuntu-22.04` / `ubuntu-22.04-arm`）：下载两个 artifact 与 `data` Release（按 `SHA256SUMS` 校验），
     `GLIMMER_IBUS_BIN=… GLIMMER_FCITX5_STAGE=… apps/linux/scripts/package.sh` 出 deb，Depends 里带 `t64` 直接失败，传 artifact `deb-<arch>`。
   `linux-release` job 收齐两个 deb，生成 `SHA256SUMS` 与 `build-info.json`（数据摘要取 `data` Release 的 `SHA256SUMS`），建 Release，跑 `publish-releases-json.sh`，配了令牌就 `bump-website.sh`。

   **为什么这样拆**：deb 的 Depends 跟着构建机走。在 24.04 上编、打包得到 `libc6 (>= 2.39), libssl3t64`，只有 24.04 起装得上；
   IBus 引擎在 22.04 上编、在 22.04 上跑 `dpkg-shlibdeps`，得到 `libc6 (>= 2.3x), libgcc-s1, libssl3 (>= 3.0.0)` 一类，22.04 / Debian 12 / 24.04 都能装
   （24.04 的 `libssl3t64` 有 `Provides: libssl3`）。Fcitx5 5.1 开发包只在 24.04 / Debian 13 起才有，插件只能在 24.04 上编；它只在 fcitx5 ≥ 5.1 的系统上被加载，所以不进 Depends 的计算。
4. 产物：`Glimmer-<版本>-amd64.deb`、`Glimmer-<版本>-arm64.deb`。只出 deb，不出 tar.gz / rpm / AppImage；同一个 deb 同时带 IBus 引擎与 Fcitx5 插件。

**版本号规则**：文件名用 Cargo 原样的版本；带 `-dev` 时接 git 短哈希，工作区有改动再加 `+`（`Glimmer-0.1.0-linux.1-dev-1a2b3c4-arm64.deb`）。
deb 的 `Version` 字段把 `-` 换成 `~`（`~` 在 dpkg 比较里排在一切之前，预发布版低于正式版）：`0.1.0-linux.1` → `0.1.0~linux.1`；
dev 版再接 `+g<短哈希>`，有改动加 `.dirty`：`0.1.0~linux.1~dev+g1a2b3c4`，比 `0.1.0~linux.1` 低，正式版装上去会覆盖 dev 版。

**deb 布局**（与 `glimmer.iss` 的文件清单一致，`glimmer_platform::resources::bundled_root()` 认 exe 同级的 `data/` 与 `assets/`）：

| 路径 | 内容 |
|---|---|
| `/usr/lib/glimmer/glimmer-ibus` | 引擎进程，IBus 按组件描述以 `--ibus` 拉起 |
| `/usr/lib/glimmer/data/generated/` | `dict.qj`、`lm.qj`、`glossary-{en,ja,zh}.qj`、`english.tsv`、`dicts/*.qj`、可选 `wubi86.qj` / `wubi98.qj` / `wubixsj.qj`（有几份带几份） |
| `/usr/lib/glimmer/data/model/model.qjm` | 本地整句模型，可选 |
| `/usr/lib/glimmer/assets/` | `emoji/emoji-{zh,en}.tsv`、`levels/levels-{en,ja}.tsv`、`sample/dict.tsv`、`wubi/wubi86/{LICENSE.LGPL-3.0,AUTHORS}`、`wubi/wubi98/LICENSE.LGPL-3.0`、`wubi/wubixsj/AUTHORS`（各自的码表带上时） |
| `/usr/share/ibus/component/glimmer.xml` | IBus 组件（`app.glimmer.IBus`，引擎名 `glimmer`），模板 `apps/linux/packaging/glimmer.xml.in` |
| `/usr/lib/<multiarch>/fcitx5/glimmer.so` | Fcitx5 插件（C++ 半边静态链进 `libglimmer_fcitx5.a`），与 IBus 引擎共用 `/usr/lib/glimmer` 下的资源 |
| `/usr/share/fcitx5/addon/glimmer.conf`、`/usr/share/fcitx5/inputmethod/glimmer.conf` | Fcitx5 的插件与输入法描述（输入法名 `glimmer`，图标名 `glimmer`），由 `apps/linux/fcitx5/addon` 的 CMake 安装规则生成 |
| `/usr/share/icons/hicolor/256x256/apps/glimmer.png` | 图标，`assets/icon/logo.png` 缩到 256 提交在 `apps/linux/packaging/`（改 logo 后重新缩放） |
| `/usr/share/doc/glimmer/copyright` | 许可（GPL-3.0-or-later，五笔码表 86 与 98 LGPL-3.0 / 新世纪 LGPL，数据来源） |

`postinst` / `postrm` 只刷新 `ibus write-cache --system` 并提示用户 `ibus restart` 或 `fcitx5 -r`，不杀用户会话里的 ibus-daemon / fcitx5。
`Depends` 是 `ibus (>= 1.5.20) | fcitx5 (>= 5.1)` 加 `dpkg-shlibdeps` **只从 IBus 引擎二进制**算出的动态库依赖。插件 `.so` 不进 `dpkg-shlibdeps`：
它在 24.04 上编，会带进 `libc6 (>= 2.39)` 与 `libfcitx5*`，逼 22.04 / Debian 12 的 IBus 用户装不上；它要的 `libstdc++` / `libfcitx5*` 由 `fcitx5 (>= 5.1)` 包带来，OpenSSL 与引擎共用。
IBus 部分支持 Ubuntu 22.04 / Debian 12 起，Fcitx5 部分要 5.1（Ubuntu 24.04 / Debian 13 起）。

**Fcitx5 插件的构建**（CI 的 `linux-fcitx5` job 与 `package.sh` 本机编时同一套命令，在 `cargo build --release --locked -p glimmer-fcitx5` 之后）：
`cmake -S apps/linux/fcitx5/addon -B <target>/fcitx5-build/<arch> -DCMAKE_BUILD_TYPE=Release -DCMAKE_INSTALL_PREFIX=/usr -DGLIMMER_RUST_LIB=<target>/release/libglimmer_fcitx5.a -DGLIMMER_DATA_ROOT=/usr/lib/glimmer`，
`cmake --build`，`DESTDIR=<staging> cmake --install`。构建依赖 `cmake g++ extra-cmake-modules libfcitx5core-dev libfcitx5config-dev libfcitx5utils-dev fcitx5-modules-dev`（CI 与 Docker 镜像都装）。
发版包必须两个框架都带：本机编插件时缺 cmake 或 Fcitx5 开发包，`package.sh` 失败退出；本地只打 IBus 包用 `GLIMMER_SKIP_FCITX5=1`（不支持交叉编译插件，`GLIMMER_TARGET` 时也要设它或给 `GLIMMER_FCITX5_STAGE`）。用户配置在 `~/.config/glimmer/config.toml`，数据与日志在 `~/.local/share/glimmer/`，卸载不动。

**`package.sh` 的环境变量**：
- `GLIMMER_IBUS_BIN`：现成的 `glimmer-ibus`，设了就不 `cargo build` 它（CI 用 22.04 上编的）。
- `GLIMMER_FCITX5_STAGE`：现成的插件安装目录（`DESTDIR=<目录> cmake --install` 的结果，里面是 `usr/…`），设了就整个拷进 staging，不编静态库也不跑 cmake。
- 两个都不设时本机一把编完，打一行警告：Depends 由构建机决定，发版包由 CI 分开编。`GLIMMER_SKIP_FCITX5=1` 不带插件（`GLIMMER_FCITX5_STAGE` 随之忽略）。
- 引擎二进制与插件 `.so` 都在 `package.sh` 里 `strip`（本机架构时）。

**本机打包**（macOS 上用 Docker）：`apps/linux/scripts/package-docker.sh` 在 `<基础镜像>` + rustup（版本读 `rust-toolchain.toml`）+ dpkg-dev 的镜像里跑 `package.sh`，
基础镜像的 `libfcitx5core-dev` ≥ 5.1 时再装 Fcitx5 构建依赖（改 apt 清单时改镜像名里的 `-deps<N>`）。基础镜像由 `GLIMMER_DOCKER_BASE` 选，缺省 `ubuntu:24.04`；
`ubuntu:22.04` 这类没有 Fcitx5 5.1 开发包的，要配 `GLIMMER_SKIP_FCITX5=1` 或 `GLIMMER_FCITX5_STAGE`，否则脚本在编译前就退出。
容器的 cargo target 放 `target/linux-docker/`（`ubuntu:24.04`）或 `target/linux-docker-<基础镜像>/`（如 `linux-docker-ubuntu22.04`，glibc 不同的产物不混用），registry 缓存放命名 volume，成品在 `target/deb/`。
`GLIMMER_IBUS_BIN` / `GLIMMER_FCITX5_STAGE` 要在仓库目录里（容器只挂仓库），脚本换成容器路径再传。缺省打本机架构（Apple Silicon 上是 arm64），
`GLIMMER_DOCKER_PLATFORM=linux/amd64` 打 amd64（走模拟，慢）。worktree 里没有产品数据时用 `GLIMMER_DATA_DIR=<主工作树>/data` 只读挂进去。
想在本机得到与发版包一样的 Depends：`GLIMMER_DOCKER_BASE=ubuntu:22.04 GLIMMER_SKIP_FCITX5=1 apps/linux/scripts/package-docker.sh`（只有 IBus），
或先在 24.04 上编出插件安装目录再用 `GLIMMER_FCITX5_STAGE` 交给 22.04 那次打包。
在 Linux 机器上直接跑 `apps/linux/scripts/package.sh`（要 `dpkg-dev`，本机编插件时另要上面的 Fcitx5 构建依赖）。和 `bundle.sh` 不同，它不从 TSV 重打 `.qj`，只装 `data/generated/` 里已有的；
没有 `dict.qj` 时只带样例词库并打警告。
装机端到端：`apps/linux/tests/docker/install.sh <deb>` 在干净的基础镜像（`GLIMMER_DOCKER_BASE`，缺省 `ubuntu:24.04`）里装 deb，先测 IBus（`e2e_router.py`），再装 fcitx5 测插件（`apps/linux/fcitx5/tests/docker/e2e.py` 的装机模式）；
`--ibus-only` 只测前者；基础镜像的 fcitx5 低于 5.1（`ubuntu:22.04`、`debian:12`）时插件那一遍自动跳过。

## 提交前检查与 CI

本地 `git config core.hooksPath .githooks` 启用一次后，每次提交前 `.githooks/pre-commit` 先拒绝装饰性分隔注释（`// ====` / `// ────`，只做视觉分组不带「为什么」），再跑 `cargo fmt --check` 与 `cargo clippy -D warnings`（含 IMK 外壳，增量几十秒）；
`.githooks/pre-push` 在推之前跑全 workspace 测试。外部 PR 走同一套 `ci.yml`，不过不合。

供应链：workflow 里的 actions 一律钉到 commit（注释写对应标签），`.github/dependabot.yml` 每周一提 Cargo 与 actions 的更新 PR；`audit.yml` 每周与 Cargo.lock 变动时跑 `cargo audit`；
cargo 命令全 `--locked`（含 `bundle.sh` 与 `build.ps1`）。普通 CI 只有 `contents: read`，checkout 不留凭据；release 的 secrets 不放顶层 env，只注入用它的那一步。

发版门禁（`release.yml` 第一步）：版本号与标签一致且不带 `-dev`；标签指向的提交必须在 `main` 上（`git merge-base --is-ancestor`）；产品数据下载后按 `data` Release 的 `SHA256SUMS` 校验，摘要写进 `build-info.json` 的 `data_sha256`。
**正式版前还欠**：产品数据改成不可变 tag 并在仓库里锁定版本（现在滚动覆盖，同一源码 tag 重跑可能拿到不同数据）、安装包内容验证（词库 / 模型 / 许可齐不齐、签名校验）。

## 两个 workflow

| 文件 | 触发 | 做什么 |
|---|---|---|
| `.github/workflows/ci.yml` | push main、PR | Linux 上 `cargo fmt --check` / clippy / test，排除 `glimmer-macos`（IMK 外壳只能在 macOS 编译，macOS runner 计费是 Linux 的 10 倍） |
| `.github/workflows/release.yml` | 推 `macos-v*` / `windows-v*` / `linux-v*` 标签 | `macos` job（`macos-26`）：下载产品数据 → 可选签名公证 → `bundle.sh --pkg` 打 arm64 与交叉编译的 x86_64 → 建 Release；`windows` job（`windows-latest`）：下载产品数据 → `build.ps1` 打 Inno Setup 安装包 → 建 Release；`linux-check` 门禁 → `linux-ibus`（`ubuntu-22.04` / `-arm` 编引擎）与 `linux-fcitx5`（`ubuntu-24.04` / `-arm` 编插件）→ `linux-package`（`ubuntu-22.04` / `-arm` 下载产品数据、用预编产物跑 `package.sh` 出 deb），`linux-release` 汇总建 Release。三者最后都跑 `publish-releases-json.sh` |

## 产品数据从哪来

词库、语言模型、释义表、三份五笔码表（`data/generated/*.qj`、`dicts/*.qj`、英文词表）不在 git 里，体积约 90 MB 且由本机数据管道生成。
它们发在仓库里一个个**不可变**的预发布 Release 上：`data-v1`、`data-v2`……每次数据重生成发一个新号、从不覆盖
（预发布不会成为 GitHub 的 latest，官网取 latest 时不会拿到它）。仓库里 `tools/release/data.lock` 钉住当前要用的标签与两个资产的 SHA-256，
跟用到新数据的代码同一个提交进去：checkout 哪个提交就拿到它对应的那版数据，离线自编译的人不会因为我们改了数据而编出坏包。

- `tools/release/data-bundle.sh`：把 `data/generated/` 打成 `glimmer-data.tar.gz`（`PRODUCT_FILES` 里 `wubi86.qj`、`wubi98.qj`、`wubixsj.qj` 三份都在，缺一份就打不出包），本地整句模型单文件 `data/model/model.qjm`
  （训练仓库导出三件套到 `data/model/`，`tools/release/pack-model.sh` 打成一个 `.qj` 容器，fp16 约 56 MB，元数据也写在那个脚本里）原样上传，
  连同 LLM 续跑中间产物 `glimmer-llm-intermediates.tar.gz` 发到下一个 `data-vN`（`--tag` 可指定，已存在就拒绝），然后改写 `data.lock`。
- `tools/release/data-fetch.sh`：按 `data.lock` 下载（有 gh 用 gh，没有就 curl 直连）、按锁文件里的哈希校验（不信 Release 自己那份 `SHA256SUMS`），
  数据包解到 `data/generated/`、`model.qjm` 放到 `data/model/`。`release.yml` 两个 job 和离线自编译走同一个脚本；
  `bundle.sh` 见到 `dict.qj` 就按产品数据打包、见到 `model.qjm` 就放进 `Resources/model/`，`glimmer.iss` 同理装进 `{app}\data\model`。
  标签与两个哈希记进 `build-info.json`（`data_tag` / `data_sha256` / `model_sha256`）。

数据重生成之后（重跑 lexicon / bigram / gloss-gen export）或模型重训之后跑一次 `data-bundle.sh`（三件套比 `.qjm` 新会自动重打），
把锁文件的改动提交（`chore(data): 数据 data-vN`），否则 CI 打的包还是锁文件指的旧数据。模型文件缺失或哈希不符时 CI 会失败，不会静默地发出错数据的包。
2026-09-16 之前用的是滚动覆盖的 `data` Release，已冻结不再更新。

## 签名与公证

没有证书时 CI 照样出包（ad-hoc 签名，Release 说明里自动加一句「首次打开要在隐私与安全性里放行」）。
Apple Developer 账号有了以后，在仓库 Secrets 里配齐 `release.yml` 头部注释列的七个值（.p12 与 .p8 都 base64），
下一次发版就是签名 + 公证 + 钉票据的包，用户下载双击即装。`bundle.sh` 本身通过 `GLIMMER_SIGN_IDENTITY` /
`GLIMMER_INSTALLER_IDENTITY` / `GLIMMER_NOTARY_PROFILE` 三个环境变量工作，本机有证书也能这样打。

## releases.json：官网下载页的数据源

`tools/release/releases_json.py` 从 `CHANGELOG.md`（日期、渠道、更新日志）、GitHub Releases API（附件、地址、大小）
与每次发布的 `SHA256SUMS` / `build-info.json`（每个包的 sha256、提交哈希、构建时间、工具链）生成，挂在每个版本的 Release 上；官网固定取
`https://github.com/<repo>/releases/latest/download/releases.json`（仓库私有期间要带令牌走 API 下载附件）。

结构对应官网 `src/lib/releases.ts` 里的 `Release` / `Asset` 类型：

```json
{
  "generated": "2026-09-07T12:00:00Z",
  "repository": "OpenFabrica/glimmer",
  "latest": "0.1.0",
  "releases": [
    {
      "version": "0.1.0",
      "date": "2026-09-07",
      "channel": "beta",
      "notes": ["整句输入：……", "候选旁有词性和译词……"],
      "commit": "869ad00…（40 位）",
      "built_at": "2026-09-07T08:38:12Z",
      "toolchain": "rustc 1.96.0 (ac68faa20 2026-05-25)",
      "assets": [
        { "platform": "macos", "arch": "Apple Silicon", "file": "Glimmer-0.1.0-arm64.pkg",
          "url": "https://github.com/OpenFabrica/glimmer/releases/download/v0.1.0/Glimmer-0.1.0-arm64.pkg",
          "size": 35989277, "sha256": "…" },
        { "platform": "macos", "arch": "Intel", "file": "Glimmer-0.1.0-x86_64.pkg", "url": "…", "size": 36172871, "sha256": "…" }
      ]
    }
  ]
}
```

- `releases` 从新到旧，`latest` 是第一条的版本号；官网「当前版本」取它，历史版本列表就是整个数组。
- `channel` 是 `alpha` / `beta` / `rc` / `stable`，显示成什么字由官网定；`commit` / `built_at` / `sha256` 给用户核对下载的包，下载页应显示 sha256 与提交短哈希。
- 平台与架构由文件名判定（`-arm64.pkg` → Apple Silicon，`-x86_64.pkg` → Intel，`-Setup.exe` → Windows x64，`-amd64.deb` → Linux x86_64，`-arm64.deb` → Linux ARM64），新的包型在脚本的 `ASSET_KINDS` 里加一行。
- `SHA256SUMS` 与 `releases.json` 自己不列进 `assets`。
- 官网侧要做的：构建时下载这个文件替代手写的 `releases` 数组（与拉 `docs/user` 的 `sync-docs.mjs` 同一处、同一个令牌），
  `downloadsOpen` 开关仍由官网自己控制。

## 本机打包

`apps/macos/scripts/bundle.sh --pkg` 打本机架构；`GLIMMER_TARGET=x86_64-apple-darwin` 交叉编译 Intel 包（要先 `rustup target add`，
本机不需要时不必装，CI 上两个都打）。成品在 `target/pkg/Glimmer-<版本>-<arch>.pkg`，每个架构一个工作目录，连着打互不覆盖。
