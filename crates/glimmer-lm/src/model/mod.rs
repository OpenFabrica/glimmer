//! 词级 n-gram 模型：结构、加载与派生量。打分在 `score`，解析在 `parse`，落盘在 `write`，导出在 `export`。

mod batch;
mod export;
mod parse;
mod score;
mod write;

use std::path::Path;

use glimmer_format::{Container, FormatError, Kind, Metadata, Table, Text, hash};

use crate::error::LmError;
use crate::smoothing::Smoothing;
use crate::successor::Successor;
use crate::word_entry::WordEntry;

/// 句首标记，语料统计时每个句子的第一个词都跟在它后面。
pub const SENTENCE_START: &str = "<s>";

/// 老做法（[`BackoffMode::Legacy`]）里 bigram 部分的权重，其余给一元概率。
///
/// [`BackoffMode::Legacy`]: crate::BackoffMode::Legacy
pub(crate) const LEGACY_LAMBDA: f64 = 0.8;

/// `.qj` 里的分节：词文本 arena、词表、词的哈希索引、二元 CSR 的段偏移与后继。
pub(crate) const WORDS_TAG: [u8; 4] = *b"WORD";
pub(crate) const ENTRIES_TAG: [u8; 4] = *b"ENTR";
pub(crate) const HASH_TAG: [u8; 4] = *b"HASH";
pub(crate) const OFFSETS_TAG: [u8; 4] = *b"OFFS";
pub(crate) const SUCCESSORS_TAG: [u8; 4] = *b"SUCC";

/// 三元 CSR 的段偏移与后继：键是二元在 `SUCC` 里的下标（一条二元 (u,v) 就是一个三元上下文）。
/// 这两节是后加的，没有它们的老文件照样能打开，只是退化成二元。
pub(crate) const TRIGRAM_OFFSETS_TAG: [u8; 4] = *b"TOFF";
pub(crate) const TRIGRAM_SUCCESSORS_TAG: [u8; 4] = *b"TSUC";

/// 回退平滑要的派生量，写进文件免得每次启动扫一遍 SUCC（几百万条，mmap 的好处就没了）：
/// `CSUM` 是每个词作前词时后继计数之和，`CONT` 是每个词的不同前词数 N₁₊(·,w)。老文件没有就加载时算。
pub(crate) const CONTEXT_SUMS_TAG: [u8; 4] = *b"CSUM";
pub(crate) const CONTINUATIONS_TAG: [u8; 4] = *b"CONT";

/// 词级 n-gram 模型（三元 → 二元 → 一元回退）。内存布局就是文件布局：
/// 词表是 arena + 定长条目 + 开放寻址哈希索引（`glimmer_format::hash`），
/// 二元按前词分组成 CSR（前词 `v` 的后继在 `successors[offsets[v]..offsets[v+1]]`，段内按后词编号升序），
/// 三元再按二元分组成一层 CSR（二元 `(u,v)` 在 `successors` 里的下标就是它的段号）。
///
/// 整句转换每一步都要查它：扁平数组二分一次要跳二十来个缓存行，分组之后一段通常就几条，落在一两个缓存行里。
#[derive(Debug, Default)]
pub struct NgramModel {
    /// 所有词首尾相接。
    words: Text,

    /// 词表，下标即编号。
    entries: Table<WordEntry>,

    /// 词 → 编号的哈希索引。
    index: Table<u32>,

    /// 二元 CSR 段偏移，长度 = 词数 + 1。
    offsets: Table<u32>,

    /// 二元 CSR 后继。
    successors: Table<Successor>,

    /// 三元 CSR 段偏移，长度 = 二元条数 + 1；空表示这份模型没有三元。
    trigram_offsets: Table<u32>,

    /// 三元 CSR 后继，段内按后词编号升序。
    trigram_successors: Table<Successor>,

    /// 每个词作前词时后继计数之和 Σ_w c(v,w)，算回退权重的分子用。
    context_sums: Table<u32>,

    /// 每个词的不同前词数 N₁₊(·,w)，接续概率用。
    continuations: Table<u32>,

    /// 一元计数总和（不含句首标记）。
    total: f64,

    /// 接续计数总和 N₁₊(·,·)，就是二元条数。
    continuation_total: f64,

    /// 句首标记的编号。
    start: Option<u32>,

    /// 回退平滑参数。
    smoothing: Smoothing,

    /// `.qj` 里的来历。
    metadata: Option<Metadata>,
}

