//! 英文语料 → 一元 / 二元 / 三元三张 TSV（`lm-en-unigram.tsv` 等），格式与中文 `bigram` 的
//! `lm-unigram.tsv` / `lm-bigram.tsv` / `lm-trigram.tsv` 完全相同，`pack lm` 直接打成 `lm-en.qj`，`glimmer_lm::NgramModel` 直接加载。
//!
//! **token 是什么。** 小写、保留撇号的词：`english.tsv` 里的词转小写去重（`I` 的 token 是 `i`，`we'll` 与 `well` 是两个 token）。
//! 运行时引擎拿「候选词转小写」去查这个模型，所以这里必须同样只做 ASCII 转小写，不去撇号、不做别的归一。
//!
//! **切词只用 `glimmer_core::english::tokenize`**，一行（一句或一条消息）切一次，事件这样用：
//!
//! - `Word(w)`：`w` 在词表里就是一个 token；不在 = 断开，与 `Gap` 一样处理。**不删掉它再把两边接起来**，那会造出语料里没有的接续。
//! - `SentenceEnd`：一句结束，下一个词从句首算（句首标记 `<s>`，与中文工具一致：一元里 `<s>` 的计数是句子数，
//!   句首词记二元 `(<s>, w)`，第二个词记三元 `(<s>, w₁, w₂)`）。
//! - `Gap`：断开但不是句首：后面的词不与前面的相连，也不记句首二元。
//!
//! 一句话在计数里就是一串 `Option<编号>`，断开是 `None`，交给中文工具同一个 [`Counts::record`]；
//! 计数、三元表的整批剪枝、输出时的阈值与上限、三元上文必须在二元表里，全部复用 `bigram` 的实现。
//! 内存：二元全放内存，三元到 `--max-trigram-entries` 条就整批剪枝（见 `bigram/counts.rs`）。
//!
//! [`Counts::record`]: crate::bigram::Counts::record

mod options;
mod vocabulary;

use std::collections::HashSet;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;
use std::time::Instant;

use glimmer_core::english::{Token, tokenize};

use crate::bigram::{Counts, top_bigrams, write_bigram, write_trigram, write_unigram};
use crate::error::ConvertError;

pub use options::NgramOptions;
use vocabulary::EnglishVocabulary;

/// 表头注释里写的子命令名。
const COMMAND: &str = "english-ngram";

/// 每读这么多行检查一次三元表要不要剪枝。
const PRUNE_EVERY_LINES: u64 = 200_000;

/// 扫语料，写出 `<前缀>-unigram.tsv` / `-bigram.tsv` / `-trigram.tsv`。
pub fn convert(options: &NgramOptions, out_dir: &Path) -> Result<(), ConvertError> {
    let started = Instant::now();
    let vocabulary = EnglishVocabulary::load(&options.words)?;
    tracing::info!(words = vocabulary.len() - 1, "英文词表加载完成");
    let mut counts = Counts::new(vocabulary.len(), options.max_trigram_entries);
    let mut sentence: Vec<Option<u32>> = Vec::new();
    let (mut lines, mut sentences, mut tokens, mut unknown) = (0u64, 0u64, 0u64, 0u64);
    for path in &options.corpus {
        tracing::info!(path = %path.display(), "统计语料");
        for line in BufReader::new(File::open(path)?).lines() {
            let line = line?;
            lines += 1;
            if lines.is_multiple_of(PRUNE_EVERY_LINES) {
                counts.prune_trigrams();
            }
            for token in tokenize(&line) {
                match token {
                    Token::Word(word) => {
                        let id = vocabulary.id(&word);
                        tokens += 1;
                        unknown += u64::from(id.is_none());
                        sentence.push(id);
                    }
                    Token::Gap => sentence.push(None),
                    Token::SentenceEnd => sentences += u64::from(flush(&mut counts, &mut sentence)),
                }
            }
            // 有词的流一定以句末结尾，这里只是保险
            sentences += u64::from(flush(&mut counts, &mut sentence));
        }
        tracing::info!(
            lines,
            bigrams = counts.bigram.len(),
            trigrams = counts.trigram.len(),
            "进度"
        );
    }
    tracing::info!(
        lines,
        sentences,
        tokens,
        unknown,
        distinct_bigrams = counts.bigram.len(),
        distinct_trigrams = counts.trigram.len(),
        elapsed_s = started.elapsed().as_secs(),
        "统计完成"
    );
    let path = |table: &str| out_dir.join(format!("{}-{table}.tsv", options.prefix));
    let pairs = top_bigrams(&counts.bigram, options.min_count, options.max_bigrams);
    write_bigram(&pairs, &vocabulary.words, &path("bigram"), COMMAND)?;
    let kept: HashSet<u64> = pairs.iter().map(|(key, _)| *key).collect();
    write_trigram(
        &counts,
        &kept,
        &vocabulary.words,
        options.min_trigram_count,
        options.max_trigrams,
        &path("trigram"),
        COMMAND,
    )?;
    write_unigram(
        &counts.unigram,
        &vocabulary.words,
        &path("unigram"),
        COMMAND,
    )?;
    tracing::info!(
        elapsed_s = started.elapsed().as_secs(),
        "英文 n-gram 已写出"
    );
    Ok(())
}

/// 把攒着的一句记进计数并清空；一个词表里的词都没有的句子不记（不给句首标记白加一次）。返回记了没有。
fn flush(counts: &mut Counts, sentence: &mut Vec<Option<u32>>) -> bool {
    let recorded = sentence.iter().any(Option::is_some);
    if recorded {
        counts.record(sentence);
    }
    sentence.clear();
    recorded
}

#[cfg(test)]
mod tests;
