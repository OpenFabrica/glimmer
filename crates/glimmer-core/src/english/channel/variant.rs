//! 生成出来、还没查词表的敲错变体。

use super::CostTier;

/// 一个变体：用户「本来想敲的」前缀，以及从已敲的串变成它要犯什么档的错。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Variant {
    /// 变体串（小写字母），当**前缀**用：`tomm` 的变体 `tomo` 能补出 tomorrow。
    pub text: String,

    /// 代价档；同一个串能由几种错误得到时取最便宜的。
    pub tier: CostTier,
}
