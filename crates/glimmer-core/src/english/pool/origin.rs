//! 变体检索时词来自哪张表。

/// 词来自哪张表：只有随包表的词频能拿来在档内排先后，领域表的词要打上领域标记。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Origin {
    /// 随包词表。
    Main,

    /// 领域词库。
    Domain,

    /// 个人英文词表。
    Personal,
}
