//! 注好错的一个词。

use super::TypoKind;

/// 一个词注了一处错之后的样子。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Typo {
    /// 模拟用户实际敲的串。
    pub typed: String,

    /// 用的哪种错法。
    pub kind: TypoKind,

    /// 敲的串与原词的公共前缀长度（字符数）：敲到第 `common + 1` 个字母时才错，退格也退到这里。
    pub common: usize,
}
