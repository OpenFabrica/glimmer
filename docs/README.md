| [notes/domain-words.md](notes/domain-words.md) | 领域词（2026-09-12）：从输入日志人工挑 48 条进基础词库的挑法、低频词当 token 统计为什么伤整句、两把尺子的前后数字 |
# docs

README.md 只介绍项目，所有技术内容放在这里，分四类：前三类是给开发者看的内部文档，`user/` 是给用户看的、官网渲染的文档。

| 目录 | 放什么 |
|---|---|
| [design/](design/) | 设计：现在是什么样、为什么这么定 |
| [plan/](plan/) | 计划：接下来做什么、按什么顺序 |
| [notes/](notes/) | 工程记录：性能优化史、事故复盘、踩坑，按时间顺序写，以后复习与面试用 |
| [user/](user/) | 用户文档：官网「文档」页的内容源，按目录结构渲染；维护约定见 [user/README.md](user/README.md) |

| 文件 | 内容 |
|---|---|
| [contributing.md](contributing.md) | 开发约定：架构约束短版、代码组织、版本号、提交信息、文档同步、提交前检查、发版与外部 PR 流程（CLAUDE.md 直接载入它） |
| [design/ai-dictionary.md](design/ai-dictionary.md) | AI 与软件开发词库：覆盖盘点、真实正文、统一开关、英文别名与混合词、独立验收 |
| [design/architecture.md](design/architecture.md) | Core 与平台层的划分、crate 结构、必须遵守的架构约束、`.qj` 数据容器 |
| [design/candidate-ui.md](design/candidate-ui.md) | 候选窗口、按键约定与翻译 annotation 的设计 |
| [design/landscape.md](design/landscape.md) | 同类项目（水杉、Rime）、可用数据源及其许可 |
| [design/rendering.md](design/rendering.md) | 跨平台 UI 与自绘渲染器（2026-09-13）：显示面与控件面的区分、Flutter / Avalonia / WebView 的调研结论、主题的两条路、tiny-skia + cosmic-text 的 spike 结果与两平台接入 |
| [plan/roadmap.md](plan/roadmap.md) | 分阶段路线图、各阶段的依赖关系与已完成项 |
| [plan/zh_tw_support_plan.md](plan/zh_tw_support_plan.md) | 繁体输出与台湾注音支持的分析与方案（贡献者 pinchiu，#22）：读音标准差异、台湾用语、简转繁一对多；两条路线 |
| [plan/wubi.md](plan/wubi.md) | 五笔支持方案（2026-09-16 起草，2026-09-17 补三版）：码表选型与许可（86 rime-wubi LGPL、98 yanhuacuo/98wubi 含词表 LGPL、新世纪 wubixinshiji）、按 fcitx5 / librime 缺省定的行为、复用 Dictionary 的架构与分步实施；三个版本都已落地，`z` 通配 / 历史造词 / 双拼反查在第二阶段 |
| [plan/todo.md](plan/todo.md) | 待办清单，按「从自用到能给别人用」排 |
| [plan/model-eval.md](plan/model-eval.md) | 模型假设与验证尺子：日志格式、各假设用什么指标与基线、英文输入与下一词预测的打字模拟（`--eval-english`）数字、已知的尺子偏差 |
| [notes/development-lexicon.md](notes/development-lexicon.md) | AI 与开发词库扩充的覆盖盘点、独立验收、性能及交付边界 |
| [notes/crate-notes.md](notes/crate-notes.md) | 各 crate / app / tool 的实现要点：入口类型、数据文件、常数、生成命令 |
| [notes/upstream-sync.md](notes/upstream-sync.md) | 同步上游 qingjian 的步骤（整树改名再 cherry-pick）、脚本用法、每次同步跳过了什么 |
| [notes/performance.md](notes/performance.md) | 历次性能优化：起因、定位方法、改法、数字前后对比与经验 |
| [notes/release.md](notes/release.md) | 发版流程：CHANGELOG、标签触发的 CI 打包、产品数据包、签名公证的 Secrets、官网用的 `releases.json` |
| [notes/phrase-layer.md](notes/phrase-layer.md) | 短语层（2026-09-12）：常用词表收不到的 我的 / 不知道 怎么从语料挖、怎么进语言模型而不伤整句、两把尺子的前后数字 |
| [notes/common-words.md](notes/common-words.md) | 补充常用词（2026-09-17）：取餐 / 弹出 / 播放器 这类为什么挖词漏掉、CC-CEDICT 与 jieba 白名单 × lm.qj 合成次数找缺口、人工审 1,357 条、不用语料的增量补词与回归数字 |
| [notes/internet-slang.md](notes/internet-slang.md) | 网络用语与聊天短语（2026-09-22）：候选表 752 条怎么并进词库、全量重跑 lexicon + bigram 两遍、含字母的词与 嗯 进不了词库的两个根因、冷查与整句回放的前后数字 |
| [notes/multi-reading.md](notes/multi-reading.md) | 一词多读音（2026-09-22）：一行 打不出的两个根因（反推标注每词一个读音、底值词频永远切不出来）、多读音标注与合成计数、主读音为什么沿用旧版、前后数字 |
| [notes/long-sentence.md](notes/long-sentence.md) | 长句准确率（2026-09-19）：按句长分桶的诊断、敲错边误伤与原样成词保护、整句接上文的权重怎么定、每词代价、从用户文本学个人 n-gram 的留出评测、试过不行的（束宽 / 每格候选 / 切分放开）与下一步 |
| [notes/constant-sweep.md](notes/constant-sweep.md) | 常数扫描记录：排序常数（2026-09-12，插值与敲错代价扫网格，全在平台区）、同输入串选择加分、个人二元绝对折扣、整句让整词与 emoji 让位、英文候选与下一词预测的常数（`--tune en-…`、`sweep.sh` 英文模式）；各节带复现步骤 |
| [notes/windows-win10.md](notes/windows-win10.md) | Windows 10 与设置程序（2026-09-13）：Reactor 早期绑定 Windows 11 才有的 AppModel API 导致加载期失败，改自包含部署 + 延迟加载 |
| [notes/chromium-imk.md](notes/chromium-imk.md) | Chromium 系浏览器（2026-09-15）：flagsChanged 送两遍让 Shift 单击失效、换焦点先 activate 后 deactivate 让旧会话拆掉全局状态 |
| [notes/imk-menu-null-action.md](notes/imk-menu-null-action.md) | IMK 输入法菜单（2026-09-16）：没有 action 的菜单项运行中一改显示状态，IMK 同步菜单时 `CFRelease(NULL)` 崩；所有项都绑选择器、状态行只改标题 |
| [notes/input-source-cache.md](notes/input-source-cache.md) | macOS 微信 / 企业微信切换崩溃：应用私有输入源缓存陈旧、隔离复现与可恢复的修复工具 |

约定：文档写中文，代码标识符一律英文。实现与文档产生分歧时以代码为准，并同步更新文档。
