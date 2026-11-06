//! 三元一层的上文量。

/// 前两个词定下的三元量，见 `NgramModel::trigram_probability` 的式子。
#[derive(Debug, Clone, Copy)]
pub(in crate::model::batch) struct TrigramScope {
    /// 段在 `trigram_successors` 里的起点。
    pub(in crate::model::batch) start: usize,

    /// 段终点（不含）。
    pub(in crate::model::batch) end: usize,

    /// 段里计数和：现有的逐词打分每次重算它，批量时只算一次。
    pub(in crate::model::batch) sum: f64,

    /// 分母：上文这条二元的计数（不小于 `sum`、不小于 1）。
    pub(in crate::model::batch) total: f64,
}
