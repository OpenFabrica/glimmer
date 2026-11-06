//! 英文数据：词表 `english.tsv`、英文 n-gram 语言模型的三张 TSV、误拼表 `english-misspellings.tsv`。
//!
//! - [`words`]：`english` 子命令，ESDB / CSpell 词表 + 展示写法 + wordfreq 词频 → `english.tsv`（一个编码可以对多个词）。
//! - [`ngram`]：`english-ngram` 子命令，英文语料 → `lm-en-unigram.tsv` / `lm-en-bigram.tsv` / `lm-en-trigram.tsv`，
//!   格式与中文 `bigram` 完全相同，再用 `pack lm` 打成 `lm-en.qj`。
//! - [`case`]：`english-case` 子命令，规范书面语料 → 原样写法的次数 `english-case.tsv`，给 `english --case-counts` 定大小写写法的先后。
//! - [`misspellings`]：`english-misspellings` 子命令，`06_misspellings.tsv` → 运行时的 `english-misspellings.tsv`。
//!
//! 三处都只用 `glimmer_dictionary::english_code` 算编码、只用 `glimmer_core::english::tokenize` 切语料，规则不在这里另写一份。

mod case;
mod misspellings;
mod ngram;
mod words;

pub use case::{CaseOptions, convert as case};
pub use misspellings::{MisspellingOptions, convert as misspellings};
pub use ngram::{NgramOptions, convert as ngram};
pub use words::{WordsOptions, convert as words};
