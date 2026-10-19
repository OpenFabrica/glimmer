# TODO

按「从自用到能给别人用」排。勾掉的移到 [roadmap.md](roadmap.md) 对应阶段。`★` 是当前建议的优先项。
2026-09-18 状态：macOS 0.1.7 / Windows 0.1.0-alpha.6 / Linux 0.1.0-linux.2 已发（GitHub Release + 官网 glimmerinput.app），产品数据锁到 data-v3。
核心输入、翻译（中→英带音标 / 日 24.9 万词、英→中 4.5 万词、注假名、修饰键上屏译词、翻译选中文字）、英文模式候选、双拼五套 + 五笔三版、繁体输出、
自建词库（基础 + 随包领域词库，含语料挖出的高频词与补充常用词）、多词库与导入 / 移除 / 开关、偏好设置、`.qj` 容器（启动 50 ms）、
个人 n-gram（二元 + 三元）、词图内敲错边、本地整句模型重排、崩溃不丢、pkg 安装器、签名公证、README 安装 / 隐私 / 许可都已完成并在自用。
给外人用还差：首次运行体验、特殊应用验证、Windows 五项真机验证（见「三、其他平台」）。

## 〇、上线前必须做（不做就不能给别人）

- [ ] 词库质量（产品词库已自建，见 roadmap）：常用词表只有 5.6 万，语料挖出的 1.57 万高频词已并入（蚌埠 / 台州 / 高铁 / 吐槽 这类补上了），挖词漏掉的补充常用词 1,357 条已并入（取餐 / 弹出 / 播放器，2026-09-17），第二轮放宽门槛加叠字与人工清单又并入 11,503 条（不齐 / 线缆 / 呵呵 / 上一页 / 会员价，2026-10-19，`notes/common-words.md`），
  网络用语目录还空着（`assets/lexicon/04_internet_slang`）；LLM 标注的多音字读音抽查；挖词过滤已进 `dict-convert mine`（2026-09-07，`oov_filter.rs`，与原脚本输出逐行一致）；
  LM 表（TSV 48 MB / `lm.qj` 29 MB）不进 git，先按发布附件对待
- [ ] ★ 首次运行体验：没有 `data/generated/` 时打进包里的样例词库只有几十条，等于不能用；随包数据（词库、LM 表、释义表、emoji）
  要有一套发布版并定好体积（现在 `dict.qj` 3 MB + 领域词库 7 MB + `lm.qj` 29 MB + 释义表 `.qj` 39 MB + 英文词表 2 MB，pkg 34 MB；可再考虑变长编码）；
  发布物 = pkg（`bundle.sh --pkg`），要么 CI 生成数据、要么把 `data/generated/` 当发布附件
- [ ] 仓库瘦身（2026-09-07 决定：主仓库继续单库、官网独立仓库，不拆代码；要处理的是数据）。
  已做：LLM 中间产物 `gloss-llm.jsonl`（30 MB）/ `gloss-en-llm.jsonl` / `pinyin-llm.jsonl` 移到 `data/generated/`（gitignore），仓库只留导出的 `glossary-*.tsv` 与词库源；
  历史在同日压成 `v0.1.0` 单提交（完整开发历史在本地分支 `archive/dev-history`，不推），所以公开仓库里从没出现过这些文件。
  还差：仓库推到 GitHub 后跑一次 `tools/release/data-bundle.sh`，把这三个 JSONL 与 `data/generated/*.qj` 传到 `data` 预发布 Release（现在只在本机与归档分支里有）；
  正式开源后若 clone 太慢，再考虑把 `assets/lexicon` + `assets/glossary` 拆成数据仓库，现在不做
- [x] 签名与公证（2026-09-17 收尾）：pkg 安装器、自动注册输入源、版本号 / 构建号（2026-09-06，`bundle.sh --pkg`）；CI 推标签出包、建 Release、生成 `releases.json`（2026-09-07）；
  Developer ID 签名 + 公证 + 钉票据在本机做（`tools/release/macos-local.sh`，凭据在 `.env`，CI 的 macOS 无签名构建由 `release.sh` 取消）；
  官网读 `releases.json`，发版后用 `deploy-website.sh` 手动部署（CI 没配 `GLIMMER_WEB_TOKEN`，配上就自动触发）。流程见 `docs/notes/release.md`
