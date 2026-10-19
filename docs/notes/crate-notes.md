# 各 crate 的实现要点

CLAUDE.md 只保留目录地图与规则，每个 crate / app / tool 的实现细节收在这里：入口类型、数据文件、常数、生成命令。
改了实现要同步改这里；与代码冲突时以代码为准。

## crates/glimmer-dictionary

词库（TSV 解析或 `.qj` mmap），键按字节序排好，查询逐音节位置二分收窄（简拼位置按音节块跳扫），
`lookup_pattern`（≥ 模式长度）与 `lookup_exact`（正好等长）同一套实现。词库键以 `v` 表示 ü，
TSV 解析、查询与生成工具把 `lue` / `nue` 统一成 `lve` / `nve`。
旧 `.qj` 含这些键时，加载器建立规范化的内存词库。新 `.qj` 继续使用 mmap。

## crates/glimmer-core

模块：`composition` / `parser` / `correction`（拼写纠错：整段一处编辑的候选纠正 + `typo` 音节级敲错变体表，后者进整句词图当带代价的边）/
`candidate` / `ranking` / `shortcut` / `sentence` / `fuzzy` / `shuangpin`（双拼：四套方案键位表、键 → 全拼解码与消耗换算）/ `zhuyin`（大千注音：键 → 注音符号 → 拼音，`[general] zhuyin` 开关，声调只判音节完整不进查询）/
`wubi`（五笔码表方案，`Engine::set_wubi(Option<Scheme>)`；与拼音侧 `Engine::set_phonetic(bool)` 是**两条独立的轴**——只用形码（拼音侧关）时双拼 / 注音 / 整句 / 纠错 / 模糊音 / 中英混输 / 快捷候选 / 组句联想全部让位，`v` `u` 是编码键，只剩 `?` 问字；两边都开是混输，见下；
  `Variant`（`Wubi86` / `Wubi98` / `Xinshiji`，`variant.rs`）是版本：配置写法 `86` / `98` / `xsj`（新世纪另认 `06` / `xinshiji`）、方案键 `wubi86` / `wubi98` / `wubixsj`、
  码表文件 `wubi86.qj` / `wubi98.qj` / `wubixsj.qj`、界面名；三版行为完全一致，只有字根与码表不同；
  `Scheme` = `Variant` + 码表 `Dictionary`（`词\t编码\t词频`，编码整个是一个音节）+ `Reverse` 字 → 全码（单字取最长码、等长取高频）+ `Options`（`[wubi]`）+ `encode` 造词规则；
  查询在 `engine/query/wubi/`（`mod.rs` 查码表与反查、`auto_commit.rs` 自动上屏、`sentence.rs` 整句）：`lookup_pattern` 前缀一次查出，全码命中在前、前缀命中短码在前当逐键提示（`hint` 开着 `reading` 注完整编码），作用域 ≤ `fixed_order_length`（缺省 2）的全码命中只按码表静态词频、不叠用户权重与选择记录；
  `WubiHit::key` 的末项是**码表存放顺序**（`assemble` 已按词频降序、平手保 TSV 原序排好）而不是文本序：补出来的名次词频归一化后 log 概率差在千分位以下、取整会打平，按文本序排会把冷词顶到前面；
  `check_wubi_auto_commit` 在 `push` 之后置 `pending_auto_commit`（四码全码命中且 `auto_select`；新键接上后无任何命中 → 旧段首选顶字、新键存 `deferred_key` 等 `commit` 后补回；满四码空码再敲字母整段丢掉），
  壳每键 `take_auto_commit` 取到就走普通 `commit`；`z` 开头走 `query_pinyin` 反查，候选 `reading` 注 `code_of`、`syllables` 是整段作用域，`typed_display` 为 `z'zhong'guo`；
  **整句**（`[wubi] sentence`，缺省关，issue #3）：连着打的编码进 `sentence::CodeLattice`（位置是字母，边是编码正好等于那 1 到 4 个字母的词，码表 + 用户词；
  句末不满四码的边再收前缀命中、扣 `TAIL_PREFIX_PENALTY` 6，扫参数字在常数的注释里），`sentence::convert_codes` 与拼音共用 `viterbi::search_lattice`（静态 n-gram、个人 n-gram、上屏链上文、`WORD_PENALTY`）；
  `convert_wubi_sentence` 按字母数取路径（至少 `MIN_RESCORE_PATHS`、到 `rescore_paths` 封顶）走 `rescore_paths`，异步重排与拼音同一条路；
  第二轮拼路径时五笔按编码字母对齐（`rescoring::PathUnit::CodeLetters`：五笔一个词只有一条编码，按音节数对不齐），所以路径上每个词的「音节」记的是敲的那段字母而不是完整编码；`SpanCache` 的键带 `\u{1}` 前缀（编码 `a` 与音节 `a` 撞键）；
  `insert_wubi_sentence`：作用域 ≤ 4 码且码表有命中时候选不动，否则首选是整句（`CandidateKind::Sentence`，`syllables = [整段]`，`typed_display` 为 `gggg'khlg`），
  后面跟开头那段编码的全码词（4 → 1 码，选了只吃那一段）；有占位（`z` 这类码表里没有的键）不出整句；开着时 `check_wubi_auto_commit` 整个不做；
  上屏时 `sentence_words` 按编码重算路径、逐词 `record_word`（不造词）；只在 `code_only()` 时有，混输下这个选项不起作用。
  评测（9807 句按码表转全码、冷启动、不接神经重排）：首选 86 版 79.7% / 98 版 76.3% / 新世纪 77.2%（同一批句子全拼是 27.5%），86 版接上 `--neural data/model/model.qjm` 是 91.7%（真前 k 条路径 + 第二轮拼合对五笔同样有效），逐键 1 ms 以内；
  98 与新世纪低 3 个点是因为码表词频是名次，语言模型不认识的词兜底时分不出高低（`docs/plan/wubi.md`）；
  上屏消耗在 `engine/commit/wubi.rs`：吃候选编码那么长，自动造词按 `encode` 出编码进用户词（造不出就不造），五笔下连着上屏两次即造（同缓冲区阈值），空码回车 `record_raw` 不学成英文词；
  `scheme_key()` 只用形码时是 `Variant::key()`（`wubi86` / `wubi98` / `wubixsj`）、混输时是 `<拼音侧>+<版本键>`（`pinyin+wubi86` / `xiaohe+wubi98`），输入日志与回放据此切方案；
  **混输**（`wubi.is_some() && phonetic`，配置 `[general] scheme` 不为 `none` 且 `wubi` 非空）在 `engine/query/mixed.rs`：`wubi_candidates` 的结果按「编码 == 作用域」切成两段，
  中间夹 `query_pinyin` 的整条 Query（含直输段 `is_raw` 按拼音侧判），顺序是**全码五笔 → 拼音 → 编码前缀五笔**（前缀命中排前面的话 `kai` 的首选会变成 `kaik` 的词），
  按文本去重、取 `MAX_CANDIDATES`；拼音解析失败不算错（`ih`），整段按五笔走，两边都空才 `Err`；五笔码最长四位，第 5 个字母起自然只剩拼音。
  混输下关掉的：`z` 反查（`z` 是声母）、四码自动上屏与顶字（`niha` 这类四字母同样可能是拼音，`check_wubi_auto_commit` 只在 `code_only()` 时调）、自动造词（前后两次上屏可能一次编码一次拼音，`auto_word_syllables` 返回 `None`）；
  模式键与双拼一样走 `shifted()`，纠错门槛是作用域 > `MAX_CODE_LEN`（四码以内还可能是编码）。
  上屏时哪条是五笔候选由 `is_wubi_candidate` 判：五笔候选只有一个「音节」且它以整段作用域开头（全码等长、前缀命中更长），拼音候选的音节要么更短要么不是前缀；
  正好重合的只有「单音节吃满整段」那种，两条路算出来的消耗与学习键一样）/ `emoji` /
`english`（英文模式候选）/ `engine`（`query::EnglishTail`：句末英文词并入整句，`woxiangxuehaorust` → 我想学好rust，尾段也像拼音时按分数与拼音读法比）。
`Engine` 是对外唯一门面，`Translator` / `Learner` trait 在 `engine` 模块；词库是「主词库 + 附加词库（`set_extra_dictionaries`，同步抽取 `@` 编码的英文名称供补全）+ 用户词」的列表；繁体输出（`traditional` 开关与 `traditional_map` 映射）依赖 `ferrous-opencc`（`s2tw`）在出候选与上屏边界转换，内部保持简体。
`Engine` 是对外唯一门面，`Translator` / `Learner` trait 在 `engine` 模块；词库是「主词库 + 附加词库（`set_extra_dictionaries`，同步抽取 `@` 编码的英文名称供补全）+ 用户词」的列表。
- 中英混输的英文词位置：`Engine::set_chinese_first`（配置 `[general] chinese_first`，缺省关）关着时拼音不像话的输入英文排第一（`extras::insert_english`，
  用户老选中文词时仍让中文在前），开着时整句先插、英文词紧随其后排第二（`query_inner` 里两步的先后按开关掉转）；句末英文词并入整句（`EnglishTail`）不受它影响。
  缺省关是回放定的（9241 词 / 269 条英文上屏：缺省开英文首选 82.5% → 7.1%）。
