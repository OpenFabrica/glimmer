//! 源表里读到的一个写法，以及同码写法的先后规则。

use std::cmp::Ordering;

/// 一个写法（逐字节相同的词只有一条）。
#[derive(Debug, Clone)]
pub(super) struct Spelling {
    /// 原样写法（`OR`、`we'll`、`TypeScript`）。
    pub(super) word: String,

    /// ESDB 常见度等级（`frequency_level` 列，越小越基础）；CSpell 技术词与没有这一列的文件为 `None`。
    pub(super) level: Option<u32>,

    /// 读入的先后（跨文件累计），前面几条规则分不出时按它。
    pub(super) order: usize,

    /// 语料里这个原样写法出现的次数；只有它所在的「只差大小写」一组按语料重排过时才有值（见 `case` 模块）。
    pub(super) case_count: Option<u64>,
}

impl Spelling {
    /// 带撇号：敲的字母不是它的原样（`we'll` 要敲 `well`）。
    pub(super) fn has_apostrophe(&self) -> bool {
        self.word.contains('\'')
    }

    /// 全小写（普通词形，不是专名 / 缩写）。
    pub(super) fn is_lowercase(&self) -> bool {
        !self.word.bytes().any(|b| b.is_ascii_uppercase())
    }

    /// 小写形式：频率表与语言模型都按它查（`I'm` → `i'm`）。
    pub(super) fn lowercase(&self) -> String {
        self.word.to_ascii_lowercase()
    }

    /// 同码里的先后：不带撇号 → 等级小（没有等级排最后）→ 全小写 → 读入先后。规则与理由见模块文档。
    pub(super) fn precedence(&self, other: &Self) -> Ordering {
        self.has_apostrophe()
            .cmp(&other.has_apostrophe())
            .then_with(|| match (self.level, other.level) {
                (Some(a), Some(b)) => a.cmp(&b),
                (Some(_), None) => Ordering::Less,
                (None, Some(_)) => Ordering::Greater,
                (None, None) => Ordering::Equal,
            })
            .then_with(|| other.is_lowercase().cmp(&self.is_lowercase()))
            .then_with(|| self.order.cmp(&other.order))
    }
}

/// 能收进词表的写法：ASCII 字母，中间可夹单个撇号，首尾是字母。
///
/// 首尾的撇号切词时当边缘标点去掉（`goin'` → `goin`），这种写法永远对不上语料与用户敲的词，不收；
/// 带重音字母、数字、符号的切词时整段是 Gap，也不收。
pub(super) fn is_accepted(word: &str) -> bool {
    let bytes = word.as_bytes();
    let (Some(first), Some(last)) = (bytes.first(), bytes.last()) else {
        return false;
    };
    first.is_ascii_alphabetic()
        && last.is_ascii_alphabetic()
        && bytes.iter().all(|b| b.is_ascii_alphabetic() || *b == b'\'')
        && !word.contains("''")
}
