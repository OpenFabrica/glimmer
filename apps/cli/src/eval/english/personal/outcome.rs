//! 一条个人词回归用例跑出来的结果。

use super::PersonalCase;

/// 过没过，以及每个前缀上个人词与对手各排第几。
#[derive(Debug, Clone)]
pub struct PersonalOutcome {
    pub case: PersonalCase,

    /// 实际造了几条背景记录（词表太小时可能不到要的数）。
    pub background: usize,

    /// 个人词里从候选里选中的次数；其余是整词敲完原样上屏的次数。
    pub selected: usize,

    /// 每个前缀：(前缀, 个人词下标, 对手下标)。
    pub ranks: Vec<(String, Option<usize>, Option<usize>)>,

    pub passed: bool,
}