- 整句读哪种切分：`parser::segment` 只按字母排（音节少、残缺少，再同就前面的音节长），零声母边界上会偏向前字吞 n / g（`dangao` 排出 `dang ao`）。
  `Engine::prefer_convertible`（`engine/query/segmentation.rs`）在与第一种同形（音节数、残缺数相同）的切分里按整句得分挑最高的挪到最前；
  查询（在查词之后、插整句之前）、上屏重算 `sentence_words` / `mixed_words`、中英混输比分、纠错的原样得分、云联想的拼音与本地参考都先调它，几处读的是同一种切分。
  只有一种同形切分时不做转换。光标按音节移动 / 按音节删与光标后剩余拼音的显示走 `Engine::preferred_segmentation`（同一种挑法）；
  查询与剩余拼音显示把挑出的切分记在 `preferred_segmentations`（最多 4 条，与 `span_cache` 一起清），方向键直接复用，不重转整句。
- 整句搜索的参数打包在 `sentence::Search`（`keep_partial` / `paths` / `initial` / `protected_extra` / `reading` / `whole_word`），`convert_paths` 只收它；拼音的整句转换都经 `Engine::convert_sentence_paths` 一处（`convert_sentence_with` 取它的第一条，五笔整句经 `convert_wubi_sentence`）。
- 词图与找路分开：`sentence/lattice/` 的 `Lattice` trait 回答「位置 `[start, end)` 上有哪些词」，`SyllableLattice`（拼音，位置是音节，格子候选 `span_candidates` 与原样成词保护都在它里面）
  与 `CodeLattice`（五笔，位置是编码字母）各实现一份；`viterbi::search_lattice` 只认 trait，`convert_paths` / `convert_codes` 是两个入口。2026-09-19 抽出时干净集 9808 句的评测数字一位不变。
  - **接上文**：`initial` 取 `Engine::context()`（`engine/surrounding/`：**上屏链上有词就用链，链是空的才用壳给的应用光标前文，都没有才当句首**；
    链上的词是用户亲手选的、还带音节，且壳只在一段组句起头读一次前文，这段里后来上屏的词不在前文里），一段拼音的第一个词按 `ln(w·P(词|上文) + (1−w)·P(词|句首))` 算（`viterbi::initial_step`，静态模型与个人 n-gram 两种读法各自插值后再混），
    `w` = `INITIAL_CONTEXT_WEIGHT` 0.5（`Interpolation::initial_weight`，`--tune initial=`）；第二个词的个人三元前二词也接 `initial.previous`。全按接续算时「前词 → 的」压过实词（`dizhi` 出 的只）。
    `Engine::set_surrounding_before(Some(前文))` 是壳送应用光标前文的入口：按末尾连续汉字切出最后两个词存成 `SurroundingBefore`，
    只在文本变了时切一次；词级排序（`query/mod.rs`）与五笔整句也用同一个 `Engine::context()`，学习记的转移仍只看上屏链。
    `Engine::seed_chain(光标前文)` 是另一条路：直接把切出来的词摆进链（不学习、不写日志），整句评测的 `chain` 口径用它。数字见 `long-sentence.md`。
  - **每词代价**：路径上每个词扣 `WORD_PENALTY` 1（`Interpolation::word_penalty`，`--tune word=`），几个单字连起来不再压过整词（笔给 / 笔记）；不进 `static_score`，神经重排后照留。
  - **整句让整词**：整段按敲的原样读正好是一个词（没有模糊音 / 敲错代价），而整句领先它不到 `WHOLE_WORD_MARGIN` 3.5 分时，整句排到第一个词后面
    （`Engine::plain_sentence` 返回的「让位」，`Interpolation::whole_word_margin`，`--tune whole=`，0 为不让）。`shangyiye` 上一页 / `gongzuotai` 工作台 排第一。
    末尾音节没打完、整句只读了前面几个音节时不让（`xiayiz`）。
    这条整词路径常被束宽 8 剪掉，所以 `Search::whole_word` 让 Viterbi 在终点剪枝前先记下它、不在前几条里就补在最后（`viterbi::whole_word` / `add_whole_word`），与别的路径一起进神经重排再比。
    整词的分取同一段文字里最高的走法：个人 n-gram 把 工作 + 台 抬得比整词 工作台 高时，前几条里只有 工作 + 台，按它比（同文字的那条也补回来）。
    3.5 是按 `gongzuotai` 定的：带神经重排时 工作太 领先 工作台 3 分多。数字与代价见 `constant-sweep.md`。
  - **多音字读音份额**（`sentence::ReadingShare`）：语言模型只看字不看音，而词库按读音分了词频，所以一个词按某个读音命中时减掉 `ln(该读音词频 / 该词全部读音词频之和)`
    （没(mo) 约 −4.85，单读音词 0），系数 `READING_WEIGHT` 1、封顶 `READING_CAP` 6（`--tune reading=` / `reading-cap=`）。
    表在 `Engine::rebuild_reading_share` 里扫全部词库建一次（主词库 + 附加词库 + 用户词，871 个多读音词，约 13 ms），装词库 / 换学习器时重建，不跟着上屏走。
    扣在 `sentence::transition_log_prob`：**只在静态模型认得这个词时减**（`fallback` 用的就是这条词目自己的词频，再减是扣两遍），减在个人插值之后
    （个人 n-gram 也只认字不认音）；算好的值存 `SpanWord::reading`，与模糊音 / 敲错的 `penalty` 分开，也不参与格子挑候选（`span_candidates` 用的词频本来就按读音分开）。
    不进 `static_score`：神经重排替换的是静态模型那部分，而字级模型同样不认识读音。词级排序（`query/mod.rs`）用同一个函数，同样扣。数字见 `long-sentence.md`。
  - **原样成词保护**：一条敲错边整个落在「按敲的原样读出的多音节词」里面（`ce shi` 测试 里的 `ce` → `de`）多扣 `TypoCosts::protected_extra` 4（`SyllableLattice::typed_word_reach`，`--tune protect=`）；
    伸到原样词外面的（`mei gan xi` 没关系 对 美感）不拦，模糊音（代价 < `PROTECTED_MIN_COST`）不算猜敲错。数字见 `long-sentence.md`。
- **个人 n-gram 怎么插值**（`sentence::UserNgram::blend`）：两层绝对折扣，每层折出来的质量给下一层——
  三元 `max(c(u,v,w) − D₃, 0)/c(u,v)` 回退到二元，二元 `max(c(v,w) − D₂, 0)/c(v)` 回退到**静态模型** P(w|v)（不是个人一元），
  个人一元 `c(w)/N` 只占 `1 − USER_LAMBDA`（0.8）那份兜底；最后按 μ = c(v)/(c(v) + `CONFIDENCE_K` 16)、封顶 `MAX_CONFIDENCE` 0.5 与静态模型插值。
  `BIGRAM_DISCOUNT` = `TRIGRAM_DISCOUNT` = 2（`--tune d2=` / `d3=`）与记录侧是同一把尺：一次普通事件记 `TRANSITION_WEIGHT` 2 份，
  正好被扣光、抬不动首选，同一条接续再来一次才算数；用户翻下去改选的记 `EXPLICIT_TRANSITION_WEIGHT` 4 份，一次就压得过一次回声。
  折出来的质量按实际折掉的算，不是 `D·类数/总数`（个人数据里计数比 D 小的占多数，后者会超过 1）。数字与取舍见 `constant-sweep.md`。
- `Engine::learn_text(文本)`（`engine/learning/text.rs`）：从用户自己写的文本学个人 n-gram——按非汉字切小句、`segment_text` 切词、逐词 `record_transition`（每句从句首起，
  每条按一次普通事件记 `TRANSITION_WEIGHT` 份），不记词频、不造词、不写日志，私密输入中不学；文本从哪来是壳的事。
  留出评测首选 23% → 61%，见 `long-sentence.md`（加折扣之后文本里只出现一次的搭配不再抬分，同一份留出掉 5 个点，见 `constant-sweep.md`）。
