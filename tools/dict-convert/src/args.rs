use std::path::PathBuf;

use clap::{Parser, Subcommand, ValueEnum};

#[derive(Debug, Parser)]
#[command(
    name = "glimmer-dict-convert",
    about = "把第三方词库 / 词典转换成微明的 TSV，或把 TSV 打包成 .qj"
)]
pub struct Args {
    /// 输出目录
    #[arg(long, default_value = "data/generated")]
    pub out_dir: PathBuf,

    #[command(subcommand)]
    pub command: Command,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    /// 校验并生成 AI 与软件开发词库（ai.qj、ai-audit.tsv、ai-coverage.tsv）
    Ai {
        /// 词表目录（terms.tsv、names.tsv、domains/*.tsv 与 sources.json）
        #[arg(long, default_value = "assets/lexicon/ai")]
        source: PathBuf,

        /// 排除主词库或其他词库已有的词与读音，可给多个；不提供时保留全部词条
        #[arg(long)]
        exclude: Vec<PathBuf>,

        /// 许可明确的中文正文；出现次数与人工权重分开记录，不提供时只使用人工权重
        #[arg(long)]
        corpus: Vec<PathBuf>,
    },

    /// 微明基础词库：从「输入法字词库_分类整理版」数据包 + Unihan 读音建 dict.tsv（两遍跑，见模块文档）
    Lexicon {
        /// 数据包目录（含 01_characters / 02_common / 03_domains），随仓库放在 assets/lexicon
        #[arg(long, default_value = "assets/lexicon")]
        pack: PathBuf,

        /// Unihan_Readings.txt
        #[arg(long, default_value = "data/unihan/Unihan_Readings.txt")]
        unihan: PathBuf,

        /// LLM 标注的多音字词读音（`gloss-gen pinyin` 的 JSONL）
        #[arg(long)]
        pinyin: Option<PathBuf>,

        /// 多读音标注（同样格式，一个词可带几个读音与占比）：列表里含 `--pinyin` 的主读音时才采用，
        /// 主读音以 `--pinyin` 为准，这份只负责补次要读音（一行 在 yi xing 之外补 yi hang）
        #[arg(long)]
        pinyin_readings: Option<PathBuf>,

        /// 语料词频（lm-unigram.tsv）；没有就按排序号 / 文档频次给底值
        #[arg(long)]
        frequency: Option<PathBuf>,

        /// 把仍靠猜读音的多音字词写到这个文件（一行一个），交给 `gloss-gen pinyin`
        #[arg(long)]
        emit_ambiguous: Option<PathBuf>,

        /// 额外并入的词（`词\t次数`，`mine` 挖出来的 oov-candidates.tsv，可给多个）：词库里没有的按次数当词频加进去，读音同领域词
        #[arg(long)]
        extra_words: Vec<PathBuf>,

        /// 领域词在语料里出现不少于这个次数就留在基础词库，否则拆到 dicts/<领域>.qj
        #[arg(long, default_value_t = 50)]
        domain_keep_min: u64,
    },

    /// CC-CEDICT `cedict_ts.u8` → glossary-en.tsv
    Cedict {
        /// 输入文件
        input: PathBuf,
    },

    /// 英文词表（每行 `词\t编码[\t…]`，带不带表头都行，比如 `assets/lexicon/05_english/00_all_words.tsv`）→ english.tsv。
    /// 一个编码可以对多个词，同码先后、展示写法替换与词频规则见 `english/words/mod.rs` 模块文档
    English {
        /// 输入文件
        #[arg(required = true)]
        inputs: Vec<PathBuf>,

        /// 词频表（`小写词\t词频`，`tools/corpus/english_frequency.py` 生成）；给了就写进第三列，前缀补全按它排
        #[arg(long)]
        frequency: Option<PathBuf>,

        /// 展示写法表：替换同词的其它大小写写法并排到同码第一（typescript → TypeScript），可给多个
        #[arg(long, default_value = "assets/lexicon/05_english/07_display_forms.tsv")]
        display_forms: Vec<PathBuf>,

        /// 词频为 0（wordfreq 不认识）的词给的底值，Zipf × 1000
        #[arg(long, default_value_t = 3000)]
        frequency_floor: u32,

        /// 语料里原样写法的次数（`english-case` 的输出 english-case.tsv）：给了就按它定只差大小写的写法谁在前
        #[arg(long)]
        case_counts: Option<PathBuf>,

        /// 一组只差大小写的写法语料次数合计不到这个数就不采信，沿用原规则
        #[arg(long, default_value_t = 20)]
        case_min_total: u64,
    },