- [x] 应用内检查更新（2026-09-18）：`crates/glimmer-update` 读 GitHub latest 的 `releases.json` 比版本号，macOS「关于」页 / 菜单与 Windows 设置程序都有「检查更新」「下载并安装」（下载校验 sha256 后拉起安装器），
  `[update] check` 自动查（macOS 启动 30 秒后 + 每 12 小时，Windows 打开设置时，12 小时内不重复）（上游 #159 也在要）。还差：Windows Server 侧没有提示（只有打开设置程序才查）；Linux deb 不做应用内更新；两端界面待真机验；
  两份同 ID 并存（`~/Library` 开发版 + `/Library` pkg）时系统拉不起输入法、选了就退回上一个（2026-09-18 装 0.1.8 踩到，注销重登才恢复）——已约束：`bundle.sh --install` 跟着现有位置覆盖，postinstall 挪走 `~/Library` 的开发版
- [ ] ★ 特殊应用逐个验证（2026-09-07 用户已验：Zed、Terminal 正常；JetBrains 系没装没验）：Secure Input（已有检测，验证密码框不组句、不发云端）、iTerm / Warp、VS Code / Electron、
  浏览器地址栏、沙盒应用（App Store 版备忘录 / 微信）、全屏游戏；每个记录 preedit 模式建议（按应用的开关已有 `[apps]` 分节，
  英文候选按应用关已做，preedit 模式按应用定可以接在同一分节）
- [ ] 登录后不恢复成微明：2026-09-06 两次开机日志都显示登录后当前输入源是系统拼音 / ABC，要手动切回（进程没崩过，panic 与崩溃报告都是零；
  Caps Lock 切 ABC 的系统开关已排除）。怀疑装在 `~/Library/Input Methods` 的 ad-hoc 包每次重装签名变、登录早期枚举没认上；等签名后装 `/Library` 再看，
  或在 `--register` 之外登录时补一次 `TISSelectInputSource`
- [ ] 卸载与重置：卸载脚本已随包（`uninstall.sh`，`--purge` 连数据删，README 有说明，2026-09-06）；还差偏好设置里的「重置学习数据」按钮、
  配置文件版本迁移（新增键有缺省，改名键要迁）
- [x] 许可证定稿（2026-09-07）：GPL-3.0-or-later，`LICENSE` 全文、`Cargo.toml` `license`、「关于」页、README 一致；名字与 logo 不授权。以后引入 JMdict 要补进署名清单


## 一、日常输入体验（决定自己会不会一直用）

- [ ] 整句转换收尾：语料再补（现在是维基 + LCCC 微博对话，新闻 / 小说体还缺；bigram 只留次数 ≥ 6 的前 300 万对，
  `wxqcf` 出 我想去蹭饭 就是「我想 → 去蹭饭」8 次进了表、「我想去 → 吃饭」没进的稀疏问题，简拼把读音放开后这类问题更显眼）；
  个人 n-gram 已做（二元 + 三元），「按应用分别记」看用起来的效果再说