- 两个候选开关都在 Core 生效、缺省开：`Engine::set_mixed_english`（配置 `[general] mixed_english_candidates`）关掉时 `insert_english` 直接返回（精确词与补全都不出，句末英文词 `EnglishTail` 照旧）；
  `Engine::set_emoji_candidates`（配置 `[general] emoji_candidates`）关掉时 `insert_emoji` 直接返回，emoji 表照常加载，热重载改开关不用重建 Engine。
  emoji 跟在前 `EMOJI_SCAN` 5 个候选里配了 emoji 的词后面（每词最多 `EMOJI_PER_WORD` 2 个，共 `EMOJI_TOTAL` 3 个），但最早排在下标 `EMOJI_MIN_INDEX` 3、
  也不插进一串快捷写法或英文词中间（`extras::emoji_slot`）：
  前三位留给字词（`zhengshi` 的 🎩 👔 原来紧跟首选，把 正是 挤到第四）。回放首选不变，没命中首选的 118 种词往前挪（菜单 / 收集 / 右键 4 → 2），
  往后挪的只有 3 次 emoji 上屏，见 `constant-sweep.md`。
- `custom_phrase::merge_replacements` 把平台给的「输入码 → 短语」表（macOS 系统文本替换）并进配置里的自定义短语：每条占该码最靠前的空位（1–9），
  输入码不是小写字母、已有同码同文本、九位都满的跳过；Core 不管数据从哪来。
- 拼写纠错（`correction`）：整段一处编辑的变体里相邻换位允许末尾音节没敲完（`loose_segmentation`，`mignt` → `ming t…`；
  有「每个音节都完整、末尾不是落单单字母」的变体时残尾变体不参与，免得 `keyyi` 的 可以 被 `ke yi y…` 抢走），
  纠正之间比较时换位减 `TypoCosts::correction_transpose_discount`（1 nat，CLI `--tune correction-transpose=`），与原样比不减；
  2026-09-15 回放 283 词 / 23 句：词首选 89.0%、整句 82.6%，与改前持平（折扣若也用在与原样比，`zhongwne` 会误纠成 中文，整句掉一条）。

`EngineSession` 保存可挂起的组句、标点、历史与学习链，`Engine::swap_session` 在同一个引擎里交换输入状态，共用词库与落盘服务。切换上下文时清除查询及异步预测缓存，并由平台恢复各自私密状态。

`Engine::discard_input` / `EngineSession::discard_input` 用于隐私能力变化时无痕清理输入，包括透传缓冲、学习链和暂存词汇曝光；`set_private` 只切换写入开关，保留已输入的组句。

## crates/glimmer-translate

`Glossary`，本地 TSV 释义表（词性 + 译文）；`LevelTable`，词汇等级表（`assets/levels/levels-{en,ja}.tsv`，CEFR A1–C2 / JLPT N5–N1，
`uv run tools/corpus/levels.py` 从 `data/levels/` 的原始 CSV 生成，来源与许可见 `assets/levels/README.md`），「统计」页按级数词汇用，不进候选。

## crates/glimmer-learning

- `FrequencyLearner`：用户选择次数（`user.tsv`）、按输入串记的选择（`user-choices.tsv`，词级排序里同输入串、同位置选过的次数换成得分加分，格式与迁移见下）、用户词（`user-words.tsv`，主词库同格式，
  Engine 与主词库一起查）、个人英文词（`user-english.tsv`，回车原样上屏的英文词与选过的英文候选，与随包英文词表一起出候选且在前）、
  个人敲错表（`user-typos.tsv`，接受过的 (敲的, 要的) 音节对，词图敲错边与整段纠错的代价按它打折）与个人 n-gram（`user-ngram.tsv`，Core `sentence::UserNgram`，
  二元 + 三元在线计数，整句转换与词级排序里与静态模型插值；Tab 接受的云端整句按 `sentence::segment_text` 切词后也记；
  连着选出的两个词记够次数自动造词进用户词，一段拼音分几次选完的合成词记两次也造）。
  按输入串记的三张表（`user-words.tsv` / `user-choices.tsv` / `user-typos.tsv`）按方案分目录：`from_path_with_scheme(path, Some("wubi98"))` 把它们放到词频文件同目录的 `wubi98/` 下
  （三个五笔版本各一个子目录：`wubi86/` / `wubi98/` / `wubixsj/`，互不共用）
  （五笔编码与拼音音节撞键，`a` 既是音节也是 工 的简码），按文本记的 `user.tsv` / `user-ngram.tsv` / `user-english.tsv` 各方案共用；子目录首次落盘时建。
- **`user-choices.tsv` 按句首 / 句中分桶**：四列 `输入串\t词\t次数\t位置`，位置是 `start`（句首）/ `after`（句中）/ `any`（不分位置）。
  `ChoiceCounts` 一对 (输入串, 词) 存三个计数，查权重 = 当前位置那一桶 + `any` 那一桶；`MAX_CHOICE_ENTRIES` 与 `decay_choices` 仍按「对」算、三桶一起减半。
  位置由 Core 的 `ChoicePosition` 定（`Engine::context()` 的 `previous.is_none()`）：记录用上屏那一刻的上文，查询用当前上文；
  整段分次选完的合成词按**整段开头**那一刻的位置记（`CommitChain::buffer_position`）。回车原样上屏的 `<raw>` 与句首句中无关，只记 `any`。
  为什么分：这一项是得分里的一份加分（`ranking::choice_bonus`，系数 1.5 / 封顶 20 次），不分位置时同一份加分在句首句中给同一个词，
  用户的 `ba` 选过 吧 25 次、把 21 次，句首也只能出 吧。
  只有「这段字母更想要中文还是英文」（`Engine::insert_english`）用不分位置的总数 `Learner::choice_total`：中英取舍是语言偏好，与句首句中无关。
- **`user-choices.tsv` 的迁移**：老的三列行读成 `any`，加载时（个人 n-gram 读完之后）一次性按 `c(<s>, 词) / c(词)` 的比例四舍五入拆进 `start` / `after`，
  n-gram 不认识的词留在 `any`（两种位置都算），拆完标脏、下次落盘写成四列。`any` 拆完清零，所以重复迁移是空操作。
  位置列放在次数后面是为了向下兼容：老版本按前三列解析、忽略多余列，读四列文件仍能读出次数，只是同一对的两行会互相覆盖（留下最后读到的那一桶），
  不会整行当坏行丢掉。
- `InputLog`：输入日志（`input-log.jsonl`，每次上屏一行：敲的键、切分、看到的前几个候选、选了第几个、来源、纠错、撤销，
  Core `InputLogger` trait 的落盘实现，`[general] input_log` 缺省开，只写本机，给离线回归评测与个人模型用）。
- `UsageStats`：输入统计（`usage.tsv`，按天记汉字 / 中文词 / 英文词 / 上屏次数，Core `UsageMeter` trait 的实现，Engine 每次上屏 `Usage::of_text` + 按来源定词数，
  整句按 `segment_text` 切词数；与输入日志无关，偏好设置「统计」页显示，`book_scale` 折成几本《某书》）。
- `VocabularyBook`：词汇记录（`user-vocab.tsv`，Core `VocabularyTracker` trait 的实现：学习语言的每条译词看到过几轮 / 上屏过 / ⌥+数字 打出过几次；Core 私密输入统一跳过曝光和提交写入，但仍可读取已有记录用于排序和生词标记；
  Engine `annotate` 据此填 `Sense::fresh`，看到轮次不到 `FRESH_UNTIL` = 3 的译词壳里画橙色；「看到」按上屏那一刻屏幕上那一页算，壳每次画完 `Engine::note_displayed` 告知当前页）。
- 各表落盘走 Core `storage::write_atomic`（临时文件 + fsync + 改名），加载按行容错（坏行警告跳过，真读不了壳退回内存学习），
  壳激活期间每 60 秒 `Engine::flush_learning`；IMK 回调边界 `imk::catch_panic` 拦 panic、缓冲区字母原样上屏（见 architecture.md「崩溃不丢」）。

## crates/glimmer-predict

- `CloudPredictor`：`Predictor` trait 的网络实现（async-openai，OpenAI 兼容接口，默认 DeepSeek），后台线程防抖 / 缓存 / 超时，`submit` / `poll` 非阻塞。
  `PredictConfig` 是配置的 `[predict]` 分节。只在组句中联想，一次请求给云端词（容错校验后补进候选第一页末尾 `[predict] slots` 格，缺省 2，不预留不占位，
  前面的本地候选不挪；排布在 Core `CandidateLayout`）和整句补全（preedit 右侧，Tab）；上屏后不联想，本地历史不进请求。
  简拼（半数以上音节是缩写）的请求 `max_items = 0`，只求整句补全（`prediction::mostly_abbreviated`）：按声母凑出来的词大多是生造词，
  拼音校验又按首字母序列匹配放行缩写，拦不住；问字模式的答案不受这条限制。
