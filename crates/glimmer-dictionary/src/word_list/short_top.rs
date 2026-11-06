//! 1 个与 2 个字母前缀的「词频前 N」下标，加载时算好。

use super::WordEntry;

/// 每种短前缀预先留多少条（按词频取前几）。
pub const SHORT_PREFIX_TOP: usize = 512;

/// 26 个单字母前缀 + 676 个双字母前缀。
const SLOTS: usize = 26 + 26 * 26;

/// 短前缀（1–2 个小写字母，共 702 种）下词频最高的条目下标。
///
/// 为什么要预先算：敲第一、二个字母时前缀区间有几千条（`s` 约 1 万），每键现排序太费；
/// 3 个字母起区间一般只有几百条，现扫就够，不预先算。
#[derive(Debug, Default)]
pub(super) struct ShortTop {
    /// 按槽位存下标，每槽按（词频降序、下标升序）排好，最多 [`SHORT_PREFIX_TOP`] 条；空表时整个为空。
    slots: Vec<Box<[u32]>>,
}

impl ShortTop {
    /// 对按编码排好的词目算出全部 702 个槽。
    ///
    /// 每个单字母前缀与双字母前缀的区间各扫一遍，所以总共只把全表扫两遍；
    /// 每槽用 `select_nth_unstable` 先挑出前 N 再排这 N 条，不对整个区间排序。
    pub(super) fn build(entries: &[WordEntry]) -> Self {
        if entries.is_empty() {
            return Self::default();
        }
        let mut slots: Vec<Box<[u32]>> = Vec::with_capacity(SLOTS);
        for slot in 0..SLOTS {
            let prefix = slot_prefix(slot);
            let span = span_of(entries, prefix.as_bytes());
            slots.push(top_of(entries, span));
        }
        Self { slots }
    }

    /// 前缀是 1–2 个小写字母时给出该槽的下标（已排好）；其余前缀或空表为 `None`。
    pub(super) fn get(&self, prefix: &str) -> Option<&[u32]> {
        let slot = slot_of(prefix.as_bytes())?;
        self.slots.get(slot).map(|top| &top[..])
    }
}

/// 前缀 → 槽位：`a`–`z` 是 0–25，`aa`–`zz` 是 26 起。
fn slot_of(prefix: &[u8]) -> Option<usize> {
    let letter = |b: u8| b.is_ascii_lowercase().then(|| usize::from(b - b'a'));
    match prefix {
        [a] => letter(*a),
        [a, b] => Some(26 + letter(*a)? * 26 + letter(*b)?),
        _ => None,
    }
}

/// 槽位 → 前缀，`slot_of` 的反函数。
fn slot_prefix(slot: usize) -> String {
    let letter = |i: usize| char::from(b'a' + i as u8);
    if slot < 26 {
        letter(slot).to_string()
    } else {
        let rest = slot - 26;
        [letter(rest / 26), letter(rest % 26)].iter().collect()
    }
}

/// 按编码排好的词目里以 `prefix` 开头的下标区间。
pub(super) fn span_of(entries: &[WordEntry], prefix: &[u8]) -> std::ops::Range<usize> {
    let start = entries.partition_point(|e| e.code.as_bytes() < prefix);
    let len = entries[start..].partition_point(|e| e.code.as_bytes().starts_with(prefix));
    start..start + len
}

/// 区间里词频前 [`SHORT_PREFIX_TOP`] 的下标，按（词频降序、下标升序）排。
fn top_of(entries: &[WordEntry], span: std::ops::Range<usize>) -> Box<[u32]> {
    let mut indices: Vec<u32> = span.map(|i| i as u32).collect();
    select_top(entries, &mut indices, SHORT_PREFIX_TOP);
    indices.into_boxed_slice()
}

/// 把 `indices` 截成词频最高的 `limit` 个并排好：词频降序，同词频按下标（即编码、再文件顺序）升序。
pub(super) fn select_top(entries: &[WordEntry], indices: &mut Vec<u32>, limit: usize) {
    let key = |&i: &u32| (std::cmp::Reverse(entries[i as usize].frequency), i);
    if limit == 0 {
        indices.clear();
        return;
    }
    if indices.len() > limit {
        indices.select_nth_unstable_by_key(limit - 1, key);
        indices.truncate(limit);
    }
    indices.sort_unstable_by_key(key);
}
