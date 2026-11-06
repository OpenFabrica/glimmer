//! 英文词表的一条词目：检索编码、原样写法、词频。

/// 英文词表里的一条：编码只管检索，词本身才是身份（`well` 与 `we'll` 同码 `well`，是两条）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WordEntry {
    /// 检索编码：小写、去掉撇号（见 [`crate::english_code`]），文件里给了编码列就照用。
    pub(super) code: String,

    /// 原样写法（`iPhone`、`don't`、`C++`）。
    pub(super) word: String,

    /// 词频：随包词表是 Zipf × 1000，个人词表是选过的次数；只在同一张表里比大小。
    pub(super) frequency: u32,
}

impl WordEntry {
    /// 检索编码。
    pub fn code(&self) -> &str {
        &self.code
    }

    /// 原样写法。
    pub fn word(&self) -> &str {
        &self.word
    }

    /// 词频。
    pub fn frequency(&self) -> u32 {
        self.frequency
    }
}