- `CloudGlossFiller`：释义兜底（Core `GlossFiller` trait，与 Predictor 分开的线程与通道，攒 1.5 秒 / 8 个词发一次，问过不再问）：
  随包释义表没有的词库词 / 云端词上屏后入队，结果壳每秒 `Engine::poll_glosses` 经 `Translator::learn` 写进 `glimmer-translate::PersonalGlossary`
  （`user-glossary-<语言>.tsv`，`LayeredTranslator` 个人表优先）；随云联想开关一起开。
- 问字键（缺省 `u`）开头是问字模式（`PredictionKind::Question`，答案带读音、不校验拼音），`?` 开头要 `ModeKeys::question_mark` 开着才算（配置 `[shortcut] question_mark`，缺省关，壳用 `Engine::takes_question_mark` 决定空缓冲区的 `?` 是入口还是标点）；`PredictionKind::Translate` 是壳里快捷键触发的「翻译选中文字」
  （双向：汉字为主译成学习语言，外文译回中文，`prediction::translation_target`），译文走结果的 `sentence`。

## crates/glimmer-update

- 应用内检查更新，平台无关：`check_blocking` / `UpdateCheck`（线程 + 通道）拉 `[update] feed_url`（缺省 GitHub latest Release 的 `releases.json`，
  结构见 `release.md`）→ `Feed::available(current, Target::current())`：在有本机平台 / 架构安装包的条目里按版本号取最大（清单三个平台的版本号各排各的，
  不能取第一条），架构按清单标签别名表认（`Apple Silicon` / `Intel` / `x64` / `x86_64` / `ARM64`，新包型在 `target.rs` 加）。
- `Version`：SemVer 比较，`-dev-<哈希>` 单独拆成 dev 标记——同号带 dev 的比发布版小、比前一版大（纯 SemVer 会把 `alpha.7-dev-x` 排在 `alpha.7` 后面）。
- `download_blocking` / `Download`（带进度）：写 `<文件名>.part` 边下边算 sha256，对上清单才改名；清单没 sha256 拒绝；目录里旧的 `Glimmer-*` 先删。安装本身交给壳。
- `UpdateConfig` 是 `[update]` 分节（`check` 缺省开、`feed_url`）；`check_due` / `touch` 用数据目录 `update-check` 文件的 mtime 做「每 12 小时最多一次」（`CHECK_INTERVAL`）。
- 联网测试：`cargo test -p glimmer-update -- --ignored` 真拉一次缺省清单。

## crates/glimmer-format

`.qj` 数据容器（`Container` mmap 读、`Writer` 写、`Table<T>` / `Text` 零拷贝视图、`hash` 可落盘哈希索引、`Metadata` 名称 / 许可证 / 署名）。
词库与语言模型都能 `write_qj` / 从 `.qj` 打开，启动 50 ms；`cargo run --release -p glimmer-dict-convert -- pack dict|lm --name … --license …`
生成 `data/generated/{dict,lm}.qj`，`bundle.sh` 在 TSV 更新时自动重打并只把 `.qj` 打进包。设计见 `docs/design/architecture.md`「数据文件：`.qj` 容器」。

## crates/glimmer-neural

`CharScorer`，Core `sentence::SentenceScorer` trait 的实现：candle 加载字级 Transformer（GPT-2 风格 decoder，训练仓库（本地 `../train`，私有，不在本仓库）导出的
`model.safetensors` + `config.json` + `vocab.json`），给「前文 + 整句」按字累加 log 概率；前文的每层 K / V 缓存（`PrefixCache`），
同一段前文只算一次，每个候选只算自己那几个字（64 字前文 × 8 条 28 ms，Metal）。features `accelerate` / `metal` 换后端，壳用 `metal`。

Engine 侧在 `engine/rescoring/`：接了打分器就取 Viterbi 的前几条路径（每个音节 2 条，`MIN_RESCORE_PATHS` 6 到 `RESCORE_PATHS` 24，`--tune paths=` 改封顶）。
前几条是真的前 k 条：词图上每个节点留前 k 种走法（`sentence/viterbi/arrival.rs` 的 `Arrival`，回指带「接的是前驱的第几种走法」），只要一条时与只留最优回指逐位相同；
以前只留最优回指，「前 k 条」只是 k 个句尾词各自的最优链，句子中间全一样。
第二轮（`rescoring/combination.rs`）：各条路径相对词级最优路径净赚、互不重叠的替换拼成一条新路径，得分按可加估、再拿神经分一起排；异步时它的分晚一拍，
壳收到第一轮的分重查之后 `rescoring_pending()` 又为真，要接着 `request_rescoring`（mac `poll_rescoring`、Server `tick`、CLI `rescoring::settled` 都这么做）。
每条路径按 `路径分 + λ·(神经分 − 静态二元分)` 重排（λ `NEURAL_WEIGHT` 0.5，
个人 n-gram / 用户加分 / 代价不动），分走「前文 + 文本 → 神经分」缓存 `NeuralCache`；同步打分器（`with_sentence_scorer`，CLI 评测）当场补分，
异步的（`with_async_sentence_scorer`，后台线程 `RescoreWorker`）查询不等模型：缺分的记下来，壳停键后 `request_rescoring`、`poll_rescoring` 到了再 `query` 一次。
前文优先用壳给的应用光标前文（`set_surrounding_before`），没有用本会话最近 64 个上屏字符。CLI `--neural <导出目录>`（`--neural-weight` / `--neural-context` / `--neural-async`）。

## crates/glimmer-lm

`NgramModel`（`model/` 目录：`mod.rs` 结构与加载、`parse.rs` TSV、`write.rs` 落盘、`score.rs` 打分、`export.rs` 导出），
Core `sentence::LanguageModel` trait 的实现，从 `data/generated/lm.qj`（或 `lm-unigram.tsv` / `lm-bigram.tsv` / 可选 `lm-trigram.tsv`）加载
（没有这些文件就退化为一元词频整句）。数据由 `tools/corpus/parquet_to_text.py`（uv 脚本，HF parquet → 简体纯文本）加
`cargo run --release -p glimmer-dict-convert -- bigram --phrases assets/lexicon/phrases.tsv --phrases assets/lexicon/domain_words.tsv --phrases assets/lexicon/common_words.tsv --brand assets/lexicon/brand.tsv --brand assets/lexicon/mixed_words.tsv data/corpus/*.txt` 生成；语料在 `data/corpus/`（gitignore）。
短语层不当 token 统计（分词时摘掉、统计完按成分合成一元 / 二元，短语得分等于原来两个词的路径，见 `bigram/mod.rs` 模块注释），品牌词按给定次数写进一元与句首二元；
**三元不给短语合成**，短语在那一层没有条目、退回二元。

`.qj` 的分节：`WORD` / `ENTR` / `HASH` 词表，`OFFS` / `SUCC` 二元 CSR（按前词分组），`TOFF` / `TSUC` 三元 CSR
（段号 = 二元在 `SUCC` 里的下标，一条二元 (u,v) 就是一个三元上下文），`CSUM` / `CONT` 是回退要的派生量
（每个前词的后继计数之和、每个词的不同前词数 N₁₊(·,w)）。后四节都是后加的，缺了照样加载：没有 `TOFF` 退化成二元，
没有 `CSUM` / `CONT` 就加载时扫一遍 `SUCC` 算（写进文件只多 1 MB，省掉启动时把几百万条二元全读一遍）。

打分是三元 → 二元 → 一元的绝对折扣回退（`Smoothing`：`mode` / `trigram_discount` D₃ / `bigram_discount` D₂，
`glimmer-cli --tune lm-mode=,lm-d3=,lm-d2=` 可换）。二元那层的分母用语料真实的 c(v)（一元计数）而不是表里后继计数之和：
二元表按次数砍过，用和当分母等于把留下的几条抬高十几倍。`mode` 有三档：0 是改成三元之前的固定 λ=0.8 插值（复现旧数字用）、
1 是绝对折扣 + 一元 c(w)/N、2 再把一元换成接续概率 N₁₊(·,w)/N₁₊(·,·)。数字与取舍见 `language-model.md`。
`unigrams()` / `bigrams()` / `trigrams()` 按编号顺序导出全部计数，`dict-convert supplement` 用它在已有 `lm.qj` 上补词（三元原样带走）。

## crates/glimmer-platform

