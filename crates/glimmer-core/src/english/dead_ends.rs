//! 同一段组句里已知走不通的英文编码：往后多敲字母也不会变好，后面的键直接跳过那部分检索。
//!
//! 记两种：
//! - 一个候选都没有（[`crate::english::stays_empty`] 成立时）：整个查询跳过，各长度段的理由见那里。
//! - 纠错变体一个都没有：真的试过两处错误也没有结果，只在 8 到 [`crate::english::MAX_TWO_EDIT_LETTERS`] 个字母时记。
//!   这时一处错误没结果，敲的串本身也不是任何词的前缀（是的话删掉最后一个字母就是一处错误的结果）；
//!   更长的串的任一个至多两处错误的变体，取它对应原串的那段开头，就是原串至多两处错误的变体、又是词的前缀，
//!   与「都没有」矛盾。所以往后敲（含加长到 13 个字母以上、本来就不试两处的那些键），补全、一处、两处错误都没有，
//!   一处与两处错误的搜索可以不做，结果不变。误拼表是按前缀查的另一路，照查。
//!   13 个字母以上一处没结果时不试两处，不知道两处有没有，不记这一条；整个查询都空时按上一条记。
//!
//! 只在同一段组句、同一组词表下成立：新一段组句、换词库时 [`DeadEnds::clear`]。

use std::cell::RefCell;

/// 两种走不通的编码，各记最近的一个（只会越敲越长，记一个就够）。
#[derive(Debug, Default)]
pub struct DeadEnds {
    /// 一个候选都没有的编码。
    empty: RefCell<Option<String>>,

    /// 纠错变体一个都没有的编码。
    edits: RefCell<Option<String>>,
}

impl DeadEnds {
    /// `code` 是不是已知「一个候选都没有」的编码本身或它的延长。
    pub fn is_empty(&self, code: &str) -> bool {
        covers(&self.empty, code)
    }

    /// 记下一个候选都没有、往后也不会有的编码。
    pub fn mark_empty(&self, code: &str) {
        self.empty.replace(Some(code.to_owned()));
    }

    /// `code` 是不是已知「纠错变体一个都没有」的编码本身或它的延长。
    pub fn edits_exhausted(&self, code: &str) -> bool {
        covers(&self.edits, code)
    }

    /// 记下试过两处错误也没有变体的编码。
    pub fn mark_edits_exhausted(&self, code: &str) {
        self.edits.replace(Some(code.to_owned()));
    }

    /// 两种记录都清掉。
    pub fn clear(&self) {
        self.empty.replace(None);
        self.edits.replace(None);
    }
}

/// 记着的编码是 `code` 的前缀（含相等）。
fn covers(slot: &RefCell<Option<String>>, code: &str) -> bool {
    slot.borrow()
        .as_deref()
        .is_some_and(|dead| code.starts_with(dead))
}
