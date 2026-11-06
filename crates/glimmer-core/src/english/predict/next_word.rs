//! 预测出来的一个下一词。

/// 一个下一词：词表写法与它在上文下的概率。
#[derive(Debug, Clone, PartialEq)]
pub struct NextWord {
    /// 随包词表里的写法（`I`，不是模型 token `i`）。
    pub text: String,

    /// P(词 | 上文)，语言模型带回退插值的概率，0–1。
    pub probability: f64,
}