impl NgramModel {
    /// 打开 `.qj` 语言模型。没有三元分节的老文件照常打开，按二元工作。
    pub fn from_path(path: &Path) -> Result<Self, LmError> {
        let container = Container::open(path, Kind::LanguageModel)?;
        let words = container.text(WORDS_TAG)?;
        let entries: Table<WordEntry> = container.table(ENTRIES_TAG)?;
        let index: Table<u32> = container.table(HASH_TAG)?;
        let offsets: Table<u32> = container.table(OFFSETS_TAG)?;
        let successors: Table<Successor> = container.table(SUCCESSORS_TAG)?;
        let trigram_offsets: Table<u32> =
            optional(container.table(TRIGRAM_OFFSETS_TAG))?.unwrap_or_default();
        let trigram_successors: Table<Successor> =
            optional(container.table(TRIGRAM_SUCCESSORS_TAG))?.unwrap_or_default();
        let context_sums: Table<u32> =
            optional(container.table(CONTEXT_SUMS_TAG))?.unwrap_or_default();
        let continuations: Table<u32> =
            optional(container.table(CONTINUATIONS_TAG))?.unwrap_or_default();
        let count = entries.len();
        for entry in entries.iter() {
            let start = entry.text_start as usize;
            if words
                .get(start..start + usize::from(entry.text_len))
                .is_none()
            {
                return Err(LmError::Corrupt("word entry points outside the arena"));
            }
        }
        if !hash::is_valid(&index, count) {
            return Err(LmError::Corrupt("hash index does not match the word table"));
        }
        if !csr_is_valid(&offsets, count, successors.len())
            || successors.iter().any(|s| s.word as usize >= count)
        {
            return Err(LmError::Corrupt("CSR offsets are inconsistent"));
        }
        if !trigram_offsets.is_empty()
            && (!csr_is_valid(&trigram_offsets, successors.len(), trigram_successors.len())
                || trigram_successors.iter().any(|s| s.word as usize >= count))
        {
            return Err(LmError::Corrupt("trigram CSR offsets are inconsistent"));
        }
        if (!context_sums.is_empty() && context_sums.len() != count)
            || (!continuations.is_empty() && continuations.len() != count)
        {
            return Err(LmError::Corrupt("derived count table has the wrong length"));
        }
        let mut model = Self {
            words,
            entries,
            index,
            offsets,
            successors,
            trigram_offsets,
            trigram_successors,
            context_sums,
            continuations,
            total: 0.0,
            continuation_total: 0.0,
            start: None,
            smoothing: Smoothing::DEFAULT,
            metadata: Some(container.metadata().clone()),
        };
        model.finish();
        tracing::debug!(
            words = model.word_count(),
            bigrams = model.bigram_count(),
            trigrams = model.trigram_count(),
            name = %container.metadata().name,
            "语言模型已映射"
        );
        Ok(model)
    }

    /// 各段数据就位后算派生量：句首编号、总计数，以及老文件缺的两张派生表。
    pub(crate) fn finish(&mut self) {
        self.start = self.word_id(SENTENCE_START);
        self.total = self
            .entries
            .iter()
            .enumerate()
            .filter(|&(id, _)| Some(id as u32) != self.start)
            .map(|(_, e)| f64::from(e.count))
            .sum::<f64>()
            .max(1.0);
        if self.context_sums.len() != self.entries.len() {
            self.context_sums = Table::Owned(self.compute_context_sums());
        }
        if self.continuations.len() != self.entries.len() {
            self.continuations = Table::Owned(self.compute_continuations());
        }
        self.continuation_total = (self.successors.len() as f64).max(1.0);
    }

    /// 每个词作前词时后继计数之和。
    pub(crate) fn compute_context_sums(&self) -> Vec<u32> {
        self.offsets
            .windows(2)
            .map(|range| {
                self.successors[range[0] as usize..range[1] as usize]
                    .iter()
                    .fold(0u32, |sum, s| sum.saturating_add(s.count))
            })
            .collect()
    }

    /// 每个词的不同前词数：CSR 段内后词不重复，所以数一遍以它为后词的条目就是 N₁₊(·,w)。
    pub(crate) fn compute_continuations(&self) -> Vec<u32> {
        let mut counts = vec![0u32; self.entries.len()];
        for successor in self.successors.iter() {
            counts[successor.word as usize] += 1;
        }
        counts
    }

    /// 换一组平滑参数（`glimmer-cli --tune lm-…` 扫参用；引擎与壳用缺省）。
    pub fn set_smoothing(&mut self, smoothing: Smoothing) {
        self.smoothing = smoothing;
    }

    pub fn smoothing(&self) -> Smoothing {
        self.smoothing
    }

    pub fn word_count(&self) -> usize {
        self.entries.len()
    }

    pub fn bigram_count(&self) -> usize {
        self.successors.len()
    }

    pub fn trigram_count(&self) -> usize {
        self.trigram_successors.len()
    }

    /// `.qj` 里的来历；TSV 解析的返回 `None`。
    pub fn metadata(&self) -> Option<&Metadata> {
        self.metadata.as_ref()
    }

    pub(crate) fn word_id(&self, word: &str) -> Option<u32> {
        hash::find(&self.index, word, |id| {
            word_text(&self.words, &self.entries[id as usize])
        })
    }

    /// 二元 (previous, word) 在 `successors` 里的下标与计数。下标同时是它作三元上下文时的段号。
    pub(crate) fn bigram_slot(&self, previous: u32, word: u32) -> Option<(usize, u32)> {
        let start = *self.offsets.get(previous as usize)? as usize;
        let end = *self.offsets.get(previous as usize + 1)? as usize;
        let successors = &self.successors[start..end];
        let index = successors.binary_search_by_key(&word, |s| s.word).ok()?;
        Some((start + index, successors[index].count))
    }
}

/// 缺分节当没有，别的错照报：三元与两张派生表都是后加的。
fn optional<T>(result: Result<Table<T>, FormatError>) -> Result<Option<Table<T>>, LmError> {
    match result {
        Ok(table) => Ok(Some(table)),
        Err(FormatError::MissingSection(_)) => Ok(None),
        Err(error) => Err(error.into()),
    }
}

/// CSR 的偏移表合不合法：长度 = 段数 + 1、单调不减、末位等于后继条数。
fn csr_is_valid(offsets: &[u32], groups: usize, entries: usize) -> bool {
    offsets.len() == groups + 1
        && offsets.windows(2).all(|w| w[0] <= w[1])
        && offsets.last().is_some_and(|&end| end as usize == entries)
}

/// 条目对应的词。
pub(crate) fn word_text<'a>(words: &'a str, entry: &WordEntry) -> &'a str {
    let start = entry.text_start as usize;
    &words[start..start + usize::from(entry.text_len)]
}

/// 给词表建哈希索引。
pub(crate) fn build_index(words: &str, entries: &[WordEntry]) -> Vec<u32> {
    hash::build(entries.len(), |id| word_text(words, &entries[id as usize]))
}

#[cfg(test)]
mod tests;
