//! 把数据源转换成微明的 TSV 格式，或打包成 `.qj`。
//!
//! - `lexicon`：微明基础词库，「输入法字词库_分类整理版」数据包（规范字 / 常用词 / THUOCL 领域词）+ Unihan 读音 + LLM 多音字标注 → `dict.tsv`
//! - `cedict`：CC-CEDICT（CC BY-SA 4.0）→ `glossary-en.tsv`（释义表的备用来源，现在用 gloss-gen 的 LLM 表）
//! - `english`：`词\t编码` 英文词表（数据包的 `05_english`，ESDB / CSpell，MIT）+ 展示写法 + wordfreq 词频 → `english.tsv`（一码可多词）
//! - `english-ngram`：英文语料（`data/corpus/en/train/`）→ `lm-en-unigram.tsv` / `lm-en-bigram.tsv` / `lm-en-trigram.tsv`，再 `pack lm --output lm-en.qj`
//! - `english-case`：规范书面语料（FineWeb）→ `english-case.tsv`（原样写法的次数，给 `english --case-counts` 定大小写写法的先后）
//! - `english-misspellings`：typos 误拼对照（`05_english/06_misspellings.tsv`，MIT）→ `english-misspellings.tsv`
//! - `emoji`：Unicode CLDR annotations（Unicode License v3，`--language zh|en`）→ `emoji-<语言>.tsv`（可发布，放 `assets/emoji/`）
//! - `bigram`：纯文本语料（如 `tools/corpus/parquet_to_text.py` 转出的中文维基 CC BY-SA 4.0、LCCC 对话 MIT）→ `lm-unigram.tsv` + `lm-bigram.tsv`
//! - `mine`：语料里分词落成连续单字的段 → `oov-candidates.tsv`（词库没收的高频词，标音后用 `lexicon --extra-words` 并入）
//! - `phrases`：bigram 表的相邻两词 + 语料的相邻三词 → `phrases.tsv`（我的 / 不知道 这类短语层，读音由成分词拼出，同样用 `lexicon --extra-words` 并入）
//! - `gaps`：许可清楚的外部词表 × 已有 `lm.qj` 的成分二元 → `gap-candidates.tsv`（词库与语言模型都没有的常用词，人工挑进 `assets/lexicon/common_words.tsv`）
//! - `supplement`：补充词表并进已有的 `dict.tsv` 与 `lm.qj`（合成计数，不用语料）→ `dict.tsv` + `lm-unigram.tsv` + `lm-bigram.tsv`
//! - `wubi`：Rime 五笔码表 `assets/wubi/<方案>/*.dict.yaml`（86 / 98 / 新世纪）→ `wubi<方案>.tsv`（编码当音节，缺省按常用字集过滤，`--extended` 全留）
//! - `pack dict|lm|glossary|model`：TSV → `.qj` 容器（`dict.qj` / `lm.qj`），带名称 / 许可证 / 署名元数据，输入法与 CLI 优先加载它；
//!   `model` 把本地整句模型的三件套目录打成一个 `model.qjm`；`--output` 改输出文件名（五笔码表打成 `wubi86.qj` / `wubi98.qj` / `wubixsj.qj`）
//!
//! 输出默认写到仓库根目录 `data/generated/`（gitignore）。

mod ai;
mod args;
mod bigram;
mod cedict;
mod emoji;
mod english;
mod error;
mod lexicon;
mod oov_filter;
mod pack;
mod phrases;
mod supplement;
mod syllable;
mod wubi;

use clap::Parser;
use tracing_subscriber::EnvFilter;

use crate::args::{Args, Command};
use crate::error::ConvertError;

