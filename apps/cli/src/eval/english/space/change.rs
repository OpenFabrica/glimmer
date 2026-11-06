//! 按空格之后上屏的字和敲的词比，变成了什么样。

/// 一个词按空格上屏后的变化。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Change {
    /// 原样上屏。
    Same,

    /// 只改了大小写（`or → OR`、`i → I`）：忽略 ASCII 大小写后相同。
    CaseOnly,

    /// 换成了别的词（被补全或纠错替换）。
    Replaced,
}

impl Change {
    /// 比较敲的 `typed` 与上屏的 `committed`。
    pub fn of(typed: &str, committed: &str) -> Self {
        if typed == committed {
            Self::Same
        } else if typed.eq_ignore_ascii_case(committed) {
            Self::CaseOnly
        } else {
            Self::Replaced
        }
    }

    /// 「小写词被改成大写」：敲的全是小写、上屏的含大写字母（只改大小写的一种）。
    pub fn upcased(typed: &str, committed: &str) -> bool {
        Self::of(typed, committed) == Self::CaseOnly
            && !typed.chars().any(|c| c.is_ascii_uppercase())
            && committed.chars().any(|c| c.is_ascii_uppercase())
    }
}
