//! 打分：三元 → 二元 → 一元的回退。

use glimmer_core::english::NgramHistory;
use glimmer_core::sentence::{Context, LanguageModel};

use crate::smoothing::BackoffMode;

use super::{LEGACY_LAMBDA, NgramModel};

impl LanguageModel for NgramModel {
    fn log_prob(&self, context: Context<'_>, word: &str) -> Option<f64> {
        let id = self.word_id(word)?;
        let probability = match self.smoothing.mode {
            BackoffMode::Legacy => self.legacy_probability(context, id),
            _ => self.backoff_probability(context, id),
        };
        Some(probability.max(f64::MIN_POSITIVE).ln())
    }

    /// 上文的量只求一次，见 `batch.rs`。
    fn log_probs(&self, history: NgramHistory<'_>, words: &[&str]) -> Vec<Option<f64>> {
        self.batch_log_probs(history, words)
    }

    /// 扫上文那一层的后继段，见 `batch.rs`。
    fn successors(
        &self,
        history: NgramHistory<'_>,
        keep: &mut dyn FnMut(&str) -> bool,
    ) -> Vec<(String, u32)> {
        self.listed_successors(history, keep)
    }
}

impl NgramModel {
    /// 三元 → 二元 → 一元：每层先看这层见没见过这条接续，没见过（或这层的上文本身没见过）就把整份质量交给下一层。
    fn backoff_probability(&self, context: Context<'_>, word: u32) -> f64 {
        let unigram = self.unigram_probability(word);
        let Some(previous) = self.context_id(context.previous) else {
            return unigram;
        };
        let bigram = self.bigram_probability(previous, word, unigram);
        if !self.smoothing.use_trigram || self.trigram_offsets.is_empty() {
            return bigram;
        }
        // 三元上下文 (u, v) 必须自己是一条二元，否则没有段可查；句首词没有前二词
        let Some(earlier) = self.earlier_id(context) else {
            return bigram;
        };
        let Some((pair, pair_count)) = self.bigram_slot(earlier, previous) else {
            return bigram;
        };
        self.trigram_probability(pair, pair_count, word, bigram)
    }

    /// 一元：缺省是 c(w)/N，[`BackoffMode::Continuation`] 换成接续概率 N₁₊(·,w)/N₁₊(·,·)。
    /// 接续计数加一再算：剪枝后的表里有些词（短语层合成进来的 token）一次也没当过后词，不能给它概率 0。
    pub(super) fn unigram_probability(&self, word: u32) -> f64 {
        if self.smoothing.mode == BackoffMode::Continuation {
            let continuations = f64::from(self.continuations[word as usize]) + 1.0;
            return continuations / (self.continuation_total + self.entries.len() as f64);
        }
        f64::from(self.entries[word as usize].count) / self.total
    }

    /// 二元：P(w|v) = max(c(v,w) − D₂, 0)/c(v) + (1 − 让出去的质量)·P₁(w)。
    ///
    /// 分母用语料里真实的 c(v)（一元计数）而不是表里后继计数之和：二元表按次数砍过（只留前几百万条），
    /// 用和当分母会把留下来的那几条的概率抬高十几倍，剪枝反而变成了「见过就一定对」。
    fn bigram_probability(&self, previous: u32, word: u32, unigram: f64) -> f64 {
        let count = self
            .bigram_slot(previous, word)
            .map_or(0.0, |(_, count)| f64::from(count));
        let start = self.offsets[previous as usize] as usize;
        let end = self.offsets[previous as usize + 1] as usize;
        let distinct = (end - start) as f64;
        let sum = f64::from(self.context_sums[previous as usize]);
        let total = f64::from(self.entries[previous as usize].count)
            .max(sum)
            .max(1.0);
        mix(
            count,
            sum,
            distinct,
            total,
            self.smoothing.bigram_discount,
            unigram,
        )
    }

    /// 三元：同二元的式子，上文是二元 `pair`（计数 `pair_count`），回退到 `bigram`。
    fn trigram_probability(&self, pair: usize, pair_count: u32, word: u32, bigram: f64) -> f64 {
        let start = self.trigram_offsets[pair] as usize;
        let end = self.trigram_offsets[pair + 1] as usize;
        if start == end {
            return bigram;
        }
        let row = &self.trigram_successors[start..end];
        let count = row
            .binary_search_by_key(&word, |s| s.word)
            .map_or(0.0, |index| f64::from(row[index].count));
        // 一段三元通常只有几条，直接加一遍比另存一张总计表划算（另存要多 4 字节 × 二元条数）
        let sum = row.iter().fold(0.0, |sum, s| sum + f64::from(s.count));
        let total = f64::from(pair_count).max(sum).max(1.0);
        mix(
            count,
            sum,
            row.len() as f64,
            total,
            self.smoothing.trigram_discount,
            bigram,
        )
    }

    /// 改成三元之前的打分：固定 λ 的插值，只看前一个词。
    pub(super) fn legacy_probability(&self, context: Context<'_>, word: u32) -> f64 {
        let unigram = f64::from(self.entries[word as usize].count) / self.total;
        match self.context_id(context.previous) {
            Some(previous) if self.entries[previous as usize].count > 0 => {
                let pair = self
                    .bigram_slot(previous, word)
                    .map_or(0.0, |(_, count)| f64::from(count));
                LEGACY_LAMBDA * pair / f64::from(self.entries[previous as usize].count)
                    + (1.0 - LEGACY_LAMBDA) * unigram
            }
            // 前词不在模型里：只剩一元概率
            _ => unigram,
        }
    }

    /// 前一个词的编号；句首用句首标记。模型不认识这个词时返回 `None`（只能退到一元）。
    pub(super) fn context_id(&self, previous: Option<&str>) -> Option<u32> {
        match previous {
            None => self.start,
            Some(text) => self.word_id(text),
        }
    }

    /// 再前一个词的编号：前一个词在句首（或调用方只知道一个词）时用句首标记，
    /// 与语料统计时每句前面加 `<s>` 的做法对齐；这个词本身在句首时没有前二词。
    pub(super) fn earlier_id(&self, context: Context<'_>) -> Option<u32> {
        context.previous?;
        match context.earlier {
            None => self.start,
            Some(text) => self.word_id(text),
        }
    }
}

/// 一层回退：`count` 是这条接续见过几次，`sum` / `distinct` 是这个上文全部接续的计数和与条数，
/// `total` 是上文自己的计数，`discount` 是绝对折扣 D，`lower` 是下一层的概率。
///
/// 每条见过的接续让出 D 份，让出去的与没分配的（剪枝砍掉的、语料里根本没接过的）合起来按 `lower` 分。
pub(super) fn mix(
    count: f64,
    sum: f64,
    distinct: f64,
    total: f64,
    discount: f64,
    lower: f64,
) -> f64 {
    let seen = (count - discount).max(0.0) / total;
    let taken = ((sum - discount * distinct).max(0.0) / total).clamp(0.0, 1.0);
    seen + (1.0 - taken) * lower
}