- [ ] ★ 长句准确率（诊断与已做的两步见 `docs/notes/long-sentence.md`，2026-09-19）：句子越长整句全对越难（docs 集首选 40% → 8%），字准确率一直 70–77%。
  已做：原样成词保护、整句接上文、每词代价、`Engine::learn_text`（从用户文本学个人 n-gram，留出评测首选 23% → 61%）、评测按句长分桶 / 分段输入 / 注错集。还差，按值排：
  ⓪ 「从文件学习」macOS 已有入口（「高级」页，2026-09-19，待真机验：对话框多选 / 选文件夹、状态行文案、学完整句有没有变）；还差 Windows 设置程序与 Linux 的入口、
  可撤（记一份转移清单按 `unrecord_transition` 退）、同一个文件学两遍会记双份（要按文件哈希记已学过的）；
  ① ~~真 k-best + 两轮重排~~ 已做（全集开模型 32.1% → 42.0%，11–20 字 12% → 24%）；待真机验：mac 第二轮的分晚一拍到时画面跳不跳、Windows CPU 上 24 条一批的耗时（超了把 `RESCORE_PATHS` 降到 12）；
  ② 长句里的敲错召回——注错集 11 字以上接近 0（整段一处编辑只到 `correction::MAX_LETTERS` 24，词图敲错边只管「敲错后仍是合法音节」的）；
  ③ ~~语言模型：三元用回指近似、固定 λ 换成回退平滑~~ 2026-09-22 已做（`docs/notes/language-model.md`）：随包 lm.qj 当初只统计了两成维基，全量语料 bigram 27.9% → 28.8%，
  三元 + Kneser-Ney 式回退再到 29.2%（16–20 字 7.6% → 11.5%），回放词 +6；验收只过一半（6 → 12 仍掉 0.3，放到 24 才不掉）。
  还差：**定随包发哪份模型数据**（全量 bigram 48 MB / 三元 149 MB，都在主树 `data/lm-*`，`data/generated/lm.qj` 还是旧的）并走 `tools/release/data-bundle.sh`；
  语料补技术 / 新闻 / 小说；领域词库只剩 `.qj` 没有 TSV，`dict-convert bigram` 分词读不到（词表 10.1 万，比随包模型少 4 万词）；三元没给短语层合成计数；
  ④ ~~回退换成按前词定~~ 随 ③ 一起做了（接续概率是三元涨分的前提）；⑤ 100 键以上每键 parse 到 10 ms（切分 DP 随长度涨），要增量化；
  ⑥ ~~整句读应用光标前文~~ 2026-09-22 已做（`Engine::set_surrounding_before`，链空时切前文末两词；壳不开本地模型也送；`--eval-context app`），真机已验；
  ⑦ ~~多音字按读音份额扣分~~ 2026-09-22 已做（`sentence::ReadingShare`，没(mo) 扣 4.85，干净集 29.1% → 29.7%、回放词 8857 → 8880；回放整句 −20 条都是用户真用冷门读音打的字：
  个人 n-gram 记词不记读音，整句侧没有选择加分可救，要根治得学习侧按读音记）；
  ⑧ ~~网络用语 / 聊天短语~~ 2026-09-22 已并入 752 条并全量重跑词库 + 三元模型（`notes/internet-slang.md`）：词库 93,276 → 105,694 词形（11,712 条领域词升进基础词库），
  12 本领域词库 TSV 回来、分词词表 23 万，`lm.qj` 88 MB；干净集 29.7% / 72.8%、回放词 8885、候选表冷查首选 353 → 548。**待发 data-v5**（`pinyin-llm.jsonl` 是从已发词库反推的，随包一起发）。
  还差：摆烂 / 避雷 / 睡会儿 语料里一次没出现、翻不过同音整句候选，要补现代对话语料；混杂词（PUA / KPI / TODO）输入码就是英文单词、被英文候选压着；`P0` 这类含数字的输入码在壳里没验；
  **个人数据按字不按读音**（用户词频、选择记录、个人 n-gram）：句首 `没` 记了 32 次，`mohuiyu` 带个人数据仍出 没会鱼（冷启动已是 摸会鱼），与 ⑦ 回放整句掉的 20 条同根，学习侧要按读音记；
  同日还做了：选择次数按句首 / 句中分桶 + 改软加分（`CHOICE_BONUS` 1.5）、个人二元绝对折扣（D=2、记录刻度 ×2，一次普通事件不翻首选；留出学文本 73.2% → 68.3% 是设计代价），数字见 `notes/constant-sweep.md`
- [ ] 按场景切词库分组（2026-09-22 提出）：把随包领域词库、导入词库、网络用语表这些编成「场景」（词库组合），按场景查词与排序，提高各场景下的准确率。
  场景能自动切也能手动切：自动按前台应用定（终端 / IDE 用开发场景、聊天软件用聊天场景，接在已有的 `[apps]` 分节上，与按应用关英文候选 / preedit 同一处配置），
  手动在菜单栏 / 状态条切并可为某个应用记住；为降低门槛随包给一个「通用」场景（默认全开或按使用面挑一组），用户不配也能用。
  与现有的领域词库开关（`[general] domains`、`extra_dictionaries`）合并成一套，不做两层开关；场景切换不能让词图缓存 / 个人数据错位（`SpanCache` 按词库集合失效）。
  评测：回放按 `commit.app` 分应用算首选命中（`model-eval.md`「按应用风格」），先量各应用现在的数字再定分组
- [ ] 释义兜底收尾：已做（2026-09-06，只对上屏的词库词 / 云端词查，个人释义表 `user-glossary-<语言>.tsv`）；还可以考虑停顿后高亮的候选也查、
  偏好设置里看 / 清个人释义表、导入词库时批量预填
