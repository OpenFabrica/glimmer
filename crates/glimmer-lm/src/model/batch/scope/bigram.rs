//! 二元一层的上文量。

/// 前一个词定下的二元量，见 `NgramModel::bigram_probability` 的式子。
#[derive(Debug, Clone, Copy)]
pub(in crate::model::batch) struct BigramScope {
    /// 前一个词（或句首标记）的编号。
    pub(in crate::model::batch) previous: u32,

    /// 前一个词全部后继的计数和 Σ_w c(v,w)。
    pub(in crate::model::batch) sum: f64,

    /// 不同后继条数。
    pub(in crate::model::batch) distinct: f64,

    /// 分母：前一个词的一元计数（不小于 `sum`、不小于 1）。
    pub(in crate::model::batch) total: f64,
}