fn main() {
    if let Err(error) = run() {
        eprintln!("error: {error}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), ConvertError> {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .with_target(false)
        .init();
    let args = Args::parse();
    std::fs::create_dir_all(&args.out_dir)?;
    match args.command {
        Command::Ai {
            source,
            exclude,
            corpus,
        } => ai::convert(&source, &exclude, &corpus, &args.out_dir),
        Command::Lexicon {
            pack,
            unihan,
            pinyin,
            pinyin_readings,
            frequency,
            emit_ambiguous,
            extra_words,
            domain_keep_min,
        } => lexicon::convert(
            &pack,
            &unihan,
            pinyin.as_deref(),
            pinyin_readings.as_deref(),
            frequency.as_deref(),
            emit_ambiguous.as_deref(),
            &extra_words,
            domain_keep_min,
            &args.out_dir,
        ),
        Command::Cedict { input } => cedict::convert(&input, &args.out_dir.join("glossary-en.tsv")),
        Command::English {
            inputs,
            frequency,
            display_forms,
            frequency_floor,
            case_counts,
            case_min_total,
        } => english::words(&english::WordsOptions {
            inputs,
            display_forms,
            frequency,
            frequency_floor,
            case_counts,
            case_min_total,
            output: args.out_dir.join("english.tsv"),
        }),
        Command::EnglishCase { corpus, words } => english::case(&english::CaseOptions {
            corpus,
            words,
            output: args.out_dir.join("english-case.tsv"),
        }),
        Command::EnglishNgram {
            corpus,
            words,
            prefix,
            min_count,
            max_bigrams,
            min_trigram_count,
            max_trigrams,
            max_trigram_entries,
        } => english::ngram(
            &english::NgramOptions {
                corpus,
                words,
                prefix,
                min_count,
                max_bigrams,
                min_trigram_count,
                max_trigrams,
                max_trigram_entries,
            },
            &args.out_dir,
        ),
        Command::EnglishMisspellings { input, words } => {
            english::misspellings(&english::MisspellingOptions {
                input,
                words,
                output: args.out_dir.join("english-misspellings.tsv"),
            })
            .map(drop)
        }
        Command::Emoji { inputs, language } => emoji::convert(
            &inputs,
            &args.out_dir.join(format!("emoji-{language}.tsv")),
            &language,
        ),
        Command::Bigram {
            corpus,
            dict,
            phrases,
            brand,
            min_count,
            max_bigrams,
            min_trigram_count,
            max_trigrams,
            max_trigram_entries,
        } => bigram::convert(
            &bigram::ConvertOptions {
                corpus,
                dict,
                phrases,
                brand,
                min_count,
                max_bigrams,
                min_trigram_count,
                max_trigrams,
                max_trigram_entries,
            },
            &args.out_dir,
        ),
        Command::Mine {
            corpus,
            dict,
            min_count,
            max_chars,
            frequency,
            min_pmi,
            candidates,
        } => bigram::mine(
            &bigram::MineOptions {
                corpus,
                dict,
                min_count,
                max_chars,
                frequency,
                min_pmi,
                candidates,
            },
            &args.out_dir,
        ),
        Command::Phrases {
            corpus,
            dialogue,
            dict,
            refresh,
            min_count,
            max_chars,
        } => phrases::mine(
            &phrases::PhraseOptions {
                dict,
                refresh,
                corpus,
                dialogue,
                min_count,
                max_chars,
            },
            &args.out_dir,
        ),
        Command::Gaps {
            candidates,
            dict,
            lm,
            min_count,
            max_chars,
        } => supplement::gaps(
            &supplement::GapOptions {
                candidates,
                dict,
                lm,
                min_count,
                max_chars,
            },
            &args.out_dir,
        ),
        Command::Supplement {
            words,
            dict,
            lm,
            min_count,
        } => supplement::supplement(
            &supplement::SupplementOptions {
                words,
                dict,
                lm,
                min_count,
            },
            &args.out_dir,
        ),
        Command::Wubi {
            from: args::WubiSource::Rime,
            input,
            out,
            extended,
            charset,
        } => wubi::convert(&input, &out, extended, charset.as_deref()),
        Command::Pack {
            kind,
            input,
            name,
            license,
            attribution,
            source,
            data_version,
            language,
            output,
        } => pack::pack(
            kind,
            &input,
            &language,
            output.as_deref(),
            glimmer_format::Metadata {
                name,
                license,
                attribution,
                source,
                version: data_version,
                ..glimmer_format::Metadata::default()
            },
            &args.out_dir,
        ),
    }
}