- [ ] 候选窗口里超长释义要截断（CEDICT 表时 你 那条能拉到整屏宽；LLM 表已短，仍要兜底）
- [ ] 翻译选中文字：译文很长时候选窗口的折行（读不到选区的提示已做）
- [ ] 英文模式候选：把数据包的误拼对照表（typos，10 万对）喂进纠正，技术词（kubectl）没有 wordfreq 词频要给底值，两处编辑的纠正
- [ ] 全角 / 半角切换，`。` 与 `．`，`-` `=` 非组句时的行为
- [ ] 快捷短语（`i` 前缀）：编码 → 短语表，用户可增删
- [ ] 问字模式离线版：拆字表（IDS 数据）按部件查字，云端版已有
- [ ] 双拼收尾：菜单栏增加方案切换
- [ ] 五笔第二阶段（第一阶段 2026-09-16 已进 Core，`docs/plan/wubi.md`）：~~98 码表~~ 2026-09-17 已做，连新世纪一起（`[general] wubi = "86"|"98"|"xsj"`，三份码表随包，学习数据按版本分目录）；
  ~~五笔与拼音混输~~ 2026-09-18 已做（配置拆成 `[general] scheme` 拼音侧 + `wubi` 形码侧两条轴，`Engine::set_phonetic`，候选顺序 全码五笔 → 拼音 → 编码前缀五笔；
  混输下没有 `z` 反查与四码自动上屏）——两端设置界面还要跟上（macOS 偏好设置与 Windows 设置程序的「五笔」菜单要改成两个独立的选择）；
  ~~五笔整句~~ 2026-09-19 已做（issue #3，`[wubi] sentence` 缺省关，两端设置界面各一个开关，Windows 要真机验）；
  `z` 在中间位当通配符；对已上屏历史造词（`encode_commit_history`，现在只对同一缓冲区连着上屏的词造）；反查支持双拼；
  CLI `--typing` 不模拟四码自动上屏（逐键计时时四码之后还留在缓冲区，与壳的行为不同）；增广字集（全部字）要另打一份 `--extended` 码表并给切换入口
- [ ] 双拼方案数据化：加方案不用改代码（放一张键位表就能用，与 #51 的方案导入一起定）；解码支持同码出多个读音——小浪的 `lk`（lai / lia）、`nm`（niang / nen）、`dk`（dai / dia）、`un`（en / eng）现在只解第一个，做完去掉 `every_syllable_round_trips` 里小浪的四个例外（上游 #129）
- [ ] 敲错边收尾：敲错四类代价与整段纠错代价 2026-09-12 已在冻结日志（12886 条可评）上扫过，都在峰上，不改（`docs/notes/constant-sweep.md`）；
  个人折扣上限（3）回放从零学分不出好坏，等 `user-typos.tsv` 能带进回放（`--user-dict`）再看；
  退格重打（组句内、跨上屏）已进输入日志 `retype` 事件（2026-09-12），攒够后先按 `docs/plan/model-eval.md` 的尺子算召回与误纠率，再决定喂不喂个人敲错表；
  整段一处编辑那路仍是逐变体跑整句，长句慢（`nihoama` 1 ms、更长的几毫秒）
- [x] 个人 n-gram 插值常数（λ 0.8 / K 8 / 上限 0.5 / D 0.75）2026-09-12 回放扫过：K 全平，λ 0.7–0.75 与 0.8 差 0.3 个点、带神经重排后无差，封顶 0.7 起掉；保留现值（`docs/notes/constant-sweep.md`）。
  「挤掉了」要选四次才翻过「给掉了」是 K 与封顶合起来的设计（一次误选翻不过），不是常数没调
- [x] 词库缺高频「代词 + 的」（我的 / 你的 / 他的）：2026-09-12 加了短语层 `assets/lexicon/phrases.tsv`（`dict-convert phrases`，语料相邻两三词、对话语料里 ≥ 2000 次、
  边界规则，5400 条：我的 / 好的 / 不知道 / 有没有 / 那我），品牌词 `brand.tsv`（微明 210）；词典词头收不到的这一层以后按同一方法补
- [x] 语料里少的领域词（对齐 / 后端 / 词库 / 候选框）：2026-09-12 从日志人工挑 48 条进 `assets/lexicon/domain_words.tsv`，语言模型走合成计数（`docs/notes/domain-words.md`）；
  语料 0 次的（微软拼音 / 悬浮条）按规矩没进，要进得另立白名单