`Config`（TOML 配置文件，`[general]` / `[shortcut]` / `[fuzzy]` / `[dictionaries]` / `[apps]` / `[predict]` 分节，首次运行写模板，
`set_value` 用 toml_edit 原地改键保留注释；`[model] enabled` 本地整句模型开关，`LocalModelConfig`）；
输入方案是两条轴：`config/scheme.rs` 的 `Scheme`（拼音侧，`Pinyin` / `Shuangpin(六套)` / `Zhuyin` / `Off`，`ALL` 给设置界面按顺序列，`log_key` 给输入日志）与 `[general] wubi` 的 `WubiVariant`（形码侧）；
`GeneralConfig::scheme()` 在 `scheme` 为空串时回落到旧键 `shuangpin` / `zhuyin`（老配置照常，写了 `scheme` 就不看它们），`mixed()` 是两边都开，
`scheme_label(pinyin, wubi)` 出状态条上的方案名（五笔在前，与候选顺序一致；单开全拼时为空串）；`extra_dictionaries` 列出 / 加载随包领域词库与用户 `dicts/`
（mac 壳与 Windows Server 共用，同名 `.qj` 优先于 `.tsv`）；`protocol` 模块是 Windows Server ↔ TSF DLL 的 IPC 协议类型
（`ClientMessage` / `ServerMessage` / `Frame` / `PreeditSegment`，全 serde，两端共用，见 `docs/design/architecture.md`「Windows：TSF」）。

## crates/glimmer-server

输入法 Server 的平台无关部分，从 `apps/windows/server` 抽出，Windows Server 进程与 Linux IBus 引擎进程共用：`assembly`（按 `AssemblySpec` 装配 Engine：词库 / 释义 / 学习 / 语言模型 / 五笔码表——`WubiSpec` 给版本 + `[wubi]` 选项，`load_scheme` 按 `Variant::data_file()`
在主词库同目录找 `wubi86.qj` / `wubi98.qj` / `wubixsj.qj`，学习器按 `Variant::key()` 分子目录；配置热加载在 `dispatch/reload/wubi.rs`，开关或换版本才重开码表；
拼音侧 `set_phonetic(scheme().is_on() || !engine.wubi_mode())`——配置说关但码表没装上时留着拼音兜底，所以要在五笔装完之后设）、
`dispatch::Router`（`ClientMessage` → Engine → `ServerMessage` / `Frame`：多会话、按键、上屏、翻译选中文字、配置热加载、本地整句模型重排的节拍 `next_tick` / `tick`）、`error::ServerError`。
不含传输与绘制：候选窗 / 状态条经 `CandidateSink` / `StatusSink` 由平台壳注入，默认不画。键码沿用 Windows 虚拟键码（VK）语义，Linux 端把 keysym 翻成 VK 再发。
输入日志的会话条目（版本号 / 平台名）由壳用 `Router::set_log_identity` 设置。集成测试 `tests/engine_loop.rs` / `wubi_loop.rs` 用 `assets/sample/` 样例数据，全平台可跑。

## crates/glimmer-render

自绘渲染器：候选窗一帧 + 主题 → 预乘 RGBA 位图，tiny-skia 栅格 + cosmic-text 文字（fontdb 按平台清单只加载几个字体文件、不扫系统），
自己解析 `trak` 字距表、按主题 gamma 加深笔画；cosmic-text 打了 `opsz` 光学字号补丁（qingjian-team/cosmic-text 分支 `qingjian-opsz`，workspace `[patch.crates-io]` 钉 rev）。
`examples/preview.rs` 出 PNG 与真机截图并排比、`--measure` 与 AppKit 对宽度。mac 壳 `candidates/bitmap/` 贴位图，`[general] renderer = "system"` 切回 AppKit 绘制
（过渡期退路，偏好设置「候选窗口」页可选）；`[general] font` 是候选窗字族名（空为系统字体，`bitmap/font_files.rs` 用 CoreText 按字族名找文件只加载那几个，没装就回系统字体；
设置页 `preferences/font_picker/` 是搜索框 + 列表）；`[general] font_size` 是候选词字号（12–28，缺省 16 = `DEFAULT_TEXT_SIZE`），`Theme::with_text_size` 让译文与序号字号、三者行高同比缩放（行高向上取整）；`[general] color_scheme` 选配色预设（`Palette::blue` 等，各有浅深两版），`background_color` / `highlight_color` 经 `Palette::with_custom` 覆盖，字色按底色亮度自动配（`Palette::from_colors`）。设计与验收见 `docs/design/rendering.md`。

## apps/cli

测试工具，`cargo run -p glimmer-cli -- kaifa`。

- `--predict` 强制开云联想并等结果打印，交互模式下上屏后也联想。
- `--typing` 逐键计时（性能测试用 release 构建跑，目标每键 10 ms 以内）。
- `--chinese-first` 打开中文优先（`[general] chinese_first = true` 的排法），配合 `--replay` 比两种英文词位置。
- `--replay <input-log.jsonl>` 回放评测：把日志里每次上屏的键重新喂给引擎，按来源算首选 / 前五命中率、平均名次、不在候选的条数，打印没命中的例子（`--misses N`）；
  只在内存里学习不写文件，加 `--user-dict` 可带上现有学习数据。日志每条带 `scheme`，回放在 `replay/scheme.rs` 的 `SchemeSwitcher` 里按它切双拼 / 注音 / 五笔 / 混输
  （`split` 把 `xiaohe+wubi98` 拆成拼音侧 + 版本键，`wubi86` 单独出现是「只用五笔」）：要用五笔的条目得同时给对版本的 `--wubi`，没给就跳过并计数（`wubi_missing`）。
  一次回放只装得下一种五笔版本（`slot` 是单槽位：拼音条目时把码表卸到槽里、五笔 / 混输条目再装回来），日志里其他五笔版本的条目一律计入 `wubi_missing`；
  混了多种版本的日志要按版本各跑一遍。
- `--scheme pinyin|xiaohe|…|zhuyin|none` 覆盖 `[general] scheme`（拼音侧）；`--shuangpin` 是它的旧写法，两个都给以 `--scheme` 为准，`off` 等于全拼。
  `none` 配上 `--wubi` 才是「只用五笔」，两边都开就是混输。
- `--wubi 86|98|xsj|off` 覆盖 `[general] wubi`（`xsj` 即新世纪，也可写 `06`）：码表只认 `data/generated/` 下该版本的 `wubi86.qj` / `wubi98.qj` / `wubixsj.qj`
  （没有就报错，用 `dict-convert wubi` + `pack dict --output` 生成），`[wubi]` 选项照配置；
  `--user-dict` 给了时按输入串记的表落该版本的方案子目录（`wubi86/` / `wubi98/` / `wubixsj/`）。`--typing` 逐键计时不模拟四码自动上屏（`set_input` 不走 `push`）。
- `--wubi-sentence` 覆盖 `[wubi] sentence`（五笔整句）。只用五笔（`--scheme none --wubi <版本>`）时 `--eval-text` 把句子按码表转成编码来评（`Transcriber::codes`：
  码表有的词打词码、没有的逐字打，同一个字词取最长的那条编码即全码）；冻结的句子集照用，拼音那一列被换掉，码表里没有的字的句子跳过；`--eval-typos` 在这时忽略。
- `--tune 名=值`（逗号分隔）覆盖个人 n-gram 插值（含整句接上文的权重 `initial`、每词代价 `word`、词图每格候选数 `span`）与敲错代价（含原样成词保护 `protect`）的常数扫网格
  （名字见 `apps/cli/src/tuning.rs`，Core 侧是 `Engine::set_interpolation` / `set_typo_costs`，壳只用缺省值）；
  `lm-mode` / `lm-d3` / `lm-d2` 换静态语言模型的回退方式与折扣，它们在建模型时就用上（`tuning::smoothing`），不走 `Engine`。
- `--lm <目录或 lm.qj>` 换一份语言模型（目录里认 `lm.qj` 或 `lm-unigram.tsv` + `lm-bigram.tsv` + 可选 `lm-trigram.tsv`），评测新模型不必动 `data/generated`。
- `--eval-text <文本>...` 整句评测：把用户自己写的中文文本按标点切句、按词库读音转成全拼，冷启动喂给引擎看整句能不能还原原句
  （首选命中率 / 字准确率 / 查询耗时；不依赖日志里当时选了什么，给整句排序与语言模型的改动当尺子），`--eval-save` 冻结成 `句子\t拼音\t上文` 三列文件，
  之后直接 `--eval-text` 它保证比的是同一份句子（本机的在 `data/eval/sentences.tsv`）。排序、整句、纠错的改动先跑它们再合。
  报告按句长分桶（1–5 / 6–10 / 11–15 / 16–20 / 21 字以上，`eval/bucket.rs`），抽句上限 40 字；上文同时摆进上屏链（`Engine::seed_chain`）。
  `--learn-text <文件>...` 先从这些文本学个人 n-gram 再做后面的事（配 `--eval-text` 做留出评测；给了 `--user-dict` 随退出落盘，输入法开着时别对着它的数据目录写，会被它下一次落盘盖掉）。
  `--eval-chunk N` 分段输入：每句按语言模型切词、N 个词一段逐段喂，前面各段的原文当上文（量整句接上文）；
  `--eval-context chain|app` 上文走哪条路：`chain` 摆进上屏链（缺省，历来口径），`app` 只经 `Engine::set_surrounding_before` 给、上屏链保持空（量「前文是别人的话 / 粘贴来的」）；
  `--eval-typos` 每条拼音注一处敲错（相邻键换位 / 敲到旁边的键，按句子哈希定、可复现），量敲错纠正的召回；两个可以叠用。

