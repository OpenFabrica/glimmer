//! 英文模式要的两件事：同一上文一批词的打分（上文的量只求一次），按上文列后继。
//!
//! 打分式子与 `score.rs` 的逐词打分完全一样（有测试对拍），只是把「上文这一侧」的量提前算好；
//! 多出来的是能明说「不知道上文」（[`NgramHistory::Unknown`]，一元用最大似然 c(w)/N）与
//! 「只知道前一个词」（[`NgramHistory::Previous`]，只走二元、不补句首标记）。

mod scope;

use glimmer_core::english::NgramHistory;
use glimmer_core::sentence::Context;

use self::scope::{BigramScope, HistoryScope, TrigramScope};
use super::score::mix;
use super::{NgramModel, SENTENCE_START, word_text};
use crate::smoothing::BackoffMode;

impl NgramModel {
    /// 一批词在同一个上文下的 `ln P`，与 `words` 一一对应；模型不认识的词为 `None`。
    pub(crate) fn batch_log_probs(
        &self,
        history: NgramHistory<'_>,
        words: &[&str],
    ) -> Vec<Option<f64>> {
        // 老做法只看前一个词、固定 λ，没有可提前算的量：不知道上文以外逐词算
        if self.smoothing.mode == BackoffMode::Legacy && history != NgramHistory::Unknown {
            let context = history.approximate_context();
            return words
                .iter()
                .map(|word| {
                    let id = self.word_id(word)?;
                    Some(ln(self.legacy_probability(context, id)))
                })
                .collect();
        }
        let scope = self.scope(history);
        words
            .iter()
            .map(|word| {
                let id = self.word_id(word)?;
                Some(ln(self.scoped_probability(&scope, id)))
            })
            .collect()
    }

    /// `history` 指到的那一层后继里 `keep(词)` 为真的：(词, 次数)，按词编号的顺序。规则见 trait 的文档。
    pub(crate) fn listed_successors(
        &self,
        history: NgramHistory<'_>,
        keep: &mut dyn FnMut(&str) -> bool,
    ) -> Vec<(String, u32)> {
        let row = match history {
            NgramHistory::Unknown => return Vec::new(),
            NgramHistory::Previous(previous) => match self.word_id(previous) {
                Some(id) => self.bigram_row(id),
                None => return Vec::new(),
            },
            NgramHistory::Context(context) if context.previous.is_none() => match self.start {
                Some(start) => self.bigram_row(start),
                None => return Vec::new(),
            },
            NgramHistory::Context(context) => match self.trigram_range(context) {
                Some((start, end)) => &self.trigram_successors[start..end],
                None => return Vec::new(),
            },
        };
        row.iter()
            .filter(|s| Some(s.word) != self.start)
            .filter_map(|s| {
                let text = word_text(&self.words, &self.entries[s.word as usize]);
                (text != SENTENCE_START && keep(text)).then(|| (text.to_owned(), s.count))
            })
            .collect()
    }

    /// 上文的各层量；对应 `score.rs` 的 `backoff_probability` 里与后词无关的那一半。
    fn scope(&self, history: NgramHistory<'_>) -> HistoryScope {
        let context = match history {
            NgramHistory::Unknown => {
                return HistoryScope {
                    unknown: true,
                    ..HistoryScope::default()
                };
            }
            NgramHistory::Previous(previous) => {
                return HistoryScope {
                    bigram: self.word_id(previous).map(|id| self.bigram_scope(id)),
                    ..HistoryScope::default()
                };
            }
            NgramHistory::Context(context) => context,
        };
        let Some(previous) = self.context_id(context.previous) else {
            return HistoryScope::default();
        };
        let trigram = if self.smoothing.use_trigram && !self.trigram_offsets.is_empty() {
            self.trigram_scope(context)
        } else {
            None
        };
        HistoryScope {
            unknown: false,
            bigram: Some(self.bigram_scope(previous)),
            trigram,
        }
    }

    /// 前一个词的二元量。
    fn bigram_scope(&self, previous: u32) -> BigramScope {
        let row = self.bigram_row(previous);
        let sum = f64::from(self.context_sums[previous as usize]);
        BigramScope {
            previous,
            sum,
            distinct: row.len() as f64,
            total: f64::from(self.entries[previous as usize].count)
                .max(sum)
                .max(1.0),
        }
    }

    /// 前两个词的三元量：上文二元不存在、或它的三元段是空的为 `None`（整段退回二元）。
    fn trigram_scope(&self, context: Context<'_>) -> Option<TrigramScope> {
        let earlier = self.earlier_id(context)?;
        let previous = self.context_id(context.previous)?;
        let (pair, pair_count) = self.bigram_slot(earlier, previous)?;
        let start = self.trigram_offsets[pair] as usize;
        let end = self.trigram_offsets[pair + 1] as usize;
        if start == end {
            return None;
        }
        let sum = self.trigram_successors[start..end]
            .iter()
            .fold(0.0, |sum, s| sum + f64::from(s.count));
        Some(TrigramScope {
            start,
            end,
            sum,
            total: f64::from(pair_count).max(sum).max(1.0),
        })
    }

    /// 上文 `context`（前一个词有值）在三元 CSR 里的段。
    fn trigram_range(&self, context: Context<'_>) -> Option<(usize, usize)> {
        if self.trigram_offsets.is_empty() {
            return None;
        }
        let earlier = self.earlier_id(context)?;
        let previous = self.context_id(context.previous)?;
        let (pair, _) = self.bigram_slot(earlier, previous)?;
        Some((
            self.trigram_offsets[pair] as usize,
            self.trigram_offsets[pair + 1] as usize,
        ))
    }

    /// 前词编号的二元后继段。
    fn bigram_row(&self, previous: u32) -> &[super::Successor] {
        let start = self.offsets[previous as usize] as usize;
        let end = self.offsets[previous as usize + 1] as usize;
        &self.successors[start..end]
    }

    /// 用提前算好的上文量求 P(词 | 上文)。
    fn scoped_probability(&self, scope: &HistoryScope, word: u32) -> f64 {
        if scope.unknown {
            return f64::from(self.entries[word as usize].count) / self.total;
        }
        let unigram = self.unigram_probability(word);
        let Some(bigram_scope) = scope.bigram else {
            return unigram;
        };
        let count = self
            .bigram_slot(bigram_scope.previous, word)
            .map_or(0.0, |(_, count)| f64::from(count));
        let bigram = mix(
            count,
            bigram_scope.sum,
            bigram_scope.distinct,
            bigram_scope.total,
            self.smoothing.bigram_discount,
            unigram,
        );
        let Some(trigram_scope) = scope.trigram else {
            return bigram;
        };
        let row = &self.trigram_successors[trigram_scope.start..trigram_scope.end];
        let count = row
            .binary_search_by_key(&word, |s| s.word)
            .map_or(0.0, |index| f64::from(row[index].count));
        mix(
            count,
            trigram_scope.sum,
            row.len() as f64,
            trigram_scope.total,
            self.smoothing.trigram_discount,
            bigram,
        )
    }
}

/// 概率取对数，下限是最小正数（与逐词打分一致，不出 `-inf`）。
fn ln(probability: f64) -> f64 {
    probability.max(f64::MIN_POSITIVE).ln()
}

#[cfg(test)]
mod tests;