- [ ] 按输入串记的选择只认字面：`wod` 下选的 我的 惠及不到 `wode`；考虑同时按候选全拼记一份、查询取两者最大
- [ ] 已经学进用户词的错读音云端词（`我的 wo di`、`我的哦 wo di e` 这类）没有清理入口：偏好设置词库页给「按读音核对用户词」，或一次性脚本
- [ ] 正式版前的发布可信性（2026-09-12 外部 CI 检查，测试版先不做）：产品数据改不可变 tag 并在仓库锁版本 + SHA（2026-09-16 已做：`data-vN` Release + `tools/release/data.lock` + `data-fetch.sh`）；
  安装包内容验证（pkg / Setup.exe 里词库、模型、许可齐不齐，`codesign --verify` / `signtool verify`）；`cargo deny`（许可证 + 来源）；`.qj` 读取器越界 fuzz、`glimmer-format` 跑 Miri
- [ ] 本地整句模型（已进壳并随包发出，见 `docs/notes/neural-rescoring.md`；加载 12 秒是早期首次 Metal 编译的记录，2026-09-12 装机实测 102 ms，划掉）：
  重排改了切分时应用里的行内拼音要到下一键才更新；日语
- [ ] 个人微调（LoRA 挂在冻结基模上，基模更新后拿本地日志重训）：先在 train 仓库验证收益，再验 candle CPU 训练能不能跑通，都过了才进壳
  （闲时门禁、加载时合并权重、开关与一键清除）；每个模型假设先按 `docs/plan/model-eval.md` 写尺子与基线
- [ ] 日语 emoji 要等有日语输入模式

## 二、产品特色

- [ ] 释义表收尾：覆盖面再扩（次数阈值降到 100 约 10 万词）；JMdict 校对假名与日文词性；等级表（CEFR / JLPT 已随包，见 `assets/levels/`）
  也可做英→中、日→中方向的种子；四六级 / 商务英语词表要先找到许可干净的来源
- [ ] 可选复习（Phase 4）：输入统计、生词识别、词汇统计（含 CEFR / JLPT 等级分布）已做（2026-09-06）；复习容易变成打扰，先不急
- [ ] 云联想收尾（Phase 6）：密钥进钥匙串（等签名定了再做，ad-hoc 签名每次重装都弹授权）、按应用禁用、限流与用量统计
- [ ] 个人模型（Phase 7）：小 Transformer 实验（有评测门槛），见 roadmap

- [ ] ★ **主题与自绘渲染器 spike**（2026-09-13 定向，分支 `renderer-spike`，见 [design/rendering.md](../design/rendering.md)）：
  tiny-skia + cosmic-text 画一行「微明 hello 🙂 日本語」+ 圆角阴影，Windows / macOS 与原生并排截图，验四条：彩色 emoji（sbix / COLRv0）、
  中日字形回退按 locale、字体按需加载（不扫全系统）、灰度抗锯齿观感；首帧耗时与内存不劣于 GDI / AppKit。过了 Windows + macOS 一起换渲染器、主题文件 TOML；
  不过退回各平台各自渲染（Windows 走 D2D）。设置程序不自绘。

## 三、其他平台

