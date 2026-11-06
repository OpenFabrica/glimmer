//! 英文上文的三种状态。

use super::Earlier;

/// 本地记下的英文上文。
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) enum ContextState {
    /// 不知道：刚聚焦、刚动过光标、退格删过已上屏的字、停手太久、上屏的不是认识的英文词。打分只用一元。
    #[default]
    Unknown,

    /// 句首：敲了句末标点或回车。按句首算分。
    SentenceStart,

    /// 记着一到两个词。
    Words {
        /// 前一个词之前是什么。
        earlier: Earlier,

        /// 紧挨着的前一个词（模型 token：小写、留撇号）。
        previous: String,
    },
}
