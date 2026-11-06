//! 把统计好的计数写成 `NgramModel` 认的三张 TSV（一元 / 二元 / 三元），中文 `bigram` 与英文 `english-ngram` 共用。
//!
//! 词用编号表示，`words[编号]` 是它的文本，0 号是句首标记 `<s>`。表头注释里写是哪个子命令生成的，其余格式两边完全一样。

use std::collections::{HashMap, HashSet};
use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::Path;

use crate::error::ConvertError;

use super::counts::{Counts, bigram_key, trigram_parts};

/// 二元按计数降序取前 `max` 条，砍掉计数低于 `min_count` 的；同计数按键排，输出稳定。
pub(crate) fn top_bigrams(
    bigram: &HashMap<u64, u32>,
    min_count: u32,
    max: usize,
) -> Vec<(u64, u32)> {
    let mut pairs: Vec<(u64, u32)> = bigram
        .iter()
        .map(|(k, c)| (*k, *c))
        .filter(|(_, count)| *count >= min_count)
        .collect();
    pairs.sort_unstable_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
    pairs.truncate(max);
    pairs
}

/// 写二元表：`前词\t后词\t计数`。`command` 只进表头注释。
pub(crate) fn write_bigram(
    pairs: &[(u64, u32)],
    words: &[String],
    path: &Path,
    command: &str,
) -> Result<(), ConvertError> {
    let mut writer = BufWriter::new(File::create(path)?);
    writeln!(
        writer,
        "# 由 glimmer-dict-convert {command} 从语料统计。前词\\t后词\\t计数"
    )?;
    for (key, count) in pairs {
        let first = &words[(key >> 32) as usize];
        let second = &words[(key & 0xFFFF_FFFF) as usize];
        writeln!(writer, "{first}\t{second}\t{count}")?;
    }
    writer.flush()?;
    tracing::info!(path = %path.display(), bigrams = pairs.len(), "二元表已写出");
    Ok(())
}

/// 写三元表：只留计数够、且上文 (前二词, 前词) 自己也在二元表 `kept_bigrams` 里的那些，按计数取前 `max_trigrams` 条。
///
/// 上文不在二元表里的三元在模型里挂不上段（三元 CSR 的段号就是二元的下标），写出来也会被加载时丢掉，
/// 白占几十兆文件；先在这里滤掉。计数下限不低于统计时剪枝抬到的阈值，免得留下的与扔掉的不是一个标准。
/// 返回写出的条数。
pub(crate) fn write_trigram(
    counts: &Counts,
    kept_bigrams: &HashSet<u64>,
    words: &[String],
    min_trigram_count: u32,
    max_trigrams: usize,
    path: &Path,
    command: &str,
) -> Result<usize, ConvertError> {
    let min_count = min_trigram_count.max(counts.threshold() + 1);
    let mut rows: Vec<(u128, u32)> = counts
        .trigram
        .iter()
        .filter(|(_, count)| **count >= min_count)
        .filter(|(key, _)| {
            let (earlier, previous, _) = trigram_parts(**key);
            kept_bigrams.contains(&bigram_key(earlier, previous))
        })
        .map(|(k, c)| (*k, *c))
        .collect();
    rows.sort_unstable_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
    rows.truncate(max_trigrams);
    let mut writer = BufWriter::new(File::create(path)?);
    writeln!(
        writer,
        "# 由 glimmer-dict-convert {command} 从语料统计。前前词\\t前词\\t后词\\t计数"
    )?;
    for (key, count) in &rows {
        let (earlier, previous, word) = trigram_parts(*key);
        writeln!(
            writer,
            "{}\t{}\t{}\t{count}",
            words[earlier as usize], words[previous as usize], words[word as usize]
        )?;
    }
    writer.flush()?;
    tracing::info!(
        path = %path.display(),
        trigrams = rows.len(),
        min_count,
        "三元表已写出"
    );
    Ok(rows.len())
}

/// 写一元表：`词\t计数`，只输出出现过的词。返回写出的词数。
pub(crate) fn write_unigram(
    unigram: &[u64],
    words: &[String],
    path: &Path,
    command: &str,
) -> Result<usize, ConvertError> {
    let mut writer = BufWriter::new(File::create(path)?);
    writeln!(
        writer,
        "# 由 glimmer-dict-convert {command} 从语料统计。词\\t计数；<s> 是句首标记"
    )?;
    let mut written = 0usize;
    for (id, count) in unigram.iter().enumerate() {
        if *count > 0 {
            writeln!(writer, "{}\t{count}", words[id])?;
            written += 1;
        }
    }
    writer.flush()?;
    tracing::info!(unigram = %path.display(), words = written, "一元表已写出");
    Ok(written)
}
