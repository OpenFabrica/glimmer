//! 英文下一词预测：词上屏、敲了空格、还没开始下一个词时，按上文猜最可能的下一个词。
//!
//! 做法：候选来自语言模型的后继表——知道前两个词（或前一个词在句首）时取三元后继、再并上前一个词的二元后继，
//! 只知道前一个词时取二元后继；只留能变回随包词表写法的 token（句末标记、词表外的词不要），
//! 再用 [`LanguageModel::log_probs`] 给它们算带回退插值的概率，取最高的几个。第一名不到
//! [`PredictParams::min_probability`] 时整个不出。
//!
//! 常见前词的后继有上万条：三元、二元各按次数只留前 [`SUCCESSOR_LIMIT`] 条再算分（理由与实测见常数的文档）。

mod cache;
mod next_word;
mod params;

pub use cache::PredictionCache;
pub use next_word::NextWord;
pub use params::PredictParams;

use glimmer_dictionary::{WordList, english_code};

use crate::english::{NgramHistory, token_forms};
use crate::sentence::LanguageModel;

/// 三元、二元后继各按次数留前几条再算分。
///
/// 2026-11-07 在 `lm-en.qj` 上量的（release，600 个上文：词表里最常见的 300 个词 + 最常见的 300 条二元当前两个词）：
/// 全量算分时 `the` 的二元后继 10917 条、`of the` 另有三元 2403 条，首次取预测最慢 1.7 ms、九成在 0.66 ms 以内；
/// 各留前 128 条后最慢 0.34 ms、九成在 0.13 ms 以内，600 个上文的前三名与全量一个都不差（64 / 256 也不差）。
/// 剩下的时间几乎都在模型列后继本身（`the` 一行约 0.23 ms），与截多少无关。
pub const SUCCESSOR_LIMIT: usize = 128;

/// `history` 之后最可能的下一个词，概率从高到低，最多 [`PredictParams::count`] 个；
/// 不知道上文、上文没有后继、第一名概率不到下限时为空。
pub fn predict(
    model: &dyn LanguageModel,
    list: &WordList,
    history: NgramHistory<'_>,
    params: &PredictParams,
) -> Vec<NextWord> {
    predict_within(model, list, history, params, SUCCESSOR_LIMIT)
}

/// [`predict`] 的本体：三元、二元后继各按次数只留前 `limit` 条（测试拿它对比截与不截）。
pub(crate) fn predict_within(
    model: &dyn LanguageModel,
    list: &WordList,
    history: NgramHistory<'_>,
    params: &PredictParams,
    limit: usize,
) -> Vec<NextWord> {
    if params.count == 0 {
        return Vec::new();
    }
    // 不在扫的时候查词表：模型的 token 本来就来自词表，查一次要编码加二分，上万条后继时比扫本身还慢；
    // 只给最后留下的几个查写法，查不到（短语层合成的 token 之类）的跳过
    let mut all = |_: &str| true;
    let (trigram, bigram) = match history {
        NgramHistory::Unknown => return Vec::new(),
        NgramHistory::Previous(_) => (Vec::new(), model.successors(history, &mut all)),
        NgramHistory::Context(context) => match context.previous {
            None => (Vec::new(), model.successors(history, &mut all)),
            Some(previous) => (
                model.successors(history, &mut all),
                model.successors(NgramHistory::Previous(previous), &mut all),
            ),
        },
    };
    let mut tokens: Vec<String> = top_by_count(trigram, limit)
        .into_iter()
        .chain(top_by_count(bigram, limit))
        .collect();
    tokens.sort_unstable();
    tokens.dedup();
    let refs: Vec<&str> = tokens.iter().map(String::as_str).collect();
    let scores = model.log_probs(history, &refs);
    let mut scored: Vec<(&str, f64)> = refs
        .iter()
        .zip(scores)
        .filter_map(|(token, score)| Some((*token, score?.exp())))
        .collect();
    scored.sort_by(|a, b| b.1.total_cmp(&a.1).then_with(|| a.0.cmp(b.0)));
    let words: Vec<NextWord> = scored
        .into_iter()
        .filter_map(|(token, probability)| {
            Some(NextWord {
                text: display_form(list, token)?.to_owned(),
                probability,
            })
        })
        .take(params.count)
        .collect();
    if words
        .first()
        .is_none_or(|top| top.probability < params.min_probability)
    {
        return Vec::new();
    }
    words
}

/// 模型 token 在词表里的写法；只差大小写的写法不止一个时取词频最高的（同词频取词表里靠前的）。
fn display_form<'a>(list: &'a WordList, token: &'a str) -> Option<&'a str> {
    let code = english_code(token);
    let mut best: Option<(&str, u32)> = None;
    for form in token_forms(list, &code, token) {
        if best.is_none_or(|(_, frequency)| form.frequency() > frequency) {
            best = Some((form.word(), form.frequency()));
        }
    }
    best.map(|(word, _)| word)
}

/// 次数最高的 `limit` 条的 token（不到就全给，顺序不限）；同次数按 token 定，结果不随模型给的顺序变。
fn top_by_count(mut list: Vec<(String, u32)>, limit: usize) -> Vec<String> {
    if list.len() > limit {
        let order =
            |a: &(String, u32), b: &(String, u32)| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0));
        if limit == 0 {
            return Vec::new();
        }
        list.select_nth_unstable_by(limit - 1, order);
        list.truncate(limit);
    }
    list.into_iter().map(|(token, _)| token).collect()
}

#[cfg(test)]
mod tests;
