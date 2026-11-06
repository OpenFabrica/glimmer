//! 前一个词之前是什么。

/// 前一个词之前的情形：决定只知道一个词时走二元还是补句首标记走三元。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Earlier {
    /// 不知道（前一个词之前上文断过）：只走二元。
    Unknown,

    /// 前一个词在句首：补句首标记走三元，与训练时每句前面加 `<s>` 一致。
    Start,

    /// 再前一个词（模型 token）。
    Word(String),
}