    /// 规范书面语料（`data/corpus/en/train/fineweb.txt`；聊天、IRC 大量全小写，别给）→ english-case.tsv：
    /// 每个原样写法（区分大小写）的出现次数，句首、全大写行与标题行不计。给 `english --case-counts` 用
    EnglishCase {
        /// 语料文件
        #[arg(required = true)]
        corpus: Vec<PathBuf>,

        /// 英文词表：只输出转小写后在词表里的写法
        #[arg(long, default_value = "data/generated/english.tsv")]
        words: PathBuf,
    },

    /// 英文语料（一行一句或一条消息）→ lm-en-unigram.tsv / lm-en-bigram.tsv / lm-en-trigram.tsv（格式同 bigram），
    /// 再 `pack lm --input <三个文件> --output lm-en.qj`。切词用 Core 的 `english::tokenize`，词表外的词当断开
    EnglishNgram {
        /// 语料文件（只给训练那份：data/corpus/en/train/*.txt）
        #[arg(required = true)]
        corpus: Vec<PathBuf>,

        /// 英文词表：词转小写就是模型的 token
        #[arg(long, default_value = "data/generated/english.tsv")]
        words: PathBuf,

        /// 输出文件名前缀：写 <前缀>-unigram.tsv 等
        #[arg(long, default_value = "lm-en")]
        prefix: String,

        /// 计数低于此值的二元组不输出
        #[arg(long, default_value_t = 3)]
        min_count: u32,

        /// 最多输出多少条二元组（按计数取前 N）
        #[arg(long, default_value_t = 3_000_000)]
        max_bigrams: usize,

        /// 计数低于此值的三元组不输出
        #[arg(long, default_value_t = 5)]
        min_trigram_count: u32,

        /// 最多输出多少条三元组（按计数取前 N）
        #[arg(long, default_value_t = 10_000_000)]
        max_trigrams: usize,

        /// 统计时三元表在内存里最多留多少条（一条约 48 字节），到了就整批剪枝
        #[arg(long, default_value_t = 40_000_000)]
        max_trigram_entries: usize,
    },

    /// 英文误拼表（`06_misspellings.tsv`）→ english-misspellings.tsv（`误拼\t正确词`）：
    /// 滤掉只差大小写的、正确词不在词表里的、误拼本身是词表里的词的
    EnglishMisspellings {
        /// 源表
        #[arg(long, default_value = "assets/lexicon/05_english/06_misspellings.tsv")]
        input: PathBuf,

        /// 英文词表
        #[arg(long, default_value = "data/generated/english.tsv")]
        words: PathBuf,
    },

    /// Unicode CLDR emoji annotations（`annotations/<语言>/annotations.json`、`annotationsDerived/…`）→ emoji-<语言>.tsv：`词\temoji …`
    Emoji {
        /// 输入的 JSON 文件
        #[arg(required = true)]
        inputs: Vec<PathBuf>,

        /// 语言代码，决定输出文件名（zh → emoji-zh.tsv，en → emoji-en.tsv）
        #[arg(long, default_value = "zh")]
        language: String,
    },

    /// 纯文本语料（每行一段）→ lm-unigram.tsv + lm-bigram.tsv：按词库分词后统计词级一元 / 二元计数
    Bigram {
        /// 语料文件（UTF-8 纯文本，简体）
        #[arg(required = true)]
        corpus: Vec<PathBuf>,

        /// 分词用的词库（微明 TSV）；同目录 dicts/ 下的领域词库会一并用于分词（词表与拆分前一致）
        #[arg(long, default_value = "data/generated/dict.tsv")]
        dict: PathBuf,

        /// 短语层文件（assets/lexicon/phrases.tsv，可给多个，人工挑的领域词 domain_words.tsv 也走这条路）：里面的词不参与分词，统计完按成分合成它们的一元 / 二元计数（见 bigram.rs 模块注释）
        #[arg(long)]
        phrases: Vec<PathBuf>,

        /// 品牌词文件（assets/lexicon/brand.tsv，可给多个，中英混杂词 mixed_words.tsv 也走这条路）：语料里没有的词按文件给的次数写进一元表。
        /// 与 --phrases 的区别：合成计数要成分词在语料里，C盘 的 C 不是语料 token，只能直接给
        #[arg(long)]
        brand: Vec<PathBuf>,

        /// 计数低于此值的二元组不输出
        #[arg(long, default_value_t = 3)]
        min_count: u32,

        /// 最多输出多少条二元组（按计数取前 N）
        #[arg(long, default_value_t = 3_000_000)]
        max_bigrams: usize,

        /// 计数低于此值的三元组不输出。三元比二元稀疏得多，阈值太低表会大到没法随包发
        #[arg(long, default_value_t = 5)]
        min_trigram_count: u32,

        /// 最多输出多少条三元组（按计数取前 N）：一条在 lm.qj 里 8 字节，1000 万条约 80 MB
        #[arg(long, default_value_t = 10_000_000)]
        max_trigrams: usize,

        /// 统计时三元表在内存里最多留多少条（一条约 48 字节，4000 万条约 2 GB），到了就整批剪枝。
        /// 内存够就调大：剪枝会误杀语料后半段才热起来的三元
        #[arg(long, default_value_t = 40_000_000)]
        max_trigram_entries: usize,
    },

