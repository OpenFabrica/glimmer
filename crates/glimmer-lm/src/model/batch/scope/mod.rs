//! 一个上文下打分要用的量：各层的编号、后继段与计数总和，批量打分时只求一次。

mod bigram;
mod trigram;

pub(super) use bigram::BigramScope;
pub(super) use trigram::TrigramScope;

/// 上文定下来的打分环境（见 `NgramModel::scope`）。
#[derive(Debug, Clone, Copy, Default)]
pub(super) struct HistoryScope {
    /// 不知道上文：一元用最大似然 c(w)/N。
    pub(super) unknown: bool,

    /// 二元那层；`None` 时只有一元（前一个词模型不认识，或不知道上文）。
    pub(super) bigram: Option<BigramScope>,

    /// 三元那层；`None` 时到二元为止。
    pub(super) trigram: Option<TrigramScope>,
}