## apps/macos

IMK 输入法，源码按 `app / host / imk / candidates / menubar / preferences` 分目录。

- 输入法菜单（状态项 + 系统输入源菜单）与偏好设置窗口都是配置文件的前端：只写 `config.toml`，`Host::apply_config` 一条通路热加载，激活期间每秒看一次文件 mtime。
- `apps/macos/scripts/bundle.sh --install` 打包安装（开发用）：`/Library/Input Methods/` 已有 pkg 装的正式版就 sudo 覆盖它，否则装 `~/Library/Input Methods/`；
  **两处不能并存**（同 bundle ID 两份时系统拉起挑错路径、选了输入法立刻退回上一个，2026-09-18 踩到），pkg 的 postinstall 反过来把 `~/Library` 的开发版挪到数据目录 `Glimmer-dev-<时间>.app.bak`；
  同一路径反复覆盖不用注销，换路径要注销再登录。`--pkg` 做分发用的 pkg（装 `/Library/Input Methods/`，postinstall 跑 `glimmer-macos --register`
  注册、启用并切成当前输入源；签名 / 公证靠 `GLIMMER_SIGN_IDENTITY` / `GLIMMER_INSTALLER_IDENTITY` / `GLIMMER_NOTARY_PROFILE`，没设就 ad-hoc；`GLIMMER_TARGET` 指定架构，
  成品 `target/pkg/Glimmer-<版本>-<arm64|x86_64>.pkg`）；`scripts/uninstall.sh` 卸载。
- 日志在 `~/Library/Logs/Glimmer/`（按天分文件留 7 天，删了会重建），用户数据与配置在 `~/Library/Application Support/Glimmer/`。
- 配置项：云联想 `[predict]`（偏好设置「云服务」页有「测试连接」按钮：`glimmer_predict::ConnectionTest` 起线程发一条最小请求，`Host` 用独立定时器 `CloudTestMonitor` 轮询结果显示到窗口底部；
  `reasoning_effort` 缺省 `none`，DeepSeek V4 默认思考，不关正文为空）；模糊音 `[fuzzy]` 默认都关；`[general]` 学习语言（`off` 不显示译文）/ 译词读音 `translation_reading`（关掉时 `Engine::annotate` 去掉音标 / 假名）/ 每页候选数 / 翻页键 / 外观 / 竖排横排 / 拼音显示位置 /
  英文模式候选开关 / 中文优先 `chinese_first` / 中文模式英文词候选 `mixed_english_candidates` / emoji 候选 `emoji_candidates` / 拼音方案 `scheme`（`pinyin` 全拼 / 小鹤 / 自然码 / 微软 / 搜狗 / 小浪 / 智能ABC / `zhuyin` 大千注音 / `none` 关；空串时回落到旧键 `shuangpin` / `zhuyin`，老配置照常）/ 五笔 `wubi`（空为关，`86` / `98` / `xsj`，与拼音方案是两条轴、都开就是混输；行为在 `[wubi]`：`auto_select` / `hint` / `fixed_order_length`，
  码表 `Resources/wubi86.qj` / `wubi98.qj` / `wubixsj.qj` 三份都随包，按版本装一份，偏好设置「通用」页的「五笔」弹出菜单四项（关 / 86 / 98 / 新世纪）由 `Variant::ALL` 出，
  壳每次 `push` 后先 `take_auto_commit`）/ 日志级别 `log_level`（缺省 info 不含敲的内容，debug 逐键记，热切换）/ 输入日志 `input_log`；
  `[shortcut]` 模式键 v / u、`question_mark`（缺省关，开了空缓冲区敲 `?` 进问字）、上屏第一 / 第二个译词的修饰键 `translation` / `translation_second`、删候选 `delete_candidate`（缺省 shift，用户词整删、词库词清学习）、翻译选中文字 `translate_selection`、macOS 中英文切换 `mode_switch`（缺省 `shift` 单击，也支持旧版修饰键加字母，不能与翻译键冲突）；
  `[apps] english_candidates_off` 按 bundle identifier 列出英文模式不给候选的应用（缺省终端 / 编辑器 / IDE，`*` 前缀匹配）；
  `[dictionaries] domains` 打开随包的领域词库（`Resources/dicts/` 12 本（含 AI 与软件开发），缺省只开 `idioms`），`disabled` 关掉用户目录 `dicts/` 里的某本导入词库；
  偏好设置「词库」页随包的可开关、导入的可开关 / 移除，可导入 TSV / Rime yaml / .qj。
- 系统文本替换（系统设置「键盘 → 文本替换」）：`host/config/text_replacements.rs` 从 `NSUserDefaults` 全局域读 `NSUserDictionaryReplacementItems`
  （每条 `{ on, replace, with }`），激活输入法时重读，变了就经 Core `merge_replacements` 并进配置里的自定义短语再 `set_custom_phrases`；
  `[general] system_text_replacements` 开关（缺省开，「自定义短语」页勾选框），内容可能含证件号、地址，日志只记条数。
- 输入法进程由 launchd 拉起，看不到 shell 的环境变量：密钥写进配置同目录的 `.env`（`GLIMMER_API_KEY=...`，输入法启动时 dotenvy 读入）或 `config.toml` 的 `api_key`。
- 本地整句模型：`bundle.sh` 把 `data/model/`（或 `GLIMMER_MODEL_DIR`）三件套打进 `Resources/model/`，用户目录 `model/` 优先；`host/model/mod.rs` 在后台线程加载并预热（首次 Metal 编译）后
  `set_async_sentence_scorer` 接上，`refresh` 在组句第一键读应用光标前 64 字给 Engine 当上文与模型前文（有没有本地模型都读，Secure Input 不读）、查询后 `schedule_rescoring`，`RescoreMonitor` 停键 80 ms 请求、20 ms 轮询，
  结果到了重查一次只重画当前页（翻过页 / 动过高亮不动）；「云服务」页有开关（`[model] enabled`）。
- 从文件学习（`host/learn_text.rs`）：「高级」页「从文件学习…」（`Setting::LearnText`）→ `choose_text_sources`（文件可多选、可选文件夹）→ 收 `.md` / `.markdown` / `.txt` / `.text`
  （单个 ≤ 4 MB、最多 2000 个、往下 6 层、隐藏项与符号链接不进）→ 逐个 `Engine::learn_text` → `flush_learning`，结果写底部状态行；读与学都在主线程，日志只记个数。
- 检查更新（`host/update/`）：`init` 末尾 `schedule_auto_update_check` 排两只定时器（30 秒一次性 + 12 小时重复），响了走 `auto_update_check`（`[update] check` 开着且 `~/Library/Application Support/Glimmer/update-check` 超过 `CHECK_INTERVAL` 才起 `UpdateCheck`；网络全在它的线程里，主线程一次也不等），
  `UpdateMonitor` 0.2 秒轮询；有新版本菜单版本行变「微明 x（新版本 y 可用）」、「关于」页三行状态 + 「下载并安装」（`Download` 下到数据目录 `updates/`，校验过 `open` pkg 交给 Installer.app）；
  菜单「检查更新…」跳到「关于」页（`preferences::ABOUT_PAGE`）手动查。自动查失败不出声，手动查才报错。
- 端到端验证可用 `osascript` 的 System Events 往 TextEdit 发按键再读回文本（终端需要辅助功能权限；输入法得在中文模式）。
- 中英文切换的状态、事件优先级与验证边界见 [macOS 中英文切换](../design/macos-mode-switch.md)。
- macOS 安装后 `repair-input-cache.sh` 定向备份微信 / 企业微信中不含微明的旧键盘缓存；已包含微明则不动。工具随包放在 `Resources/`，不退出应用；已运行的应用需重启。见 [输入源缓存](input-source-cache.md)。

## apps/windows

一个产品两个 package：`server`（Server 进程：命名管道传输 + 自绘候选窗与悬浮状态条；Engine 装配与协议分派在 `crates/glimmer-server`）与 `tsf`（TSF 文本服务 DLL，lib 名固定 `glimmer_tsf`），
外加 `settings`（WinUI 3 设置程序）与 `installer`（Inno Setup）。不合成一个 crate，因为 DLL 不能带 Engine 的依赖树，见 `apps/windows/README.md`；
协议类型在 `glimmer-platform::protocol`，设计见 `docs/design/architecture.md`「Windows：TSF」。