    /// 从语料里挖词库没收的词：分词时被拆成连续单字的段按子串计数，出现够多的写到 oov-candidates.tsv（再交给 gloss-gen pinyin 标音、lexicon --extra-words 并入）
    Mine {
        /// 语料文件（UTF-8 纯文本，简体）；给了 --candidates 就不用扫语料
        #[arg(required_unless_present = "candidates")]
        corpus: Vec<PathBuf>,

        /// 语言模型一元表（`词\t次数`），算相邻字对 PMI 用
        #[arg(long, default_value = "data/generated/lm-unigram.tsv")]
        frequency: PathBuf,

        /// 相邻字对 PMI 的下限；0 不过滤
        #[arg(long, default_value_t = 3.0)]
        min_pmi: f64,

        /// 跳过扫语料，直接过滤上一次写出的 oov-candidates.tsv（调阈值用）
        #[arg(long)]
        candidates: Option<PathBuf>,

        /// 分词用的词库（微明 TSV）
        #[arg(long, default_value = "assets/lexicon/dict.tsv")]
        dict: PathBuf,

        /// 出现次数低于此值的不要
        #[arg(long, default_value_t = 200)]
        min_count: u32,

        /// 最多几个字
        #[arg(long, default_value_t = 4)]
        max_chars: usize,
    },

    /// 短语层：从 bigram 表的相邻两词与语料的相邻三词里挖 我的 / 不知道 这类人会整块打的组合 → phrases.tsv（人工过一遍后拷进 assets/lexicon/，`lexicon --extra-words` 并入）
    Phrases {
        /// 语料文件
        #[arg(required = true)]
        corpus: Vec<PathBuf>,

        /// 对话语料（corpus 里的一个）：短语在它里面的次数也要够 min_count，维基模板句进不来
        #[arg(long, default_value = "data/corpus/lccc.txt")]
        dialogue: PathBuf,

        /// 分词与成分读音用的词库（微明 TSV，同目录 dicts/ 一并读）
        #[arg(long, default_value = "data/generated/dict.tsv")]
        dict: PathBuf,

        /// 上一次的 phrases.tsv：词库已并入短语时重跑要给，里面的词先从分词词表里摘掉（否则 我的 是一个词，挖不出 我 + 的）
        #[arg(long)]
        refresh: Option<PathBuf>,

        /// 次数下限
        #[arg(long, default_value_t = 2000)]
        min_count: u32,

        /// 最多几个字
        #[arg(long, default_value_t = 4)]
        max_chars: usize,
    },

    /// 找常用词缺口：外部词表（每行 `词[\t拼音]`，如 CC-CEDICT / jieba 词表转出的）里词库（含同目录 dicts/）与语言模型都没有的词，
    /// 按 lm.qj 里成分词的二元合成语料次数 → gap-candidates.tsv（人工挑进 assets/lexicon/common_words.tsv）
    Gaps {
        /// 候选词表
        #[arg(required = true)]
        candidates: Vec<PathBuf>,

        /// 基础词库（微明 TSV，同目录 dicts/ 一并当已有词）
        #[arg(long, default_value = "assets/lexicon/dict.tsv")]
        dict: PathBuf,

        /// 语言模型
        #[arg(long, default_value = "data/generated/lm.qj")]
        lm: PathBuf,

        /// 合成次数低于此值的不要
        #[arg(long, default_value_t = 20)]
        min_count: u32,

        /// 最多几个字
        #[arg(long, default_value_t = 4)]
        max_chars: usize,
    },

