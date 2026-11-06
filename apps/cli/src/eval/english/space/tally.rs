//! 一类词（词表里有 / 词表里没有）按空格被改的计数。

use super::Change;

/// 总数，以及被改的里头「只改大小写」与「换成别的词」各几个。
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct SpaceTally {
    /// 这一类评了几个词。
    pub total: usize,

    /// 只改了大小写的。
    pub case_only: usize,

    /// 换成了别的词的。
    pub replaced: usize,
}

impl SpaceTally {
    /// 记一个词的变化。
    pub fn add(&mut self, change: Change) {
        self.total += 1;
        match change {
            Change::Same => {}
            Change::CaseOnly => self.case_only += 1,
            Change::Replaced => self.replaced += 1,
        }
    }

    /// 被改了的（两种合计）。
    pub fn changed(&self) -> usize {
        self.case_only + self.replaced
    }
}