TSF 原有数字 / OEM 标点 / 空格键码按当前布局用 `ToUnicodeEx` 解析（bit 2 避免改变键盘状态），
仅接受单个非代理项 UTF-16 单元。字母、小键盘和 AltGr 处理不变，不保证组合音符输入。
拼音显示位置（`[general] preedit`）在 Windows 上分两处落地：Server 把它读进 `RouterConfig.preedit` 并随 `Frame.preedit_mode`
下发给 DLL，DLL（`com/service/key_sink.rs`）按 `inline()` 决定要不要放行内拼音，Server（`ui/candidates/render_data.rs::window_preedit`）
按 `in_window()` 决定候选窗口顶部画不画拼音行；`window` 模式没有组句范围，光标矩形改从 `com/edit/anchor.rs::caret_rect`（当前选区）量。
连不上 Server 时 DLL 自己拉起它（`tsf/src/com/service/launch.rs`）：`ShellExecuteW` 起与 DLL 同目录的 `glimmer-server.exe`
（`uiAccess=true` 的 exe 用 `CreateProcess` 报 740），进程内 5 秒冷却 + 跨进程命名互斥体防止砸出一串 Server；
起完清掉重连退避，下一键就试。Server 只在登录时由「启动」文件夹拉起，中途挂了以前只能等下次登录。

