use super::Context;
use crate::english::NgramHistory;

/// 整句转换的打分来源：词级语言模型。实现放兄弟 crate（`glimmer-lm`），Core 只认这个 trait。
/// 英文模式的候选排序也用它（另装一个英文实例），多出来的两个方法都有缺省实现，中文路径不受影响。
pub trait LanguageModel: Send {
    /// `log P(word | context)`；`context` 给前一个词与再前一个词（[`Context::START`] 是句首）。
    /// 模型不认识 `word` 时返回 `None`，由 Core 用词库词频兜底。
    ///
    /// 实现可以只看 `context.previous`（二元），看前二词的（三元）自己按上下文回退。
    fn log_prob(&self, context: Context<'_>, word: &str) -> Option<f64>;

    /// 同一个上文下一批词的 `log P(词 | 上文)`，与 `words` 一一对应；模型不认识的词为 `None`。
    ///
    /// 实现应把上文相关的量（后继段、计数总和）只求一次，再逐词查；[`NgramHistory::Unknown`] 只用一元概率，
    /// [`NgramHistory::Previous`] 只走二元、不补句首标记。缺省实现逐个调 [`Self::log_prob`]（不知道上文按句首近似）。
    fn log_probs(&self, history: NgramHistory<'_>, words: &[&str]) -> Vec<Option<f64>> {
        let context = history.approximate_context();
        words
            .iter()
            .map(|word| self.log_prob(context, word))
            .collect()
    }

    /// `history` 指到的那一层后继表里、`keep(词)` 为真的后继：(词, 这条接续的次数)，顺序不限。
    ///
    /// - `Context` 且前一个词有值：三元表（上文是再前一个词或句首标记、加前一个词），不回退到二元；
    /// - [`NgramHistory::START`]：句首标记的二元后继；
    /// - [`NgramHistory::Previous`]：前一个词的二元后继；
    /// - [`NgramHistory::Unknown`]：空。
    ///
    /// 检索「上文里很常见、词频却排不进前几百」的词用。缺省实现为空：没有后继表的模型不出这一路候选。
    fn successors(
        &self,
        _history: NgramHistory<'_>,
        _keep: &mut dyn FnMut(&str) -> bool,
    ) -> Vec<(String, u32)> {
        Vec::new()
    }
}

/// 没接语言模型：一律兜底，整句转换退化为一元词频。
#[derive(Debug, Default, Clone, Copy)]
pub struct NoLanguageModel;

impl LanguageModel for NoLanguageModel {
    fn log_prob(&self, _context: Context<'_>, _word: &str) -> Option<f64> {
        None
    }
}