- [~] Windows TSF（Phase 5）：真机自用中，候选窗已覆盖商店 / 任务栏搜索（`uiAccess` + 自签）。
  与 mac 功能齐平尚缺（按价值排；工时=开发+真机验证合计，单人；真机来回是大头）：
  - [~] **① 翻译选中文字 `translate_selection`**（★★★ / 中高 / 1.5–2 天）：**代码完成，待真机测**（2026-09-10）。
    新增 `RequestSelection` / `Selection{text,rect}` 两协议消息；DLL 异步只读会话（`com/selection.rs`，`GetSelection`+`GetText`+`GetTextExt`）
    读选区 → Server `request_translation`（`PredictionKind::Translate`）→ 自绘候选窗显示译文 → 回车 / 空格替换（回 `commit` 走 `InsertTextAtSelection` 替换选区）/ Esc 保留；
    设置页「快捷键」加了翻译选中文字行（改修饰键）。mac 全绿 + windows-gnu 交叉编译 / clippy 过；沉浸式应用读选区、替换选区待真机验。
  - [~] **② 生词「看到轮次」计数 `note_displayed`**（★★☆ / 低 / 0.5 天）：**代码完成，待真机测**（2026-09-10）。
    `dispatch/mod.rs::reconcile_candidates` 里真正 show 那次按当前页调 `Engine::note_displayed`、收窗传空，对齐 macOS `render`；轮次推进、生词满 3 轮毕业。
  - [~] **③ 悬浮状态条**（★★☆ / 中 / 1.5–2 天）：**代码完成，待真机测**（2026-09-10）。
    Server 侧单例，跟候选窗同一条 UI 线程；分层窗合成器从候选窗 `surface.rs` 抽出成 `ui/layered.rs` 两边共用，绘制件（主题 / DPI / 深浅）复用；
    状态条自己一个窗口类 + 窗口过程（`ui/status/`）：`WM_NCHITTEST`→`HTCAPTION` 整块可拖、`WM_MOUSEACTIVATE`→`MA_NOACTIVATE` 不抢焦点、
    `WM_EXITSIZEMOVE` 把位置写回 `[status_bar] x/y`（`GWLP_USERDATA` 存自身指针）。新增协议 `ClientMessage::ModeChanged{english}`（fire-and-forget），
    DLL 在 `update_mode_indicator` 推模式；Server 据此刷、会话关就收起；双拼方案 Server 从 `[general] shuangpin` 知道。配置加 `[status_bar]` 分节（`enabled`/`x`/`y`），
    设置「候选窗口」页加开关。参考水杉 FTB 形态（它用 D2D 且不记位置），我们沿用 GDI 分层窗保持视觉一致并加了记位置。
    mac 全绿（+3 状态条分派测试）+ windows-gnu 交叉编译 / clippy 过；拖动 / 记位置 / DPI / 深色 / 盖高 z-band 待真机验。
  - [~] **④ 删候选屏幕提示**（★☆☆ / 低 / 0.5 天）：**代码完成，待真机测**（2026-09-11）。
    `Frame` 加 `notice: Option<String>`（不参与 `is_empty`），`dispatch/shortcut.rs::forget_on_page` 填、`handle_key` 开头清（只活到下一次按键）；
    自绘候选窗 `ui/candidates/view.rs` 画在拼音行下方（淡色小字）。host 测已加。
  - [~] **⑤ 任务栏点中/英反同步**（★☆☆ / 低 / 0.5 天）：**代码完成，待真机测**（2026-09-11）。
    激活时对转换模式 compartment 挂 `ITfCompartmentEventSink`（`com/conversion.rs`），`OnChange` 读回 `NATIVE` 位、与当前模式不同才翻转
    （防回环），顺带刷指示器 + 上报 Server 让悬浮状态条也同步。纯 DLL 改动、无新协议。
  - [ ] 发版：换 **Certum 开源代码签名证书**重签（开发全程自签 + 本机受信任根，见 `installer/sign-local.ps1`）、
    `windows-v<版本>` 标签与 CI。
  - [ ] **本地整句模型上 Windows**：Server 已接（`dispatch/rescore/`，CPU 推理，设置「云服务」页有开关，安装包带 `data\model`），待真机验：每次重排的耗时（前文 + 几条路径一次前向，CPU 上可能几十到一百多毫秒，超了就缩前文长度）、模型加载时间；
    应用光标前文已接（DLL 起组句时读、密码框跳过；2026-09-12 真机验过记事本 / Edge / 终端都读得到，Edge 密码框按 `IS_PRIVATE` 识别），真机看沉浸式应用读不读得到；
    模型单文件 `.qjm` 已做（2026-09-12，复用 `.qj` 容器 `Kind::Model`，`find_model` 先 `.qjm` 再三件套目录，`pack model` / `tools/release/pack-model.sh`，
    data Release 传 `model.qjm`，bundle.sh / glimmer.iss 只带一个文件），待 mac 与 box 真机各装一次验加载与重排；
    密码框已按 TSF 规范做（2026-09-12）：`KEYBOARD_DISABLED` compartment 整键放行不组句，`IS_PRIVATE` / 密码 / PIN 输入范围为私密（组句但不学不记不发云端，`ClientMessage::Privacy` → `Engine::set_private`），box 真机验过：Edge 密码框整键放行；InPrivate 网页文本框报 `IS_SEARCH` 不报 `IS_PRIVATE`，私密路径只靠单测覆盖；CI 两个 job 都从 `data` Release 取 `model.qjm`（已做）。
- [ ] Linux（Phase 5，IBus 版已做、未上真机）：GNOME / KDE 真机验；翻译选中文字（用 surrounding text 的选区）；面板上点第 10 个以后的候选；
  失焦时落进应用的是屏上 preedit（带音节分隔）而不是原文；全角 / 半角与设置界面；Fcitx5 插件真机验（KDE Plasma）
- [ ] 配置同步、跨平台词库
