//! 上文后继的缓存：敲一个词的过程中上文不变，第一键扫一次模型的后继段，后面每一键只在缓存里取。
//!
//! 为什么要缓存：常见前词（`the`、`I`）的后继有几万条，后继段按词编号排、不按次数排，
//! 只能「扫一遍 → 留前缀对得上的 → 按次数取前 N」。第一键扫的时候只留首字母对得上的，按编码排好，
//! 之后敲的前缀都以这个字母开头，两次二分就能定出区间，再在区间里按次数挑前几个。

mod entry;

pub use entry::SuccessorEntry;

use std::collections::HashSet;

use glimmer_dictionary::english_code;

use crate::english::NgramHistory;
use crate::english::pool::{SUCCESSOR_BUDGET, TRIGRAM_BUDGET};
use crate::sentence::LanguageModel;

/// 某个上文、某个首字母下的后继（三元、二元两张），按编码排好。
#[derive(Debug, Default)]
pub struct SuccessorCache {
    /// 缓存的是哪个上文、哪个首字母；`None` 是空缓存。
    key: Option<(String, u8)>,

    /// 前两个词的三元后继。
    trigram: Vec<SuccessorEntry>,

    /// 前一个词（或句首标记）的二元后继。
    bigram: Vec<SuccessorEntry>,
}

impl SuccessorCache {
    /// 准备好 `history` 下、编码以 `code` 首字母开头的后继；上文与首字母都没变时什么都不做。
    pub fn prepare(&mut self, model: &dyn LanguageModel, history: NgramHistory<'_>, code: &str) {
        let Some(&first) = code.as_bytes().first() else {
            return;
        };
        let key = (history.cache_key(), first);
        if self.key.as_ref() == Some(&key) {
            return;
        }
        // token 首字符一定是字母（撇号是词内的），按首字节比就是按编码首字母比
        let mut keep = |token: &str| token.as_bytes().first() == Some(&first);
        let (trigram, bigram) = match history {
            NgramHistory::Unknown => (Vec::new(), Vec::new()),
            NgramHistory::Previous(_) => (Vec::new(), model.successors(history, &mut keep)),
            NgramHistory::Context(context) => match context.previous {
                None => (Vec::new(), model.successors(history, &mut keep)),
                Some(previous) => (
                    model.successors(history, &mut keep),
                    model.successors(NgramHistory::Previous(previous), &mut keep),
                ),
            },
        };
        self.trigram = entries(trigram);
        self.bigram = entries(bigram);
        self.key = Some(key);
    }

    /// 编码以 `prefix` 开头的后继：三元按次数取前 [`TRIGRAM_BUDGET`]，再用二元按次数补到 [`SUCCESSOR_BUDGET`]（同 token 只算一次）。
    /// `prefix` 的首字母与 [`Self::prepare`] 时不同（缓存不是为它准备的）时为空。
    pub fn top(&self, prefix: &str) -> Vec<&SuccessorEntry> {
        let Some((_, first)) = &self.key else {
            return Vec::new();
        };
        if prefix.as_bytes().first() != Some(first) {
            return Vec::new();
        }
        let mut out = top_by_count(range(&self.trigram, prefix), TRIGRAM_BUDGET);
        let rest = SUCCESSOR_BUDGET - out.len();
        let taken: HashSet<&str> = out.iter().map(|entry| entry.token.as_str()).collect();
        let bigram = range(&self.bigram, prefix)
            .iter()
            .filter(|entry| !taken.contains(entry.token.as_str()));
        out.extend(top_by_count(bigram, rest));
        out
    }

    /// 清空（换了模型时）。
    pub fn clear(&mut self) {
        *self = Self::default();
    }
}

/// 模型给的 (token, 次数) → 按编码排好的缓存条目。
fn entries(list: Vec<(String, u32)>) -> Vec<SuccessorEntry> {
    let mut out: Vec<SuccessorEntry> = list
        .into_iter()
        .map(|(token, count)| SuccessorEntry {
            code: english_code(&token),
            token,
            count,
        })
        .collect();
    out.sort_by(|a, b| a.code.cmp(&b.code).then_with(|| a.token.cmp(&b.token)));
    out
}

/// 按编码排好的条目里编码以 `prefix` 开头的那段。
fn range<'a>(entries: &'a [SuccessorEntry], prefix: &str) -> &'a [SuccessorEntry] {
    let start = entries.partition_point(|e| e.code.as_str() < prefix);
    let len = entries[start..].partition_point(|e| e.code.starts_with(prefix));
    &entries[start..start + len]
}

/// 次数最高的 `limit` 条，次数降序、同次数按编码。
fn top_by_count<'a>(
    entries: impl IntoIterator<Item = &'a SuccessorEntry>,
    limit: usize,
) -> Vec<&'a SuccessorEntry> {
    let mut list: Vec<&SuccessorEntry> = entries.into_iter().collect();
    let order = |a: &&SuccessorEntry, b: &&SuccessorEntry| {
        b.count
            .cmp(&a.count)
            .then_with(|| a.code.cmp(&b.code))
            .then_with(|| a.token.cmp(&b.token))
    };
    if limit == 0 {
        return Vec::new();
    }
    if list.len() > limit {
        list.select_nth_unstable_by(limit - 1, order);
        list.truncate(limit);
    }
    list.sort_by(order);
    list
}

#[cfg(test)]
mod tests;