    /// 补充词表（`词\t次数\t拼音`）并进已有词库与语言模型，不用语料：写 dict.tsv 与 lm-unigram.tsv / lm-bigram.tsv（按成分合成计数），再 pack。
    /// 与全量重跑 `lexicon --extra-words` + `bigram --phrases` 等价；已有的词跳过，重复跑幂等
    Supplement {
        /// 补充词表，可给多个
        #[arg(long, required = true)]
        words: Vec<PathBuf>,

        /// 基础词库（微明 TSV）
        #[arg(long, default_value = "assets/lexicon/dict.tsv")]
        dict: PathBuf,

        /// 已有的语言模型
        #[arg(long, default_value = "data/generated/lm.qj")]
        lm: PathBuf,

        /// 合成计数低于此值的二元不输出（与 bigram --min-count 一致）
        #[arg(long, default_value_t = 3)]
        min_count: u32,
    },

    /// 五笔码表：Rime 四列码表（`text\tcode\tweight[\tstem]`，86 / 98 / 新世纪同一种格式）→ 微明 TSV `词\t编码\t词频`，
    /// 编码整个当一个音节；缺省只留每个字都在常用字集（GB2312 + `--charset` 主词库里出现过的字）的条目，`--extended` 保留全部
    Wubi {
        /// 码表格式
        #[arg(long, value_enum, default_value_t = WubiSource::Rime)]
        from: WubiSource,

        /// 输入码表（assets/wubi/<方案>/*.dict.yaml）
        input: PathBuf,

        /// 输出 TSV；不给缺省值，免得几个方案互相覆盖
        #[arg(long)]
        out: PathBuf,

        /// 不做字符集过滤（增广字集）
        #[arg(long)]
        extended: bool,

        /// 主词库（TSV 或 .qj）：里面出现过的字并入常用字集
        #[arg(long)]
        charset: Option<PathBuf>,
    },

    /// 把 TSV 打包成 `.qj` 容器（mmap 直接用，启动近零耗时）：`dict` 读 dict.tsv 写 dict.qj，`lm` 读 lm-unigram/bigram.tsv 写 lm.qj，
    /// `glossary --language en` 读 glossary-en.tsv 写 glossary-en.qj；`model` 把训练仓库导出的三件套目录（缺省 data/model）
    /// 打成一个 model.qjm（`--out-dir data/model` 就写回原目录，随包只带这一个文件）
    Pack {
        /// 打包哪种数据
        kind: PackKind,

        /// 输入文件；`dict` 一个 TSV，`lm` 两个（一元表、二元表），`model` 一个目录。缺省从输出目录里找同名 TSV（`model` 缺省 data/model）
        #[arg(long, num_args = 1..)]
        input: Vec<PathBuf>,

        /// 元数据：名称
        #[arg(long)]
        name: String,

        /// 元数据：许可证（SPDX 标识，如 GPL-3.0-only、CC-BY-SA-4.0）
        #[arg(long, default_value = "")]
        license: String,

        /// 元数据：署名 / 版权行
        #[arg(long, default_value = "")]
        attribution: String,

        /// 元数据：来源 URL
        #[arg(long, default_value = "")]
        source: String,

        /// 元数据：数据版本（上游版本号或日期）
        #[arg(long, default_value = "")]
        data_version: String,

        /// `glossary` 专用：释义表的语言代码（en / ja / zh / es），决定输出文件名 glossary-<语言>.qj
        #[arg(long, default_value = "en")]
        language: String,

        /// 输出文件名（放在输出目录下）；缺省按种类：dict.qj / lm.qj / glossary-<语言>.qj / model.qjm。五笔码表用 wubi86.qj
        #[arg(long)]
        output: Option<String>,
    },
}

/// `wubi` 能读的码表格式。
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum WubiSource {
    /// Rime `.dict.yaml`（rime-wubi）
    Rime,
}

/// `pack` 能打的数据种类。
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum PackKind {
    /// 拼音词库
    Dict,

    /// 词级 bigram 语言模型
    Lm,

    /// 释义表（glossary-<语言>.tsv → glossary-<语言>.qj）
    Glossary,

    /// 本地整句模型（三件套目录 → model.qjm）
    Model,
}
