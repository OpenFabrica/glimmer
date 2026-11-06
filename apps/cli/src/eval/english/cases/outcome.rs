//! 一条错前缀固定用例跑出来的结果。

use super::PrefixCase;

/// 敲完那个串之后期望词排第几、前三是什么。
#[derive(Debug, Clone)]
pub struct CaseOutcome {
    pub case: PrefixCase,

    /// 期望词在候选列表里的下标；不在为 `None`。
    pub rank: Option<usize>,

    /// 当时的前三个候选。
    pub top3: Vec<String>,
}