检查更新只在设置程序里（Server 没有界面）：打开设置时 `[update] check` 开着且 `%APPDATA%\Glimmer\update-check` 超过 12 小时就后台查一次（`spawn_background` + `check_blocking`），
「关于」页显示结果与「下载并安装」（`download_blocking` 到 `%APPDATA%\Glimmer\updates\`，校验过用 `explorer` 拉起 `Setup.exe`，走 ShellExecute 才能弹 UAC；安装器会 taskkill Server 与设置程序）。

词库导入（设置「词库」页）走 `glimmer-dictionary::import` 转成 `.qj`（空词库拒绝），多选批量、成功的从 `[dictionaries] disabled` 摘掉、页面显示每个文件的结果；
Server 每次轮询比对用户 `dicts\` 的路径 / mtime / 长度快照，配置没变也重载新增、同名更新与移除；配置解析失败时词库沿用上次有效的开关（#36）。

## apps/linux

IBus 引擎进程 `glimmer-ibus`（package `glimmer-linux`），设计见 `docs/design/architecture.md`「Linux：IBus / Fcitx」。

- 分层：`ibus/`（zbus 连私有总线、导出 Factory / Engine、IBus 对象的变体编码）→ `frontend/`（与 D-Bus 无关的会话状态机，`Frame` → preedit / 候选表）→ `backend/`（`Backend` trait：`RouterBackend` 是真后端，`EchoBackend` 只验链路）；`key/` 是 keysym → VK 与单击 Shift 判定；`startup.rs` 是 XDG 路径、配置模板与日志。
- 启动：`glimmer-ibus --ibus`（`--echo` 换回显后端）；日志写 stderr 与 `~/.local/share/glimmer/logs/glimmer-ibus.<日期>.log`，zbus 压到 warn。
- 常数：组句中 `Poll` 间隔 60 ms；空闲时学习数据 60 s 落盘一次。
- 打包：`scripts/package.sh`（Linux 上）/ `scripts/package-docker.sh`（macOS 上，`GLIMMER_DATA_DIR` 指向有产品数据的 `data/`）出 `target/deb/Glimmer-<版本>-<arch>.deb`；装机布局与版本号规则见 `docs/notes/release.md`。
- 测试：`cargo test -p glimmer-linux`（keysym、变体签名、会话状态机）；`tests/docker/run.sh`（回显后端 + 真 ibus-daemon）；`tests/docker/install.sh <deb>`（干净 Ubuntu 装包、真 Router 打字）。
- `fcitx5/`：Fcitx5 插件。`src/`（package `glimmer-fcitx5`，staticlib + rlib）是 C 接口：`global.rs`（进程级后端 / 日志 / `glimmer_tick`）、`session/`（会话与事件函数，转给 `frontend::Session`）、`outputs/`（指令串与访问器，preedit 光标换字节偏移、候选补数字标签）；
  接口声明 `include/glimmer_fcitx5.h`，改一边要改另一边。`addon/`（CMake，`src/{engine,state,candidate,action}.cpp`）出 `glimmer.so` 与两份 conf。常数：轮询 60 ms。
  构建：`cargo build --release --locked -p glimmer-fcitx5` → `cmake -S apps/linux/fcitx5/addon -B <build> -DCMAKE_INSTALL_PREFIX=/usr -DGLIMMER_RUST_LIB=<target>/release/libglimmer_fcitx5.a -DGLIMMER_DATA_ROOT=/usr/lib/glimmer` → `cmake --build` → `DESTDIR=<暂存> cmake --install`；
  Ubuntu 24.04 装到 `/usr/lib/<multiarch>/fcitx5/glimmer.so`、`/usr/share/fcitx5/addon/glimmer.conf`、`/usr/share/fcitx5/inputmethod/glimmer.conf`。运行时 `GLIMMER_DATA_ROOT` 覆盖资源根、`GLIMMER_ECHO=1` 用回显后端；日志 `~/.local/share/glimmer/logs/glimmer-fcitx5.<日期>.log`。
  测试：`cargo test -p glimmer-fcitx5`（回显后端驱动 C 接口、空指针）；`fcitx5/tests/docker/run.sh`（真 fcitx5 + 样例词库）；`e2e.py` 不设 `GLIMMER_FCITX5_STAGE` 时是装机模式（系统插件 + `/usr/lib/glimmer`），装 deb 后 `dbus-run-session -- python3 e2e.py`。

## assets

- `assets/sample/`：手写样例词库与释义表，不是产品数据。
- `assets/emoji/emoji-zh.tsv` / `emoji-en.tsv`：Unicode CLDR 中文 / 英文 annotations 转出的 emoji 表（Unicode License v3，可发布；中文词与英文词各配 emoji，两张表加载时合成一张），
  `cargo run --release -p glimmer-dict-convert -- --out-dir assets/emoji emoji --language zh data/cldr/annotations-zh.json data/cldr/annotationsDerived-zh.json`（en 同理）。
- 英文词表词频：`uv run tools/corpus/english_frequency.py data/generated/english.tsv -o data/generated/english-frequency.tsv`，再 `... english <词表> --frequency <那个文件>`。
- `assets/wubi/`：三份五笔码表的上游原文与许可，一个版本一个子目录，各有 README 记来源 URL、提交 / 取回日期、SHA256 与规模：
  - `wubi86/`：rime-wubi 的 `wubi86.dict.yaml`（提交 `152a0d3`，2023-10-25，YAML 头 `version: "0.7"`）+ `LICENSE.LGPL-3.0` + `AUTHORS`，LGPL-3.0。
  - `wubi98/`：yanhuacuo/98wubi 的含词表 `wubi98_ci.dict.yaml` + `LICENSE.LGPL-3.0`，LGPL-3.0，≈ 11.3 万条（单字 3.6 万 + 词 7.6 万）。
    不用同作者 `98wubi-tables` 的 `wubi98_U.dict.yaml`（Unlicense）：那份是纯单字表，没有词组，多字词只能逐字上屏。
  - `wubixsj/`：GuoBinyong/wubixinshiji 的 `wubixinshiji.dict.yaml` + `AUTHORS`（AUTHORS 声明 LGPL），上游是王永民《五笔字型大一统》的新世纪整合词库，≈ 11.2 万行。
  产品数据 `wubi86.qj` / `wubi98.qj` / `wubixsj.qj` 都由 `dict-convert wubi` 转出（见下），逐版本的命令写在各子目录 README 里。

## tools/gloss-gen

用 LLM 批量生成释义表：`cargo run --release -p glimmer-gloss-gen -- generate`（密钥读 `GLIMMER_API_KEY`，结果 JSONL 在 `data/generated/`，不进 git、可续跑，`--limit 80` 试跑）
再 `... export`（写 `glossary-{en,ja}.tsv`，产品数据在 `assets/glossary/`，见那里的 README；格式 `词\t词性. 译词[|读音]`，日语读音是假名、英语是音标）。CLI 与 bundle.sh 用的就是这两个文件。
英文音标不用模型：`uv run tools/corpus/ipa_en.py` 把 ipa-dict 的美音表 `data/ipa/en_US.txt`（CMUdict 风格）换写成课本写法 `data/ipa/en_US-textbook.txt`
（规则全在脚本里，Rust 只查表不改写），再 `... ipa` 给已有的 `glossary-en.tsv` 原地补（或 `export --ipa <表>` 导出时写）；多词短语逐词查全命中才拼。

## tools/dict-convert

产品数据的生成工具，输出到 `data/generated/`（gitignore）。

- `ai`：AI 与软件开发领域词库，从 `assets/lexicon/ai/{terms,names}.tsv` 和 `domains/*.tsv` 校验生成 `dicts/ai.tsv` / `ai.qj`、`ai-audit.tsv`、`ai-coverage.tsv`；8,279 源条目，主词库去重后 7,726 条（2026-10-19）。`--exclude` 只排除必选主词库，`--corpus` 同时给 `corpus.txt` 和 `development.txt`，人工权重与真实次数分列，英文按标识符边界计数。`alias` 支持 `C++/@cpp`，`mixed` 支持 `Git分支/@gitfenzhi`，构建拒绝同码不同词。Core 统一开关，英文补全个人选择优先、领域次之；配置 `domains` 仍用 `ai`。见 [设计](../design/ai-dictionary.md)。

- `lexicon`：从 `assets/lexicon/`（自建词库源：规范字 + 常用词 + THUOCL 领域词）加 Unihan 读音（`data/unihan/Unihan_Readings.txt`）、LLM 多音字标注（`gloss-gen pinyin`，
  结果 `data/generated/pinyin-llm.jsonl`，不进 git，**随数据包的 `glimmer-llm-intermediates.tar.gz` 发**，丢了就得从已发布的词库里反推，见 `internet-slang.md`）、
  语料词频（`lm-unigram.tsv`）建基础词库 `dict.tsv`（11.8 万条，含补充常用词）。标注可给一个词多个读音与占比（一行 `yi hang` 0.7 / `yi xing` 0.3，占比 < 5% 的次要读音丢掉），每个读音一条、词频按占比分，引擎的多音字读音份额就从这里来；旧版只有 `pinyin` 一项的文件照读，并把 THUOCL 领域词按语料次数 < 50 拆成
  `dicts/<领域>.tsv` + `.qj`（11 本、13 万条，`--domain-keep-min`），流程见 `assets/lexicon/GLIMMER.md`；`--extra-words` 并入人工挑的领域词 `assets/lexicon/domain_words.tsv`、
  补充常用词 `common_words.tsv`、网络用语 `04_internet_slang/slang_words.tsv` 与中英混杂词 `mixed_words.tsv`。
  音节判定在 `syllable.rs`（`lexicon` 与 `supplement` 共用）：汉字只认合法拼音音节，**拉丁字母 / 数字自成一节**（`C盘` → `c pan`、`P0` → `p 0`），
  否则 Unihan 查不到读音、整条会被 `dropped`。给定读音以词表为准（`U盘` 读 `you pan`）；放行只对非汉字生效——`儿` 的 Unihan 读音里有儿化的 `r`，
  对汉字放行会顶掉 `er` 成主读音。`kMandarin` 取全部值而不是第一个：`嗯` 是 `ǹg en`，只取第一个这个字就一个合法音节都不剩。
- `english`：转 `assets/lexicon/05_english/00_all_words.tsv`；`cedict`：释义表备用来源。
- `bigram`：统计语料的一元 / 二元 / 三元（同一遍分词），写 `lm-unigram.tsv` / `lm-bigram.tsv` / `lm-trigram.tsv`；
  三元的阈值与上限单列（`--min-trigram-count` 缺省 5、`--max-trigrams` 缺省 1000 万，一条在 `lm.qj` 里 8 字节），
  内存里三元表到 4000 万条就整批剪枝（扔掉不超过阈值的、阈值再加一，见 `bigram/counts.rs`），输出还砍掉上文本身不在二元表里的那些；
  `--phrases` 给短语层、`--brand` 给品牌词（`assets/lexicon/brand.tsv`，微明 210）与中英混杂词（`mixed_words.tsv`，C盘 / B站：合成计数要成分词在语料里，C 不是 token，只能直接给一元，次数对着同音竞争词定），领域词也走合成计数（语料里只有几十次的词当 token 统计会吸走成分词的二元证据）。
  基础词库里语料一次都没切出来的多字词（一行：词频是底值，总被切成 一 / 行）也自动走合成计数（`bigram::phrase::unseen_components`，只管 `dict.tsv`、不管 `dicts/`，免得把领域词拉回基础词库），下一轮 `lexicon` 才有它的真实词频。
- `gaps`：找常用词缺口，不用语料：外部词表（每行 `词[\t拼音]`，CC-CEDICT / jieba 词表转出，不进仓库）里主词库、`dicts/` 与语言模型都没有的 2–4 字词，
  按 `lm.qj` 成分二元合成次数（`bigram::phrase_count`，与 `--phrases` 同一公式），读音词表给了就用、否则由成分主读音拼，写 `gap-candidates.tsv` 供人工挑进 `common_words.tsv`。
- `supplement`：补充词表并进已有数据，不用语料（读音判定与 `lexicon` 共用 `syllable.rs`，所以含字母的词也收）：`assets/lexicon/dict.tsv` 原行顺序不动、新词按 (词, 音节) 二分插入；`lm.qj` 展开成计数（`supplement/counts.rs` 的 `LmCounts`）后
  走 `synthesize_phrases` 合成，写 `dict.tsv` + `lm-unigram.tsv` / `lm-bigram.tsv` 再 `pack`。已有的词跳过，幂等；与全量 `lexicon --extra-words` + `bigram --phrases` 等价。做法与数字见 `common-words.md`。
- `mine`：从语料挖词库没收的高频词并过滤（`oov_filter.rs`：虚词规则 + 相邻字对 PMI≥3，`--candidates` 只重过滤）。
- `phrases`：挖短语层（两遍扫语料：相邻两词、两段二元都够频的相邻三词，总次数与对话语料次数都 ≥ 2000 + 边界规则，读音由成分词拼出；我的 / 不知道 / 有没有 这类常用词表不收的组合，
  `assets/lexicon/phrases.tsv`；词库已并入过短语时重跑加 `--refresh`）。
- `wubi`：Rime 四列五笔码表 → 微明 TSV `词\t编码\t词频`（编码整个当一个音节）。86 / 98 / 新世纪同一种格式、同一个子命令。
  解析 YAML 头的列序（`text` / `code` / `weight[` / `stem]`）与表体；跳过编码含 `z`（符号表，`z` 留给反查）、超四码或含非小写字母的行；
  缺省按常用字集过滤（GB2312 一二级汉字 ∪ `--charset` 主词库里出现过的字，词的每个字都在集合里才留），`--extended` 全留；同（词，码）合并取最大词频，按（编码，词频降序，词）排序；
  顺带建单字全码表（每字取最长码），有 `stem` 列时与它核对，不一致只记 warn。86 原表 136,239 条 → 73,647 条，单字 8,236、有四码全码的 7,335。
  **一半以上的行没有 `weight` 列时补词频**（`apply_in_code_order_weights`，日志记 `in_code_order_weights=true`）：同一编码下按出现顺序编名次 r（从 0 起），
  词频 = `1_000_000 / (r + 1)`、最小 1，组与组之间不可比。新世纪那份上游 99% 的行只有 `词\t编码` 两列，词频全 0 时候选顺序跟上游排好的常用度完全无关。
  为什么是名次倒数而不是整表线性递减，两条缺一不可：Core 的分数是 `ln(词频 / 总频)` 再取整到千分位，相邻只差 1 的词频取整后完全打平、等于没补；
  上游文件除开头 25 行一级简码外整体按编码排序，「文件顺序 = 常用度」只在同一编码组内成立，跨组比大小没有意义。86 与 98 都有词频，一行不动。
  **`--out` 必填**（三个版本的 TSV 同在 `data/generated/`，给缺省值会互相覆盖）。生成（`<版本>` = `wubi86` / `wubi98` / `wubixsj`）：
  ```sh
  cargo run --release -p glimmer-dict-convert -- wubi --from rime assets/wubi/<版本>/<上游>.dict.yaml \
    --out data/generated/<版本>.tsv --charset data/generated/dict.qj
  cargo run --release -p glimmer-dict-convert -- pack dict --input data/generated/<版本>.tsv --output <版本>.qj \
    --name … --license … --attribution … --source … --data-version …
  ```
  每个版本的完整命令（含上游文件名与 META 的许可 / 署名 / 来源 / 版本）写在 `assets/wubi/<版本>/README.md` 里，改数据时改那里。
- `pack dict|lm|glossary`：打 `.qj`（释义表也进容器）；`--output` 改输出文件名（五笔码表打成 `wubi86.qj` / `wubi98.qj` / `wubixsj.qj`，缺省按种类 `dict.qj` / `lm.qj` / `glossary-<语言>.qj` / `model.qjm`）。
