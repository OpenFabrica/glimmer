//! 候选池里的一个词：检索时定下来源与敲错代价，打分前补上模型概率。

/// 候选池里的一个词（按词本身去重，同一个词从几路检索到时留最便宜的敲错代价）。
#[derive(Debug, Clone, PartialEq)]
pub struct PoolWord<'a> {
    /// 原样写法（`don't`、`iPhone`、`Spring Boot`）。
    pub word: &'a str,

    /// 敲错代价：以敲的串开头的补全是 1，纠错候选是它那一档的先验（见 [`super::super::ScoreParams`]）。
    pub cost: f64,

    /// 随包词表里的词频（Zipf × 1000）；不在随包词表里为 `None`。
    pub frequency: Option<u32>,

    /// 个人英文词表里的次数（选过、回车原样上屏过几次），没有为 0。
    pub personal: u32,

    /// 来自领域词库（开着领域词库时加分）。
    pub domain: bool,

    /// 原样读法：敲的串本身（没有补全时作为未登录词进池），不参加精确词规则。
    pub literal: bool,

    /// `P_模型(词 | 上文)`：打分前由调用方填好（模型概率按写法分过，或词频换算）。
    pub model: f64,
}
