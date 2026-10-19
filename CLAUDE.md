# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## 仓库现状

跨平台输入法，Core 平台无关，各平台只做壳。已发版 macOS 0.1.7（自用 + 测试者）、Windows 0.1.0-alpha.6（内测）、Linux 0.1.0-linux.2（内测，IBus 与 Fcitx5）。
阶段与已完成项见 `docs/plan/roadmap.md`，待办见 `docs/plan/todo.md`。

## 目录地图

一行一个，只说它是什么、入口在哪；实现要点（数据文件、常数、生成命令）在 `docs/notes/crate-notes.md`，改了实现要同步那里。

- `crates/glimmer-core`：引擎。`Engine` 是对外唯一门面，`Translator` / `Learner` 等 trait 在 `engine` 模块；拼音解析、纠错、候选、排序、整句、双拼、注音、英文模式都在这里。
- `crates/glimmer-dictionary`：词库（TSV 或 `.qj` mmap），按音节位置二分查询。
- `crates/glimmer-translate`：释义表 `Glossary`、词汇等级表 `LevelTable`。
- `crates/glimmer-learning`：用户侧落盘：词频 / 用户词 / 个人 n-gram / 敲错表（`FrequencyLearner`）、输入日志（`InputLog`）、输入统计（`UsageStats`）、词汇记录（`VocabularyBook`）。
- `crates/glimmer-predict`：云联想 `CloudPredictor`（OpenAI 兼容接口）与释义兜底 `CloudGlossFiller`；`PredictConfig` 是 `[predict]` 分节。
- `crates/glimmer-update`：应用内检查更新：拉发版清单 `releases.json`、`Version` 比版本、`Download` 下载并校验 sha256；`UpdateConfig` 是 `[update]` 分节，安装交给各壳。
- `crates/glimmer-lm`：整句转换的 bigram 语言模型 `BigramModel`。
- `crates/glimmer-neural`：字级 Transformer 本地推理 `CharScorer`（candle），给整句前几条路径重打分。
- `crates/glimmer-format`：`.qj` 数据容器（mmap 读、零拷贝视图、写入器、哈希索引）。
- `crates/glimmer-server`：输入法 Server 的平台无关部分：Engine 装配（`assembly`）与协议分派 `Router`（协议消息 → Engine → 帧），Windows Server 与 Linux 引擎进程共用。
- `crates/glimmer-platform`：平台层共用：`Config`（TOML 配置）、`extra_dictionaries`、Windows Server ↔ DLL 的 `protocol` 类型。
- `apps/cli`：Core 的验证工具：查询、逐键计时、输入日志回放、整句评测、常数扫描。排序 / 整句 / 纠错的改动先跑它再合。
- `apps/macos`：IMK 壳，按 `app / host / imk / candidates / menubar / preferences` 分目录；`scripts/bundle.sh --install` 装到本机，`--pkg` 出分发包。
- `apps/windows`：`server`（Server 进程：命名管道 + 自绘候选窗与状态条，Engine 与分派来自 `glimmer-server`）+ `tsf`（TSF DLL）+ `settings`（WinUI 3）+ `installer`（Inno）。DLL 不能带 Engine 的依赖树，所以是两个 package。
- `apps/linux`：IBus 引擎进程 `glimmer-ibus`（package `glimmer-linux`，D-Bus）；`fcitx5/` 是 Fcitx5 插件（Rust 静态库 `glimmer-fcitx5` 包 C 接口 + CMake 的 C++ 薄壳 `glimmer.so`）；`packaging/` 是 deb 的组件描述 / control / 维护脚本，`scripts/package.sh` 在 Linux 上出 deb，macOS 本机用 `scripts/package-docker.sh`。
- `tools/dict-convert`、`tools/gloss-gen`、`tools/corpus`：产品数据生成（词库 / 语言模型 / 释义表 / emoji / 英文词表），输出到 `data/generated/`（gitignore）。
- `tools/eval/sweep.sh`：回放扫参（多组 `--tune` 并行跑 `glimmer-cli --replay` 汇成 TSV），用法见 `docs/notes/constant-sweep.md`。
- `tools/release/`：发版脚本（一键发版 `release.sh`、macOS 本机签名公证 `macos-local.sh`、官网部署 `deploy-website.sh`、数据包上传 `data-bundle.sh`、模型单文件 `pack-model.sh`、`releases.json`、官网触发），流程见 `docs/notes/release.md`。
- `data/`：产品数据，整个 gitignore。`data/generated/` 由 tools 生成或从 GitHub 的 `data` Release 下载，`data/model/` 是本地整句模型；没有它引擎跑不起来。
- `assets/`：随包数据源与样例，各目录有 README 写来源与许可。雾凇拼音已彻底移除，不要再引入，选词时也不拿它当筛子或参照（原因是来源含华宇闭源词库，见 `docs/design/landscape.md`「许可策略」）。

`docs/` 分四类（索引在 `docs/README.md`）：`design/` 设计与决定、`plan/` 路线与待办、`notes/` 工程记录（性能、复盘、踩坑、crate 实现要点）、
`user/` 用户文档（官网构建时拉取渲染，约定见 `docs/user/README.md`，措辞面向用户、不出现实现词）。

## 常用命令

```bash
cargo build                                   # 整个 workspace
cargo test                                    # 全部测试；-p <crate> 单个，加测试名过滤
cargo test -p glimmer-core sentence           # 例：只跑 glimmer-core 里名字含 sentence 的测试
cargo clippy --all-targets -- -D warnings
cargo fmt --all
cargo run -p glimmer-cli -- <拼音>...          # Core 的主要验证方式；--replay / --eval-text 见 crate-notes
apps/macos/scripts/bundle.sh --install        # mac 壳装到 ~/Library/Input Methods/（IMK 不能 cargo run 验证）
```

钩子只需启用一次：`git config core.hooksPath .githooks`（pre-commit 跑 fmt + clippy，pre-push 跑全量测试）。
环境变量见 `.env.example`（`RUST_LOG`、`GLIMMER_LOG_DIR`、`GLIMMER_LEARNING_LANGUAGE`），复制成 `.env` 即生效。

Windows 三个 package 是 `glimmer-windows-server` / `glimmer-windows-tsf` / `glimmer-windows-settings`；本机只 `cargo check --target x86_64-pc-windows-gnu`，真编译与真机测试在 Windows 机器上做（部署方式见 `apps/windows/README.md`）；
端到端验证 mac 可用 `osascript` 往 TextEdit 发按键再读回文本。

## 约定

架构约束、代码组织、版本号、提交信息、文档同步、提交前检查与发版都在 `docs/contributing.md`，随本文件一起载入：

@docs/contributing.md

交流用中文。
