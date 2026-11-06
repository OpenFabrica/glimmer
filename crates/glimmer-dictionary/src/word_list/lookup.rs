//! `WordList` 的查询：精确词、同码全部词、前缀区间、区间词频和、按词频取前几、补全。

use std::collections::HashSet;
use std::ops::Range;

use super::short_top::{select_top, span_of};
use super::{WordEntry, WordList};

impl WordList {
    /// 输入（已小写）对应的英文词；同码多词时是文件里最先出现的那个。
    pub fn get(&self, code: &str) -> Option<&str> {
        self.get_all(code).first().map(WordEntry::word)
    }

    /// 编码正好等于 `code` 的全部词，按文件里出现的先后；没有为空切片。
    pub fn get_all(&self, code: &str) -> &[WordEntry] {
        let start = self.entries.partition_point(|e| e.code.as_str() < code);
        let len = self.entries[start..].partition_point(|e| e.code == code);
        &self.entries[start..start + len]
    }

    /// 输入（已小写）对应的词频（同码多词时取 [`WordList::get`] 那个的）；词表里没有为 `None`。
    pub fn frequency(&self, code: &str) -> Option<u32> {
        self.get_all(code).first().map(WordEntry::frequency)
    }

    /// 以 `prefix` 开头（不含正好相等的）的词里词频最高的 `limit` 个，按词频降序。
    pub fn complete(&self, prefix: &str, limit: usize) -> Vec<&str> {
        if prefix.is_empty() || limit == 0 {
            return Vec::new();
        }
        let mut hits: Vec<&WordEntry> = self
            .prefix_range(prefix)
            .iter()
            .filter(|e| e.code != prefix)
            .collect();
        hits.sort_by(|a, b| {
            b.frequency
                .cmp(&a.frequency)
                .then_with(|| a.code.cmp(&b.code))
        });
        let mut seen = HashSet::new();
        hits.into_iter()
            .map(WordEntry::word)
            .filter(|word| seen.insert(*word))
            .take(limit)
            .collect()
    }

    /// 编码以 `prefix` 开头的词目在 [`WordList::as_slice`] 里的下标区间，**含**编码正好等于 `prefix` 的；
    /// 空前缀是整张表。两次二分，不分配。
    pub fn prefix_span(&self, prefix: &str) -> Range<usize> {
        span_of(&self.entries, prefix.as_bytes())
    }

    /// 编码以 `prefix` 开头的词目（含正好相等的），按编码字节序；不排序、不分配。
    pub fn prefix_range(&self, prefix: &str) -> &[WordEntry] {
        &self.entries[self.prefix_span(prefix)]
    }

    /// `prefix_range(prefix)` 里全部词目的词频之和，查前缀和表得到。
    pub fn prefix_mass(&self, prefix: &str) -> u64 {
        self.span_mass(self.prefix_span(prefix))
    }

    /// 下标区间 `span`（来自 [`WordList::prefix_span`]）的词频之和，O(1)。
    pub fn span_mass(&self, span: Range<usize>) -> u64 {
        if span.is_empty() {
            return 0;
        }
        self.mass[span.end] - self.mass[span.start]
    }

    /// `text` 最长的、仍是某个编码前缀的开头有几个字节：`text` 本身是某个编码的前缀时就是 `text.len()`，空表为 0。
    ///
    /// 有序表里与 `text` 公共前缀最长的编码一定是它插入位置左右两个邻居之一，所以一次二分就够。
    pub fn matched_prefix_len(&self, text: &str) -> usize {
        let at = self.entries.partition_point(|e| e.code.as_str() < text);
        let common = |e: &WordEntry| {
            e.code
                .bytes()
                .zip(text.bytes())
                .take_while(|(a, b)| a == b)
                .count()
        };
        let before = at.checked_sub(1).map_or(0, |i| common(&self.entries[i]));
        let after = self.entries.get(at).map_or(0, common);
        before.max(after)
    }

    /// 编码以 `prefix` 开头、比它长的那些，紧跟在 `prefix` 后面的字节有哪些（去重，升序）：`prefix` 后能接什么。
    ///
    /// 区间里按下一个字节分块，块是连续的，每块一次二分跳过去，代价是「不同的下一个字节数」次二分，不扫区间。
    pub fn next_bytes(&self, prefix: &str) -> Vec<u8> {
        let depth = prefix.len();
        let span = self.prefix_span(prefix);
        let block = &self.entries[span];
        // 正好等于 prefix 的排在区间最前面
        let mut at = block.partition_point(|e| e.code.len() == depth);
        let mut out = Vec::new();
        while let Some(entry) = block.get(at) {
            let byte = entry.code.as_bytes()[depth];
            out.push(byte);
            at += block[at..].partition_point(|e| e.code.as_bytes()[depth] == byte);
        }
        out
    }

    /// 编码以 `prefix` 开头（含正好相等的）的词目里词频最高的 `limit` 条：词频降序，同词频按编码、再按文件先后。
    ///
    /// 1–2 个小写字母的前缀、`limit` 不超过 [`super::SHORT_PREFIX_TOP`] 时直接取加载时算好的结果；
    /// 其余现挑（先挑出前 `limit` 再排，不排整个区间）。
    pub fn prefix_top(&self, prefix: &str, limit: usize) -> Vec<&WordEntry> {
        if limit == 0 {
            return Vec::new();
        }
        let span = self.prefix_span(prefix);
        // 预计算的槽最多存 SHORT_PREFIX_TOP 条：要的不多于它、或区间本来就全在里面，才能直接用
        if let Some(top) = self.short.get(prefix)
            && (limit <= top.len() || top.len() == span.len())
        {
            return top
                .iter()
                .take(limit)
                .map(|&i| &self.entries[i as usize])
                .collect();
        }
        let mut indices: Vec<u32> = span.map(|i| i as u32).collect();
        select_top(&self.entries, &mut indices, limit);
        indices
            .into_iter()
            .map(|i| &self.entries[i as usize])
            .collect()
    }

    /// 全部词目，按编码字节序（同码按文件先后）；下标与 [`WordList::prefix_span`] 一致。
    pub fn as_slice(&self) -> &[WordEntry] {
        &self.entries
    }

    /// 全部词目：(小写编码, 原样词, 词频)，按编码字节序。
    pub fn entries(&self) -> impl Iterator<Item = (&str, &str, u32)> {
        self.entries
            .iter()
            .map(|e| (e.code.as_str(), e.word.as_str(), e.frequency))
    }

    /// 词目条数（同码多词各算一条）。
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// 是否一条都没有。
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}
