//! 误拼表的一条命中。

/// 已敲的串在误拼表里的一条命中：给出的是整词（不是前缀），档是 [`crate::english::CostTier::Misspelling`]。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MisspellingHit<'a> {
    /// 正确写法（照误拼表，可能带大写：`Aborigine`）。
    pub word: &'a str,

    /// 命中的误拼（`recieve`）。
    pub typo: &'a str,

    /// 已敲的串正好就是这个误拼；`false` 表示只是它的开头（`recie` 之于 `recieve`），误拼还没敲完。
    pub exact: bool,
}
