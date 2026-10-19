use super::{
    BIGRAM_DISCOUNT, CONFIDENCE_K, INITIAL_CONTEXT_WEIGHT, MAX_CONFIDENCE, READING_CAP,
    READING_WEIGHT, SPAN_CANDIDATES, TRIGRAM_DISCOUNT, USER_LAMBDA, WHOLE_WORD_MARGIN,
    WORD_PENALTY,
};
use crate::ranking::{CHOICE_BONUS, CHOICE_CAP};

/// 个人 n-gram 与静态模型插值的参数（见 [`UserNgram::blend`]），外加整句路径打分的两个常数（开头接上文的权重、每词代价）、
/// 整句让整词的门槛与词级排序里同输入串选择加分的两个常数。缺省值是 `sentence` 与 `ranking` 两个模块里的常数，
/// 回放调参（`glimmer-cli --tune`）时可以整组换掉，引擎与壳只用缺省值。
///
/// [`UserNgram::blend`]: super::UserNgram::blend
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Interpolation {
    /// 个人概率里 bigram 部分的权重，其余给个人一元（[`USER_LAMBDA`]）。
    pub lambda: f64,

    /// 插值权重 μ = c(v)/(c(v)+K) 里的 K（[`CONFIDENCE_K`]）。
    pub confidence_k: f64,

    /// 插值权重的封顶（[`MAX_CONFIDENCE`]）。
    pub max_confidence: f64,

    /// 个人二元的绝对折扣 D₂（[`BIGRAM_DISCOUNT`]）。
    pub bigram_discount: f64,

    /// 个人三元的绝对折扣 D₃（[`TRIGRAM_DISCOUNT`]）。
    pub trigram_discount: f64,

    /// 整句第一个词按上文算的成分（[`INITIAL_CONTEXT_WEIGHT`]），0 为一律当句首。
    pub initial_weight: f64,

    /// 整句路径上每个词扣的分（[`WORD_PENALTY`]）。
    pub word_penalty: f64,

    /// 整句领先整词不到这么多分时整词排第一（[`WHOLE_WORD_MARGIN`]），0 为不让。
    pub whole_word_margin: f64,

    /// 词图每个格子最多留几个词（[`SPAN_CANDIDATES`]）。放宽它是打分模型可信度的验收尺子，所以挂在这里能调。
    pub span_candidates: usize,

    /// 多音字读音份额扣分的系数（[`READING_WEIGHT`]），0 为关掉。
    pub reading_weight: f64,

    /// 读音份额最多扣多少（[`READING_CAP`]）。
    pub reading_cap: f64,

    /// 词级排序里同输入串、同位置选过次数的加分系数（[`CHOICE_BONUS`]）。
    ///
    /// [`CHOICE_BONUS`]: crate::ranking::CHOICE_BONUS
    pub choice_bonus: f64,

    /// 上一项加分里计入的次数上限（[`CHOICE_CAP`]）。
    ///
    /// [`CHOICE_CAP`]: crate::ranking::CHOICE_CAP
    pub choice_cap: u32,
}

impl Interpolation {
    /// 现在的常数。
    pub const DEFAULT: Self = Self {
        lambda: USER_LAMBDA,
        confidence_k: CONFIDENCE_K,
        max_confidence: MAX_CONFIDENCE,
        bigram_discount: BIGRAM_DISCOUNT,
        trigram_discount: TRIGRAM_DISCOUNT,
        initial_weight: INITIAL_CONTEXT_WEIGHT,
        word_penalty: WORD_PENALTY,
        whole_word_margin: WHOLE_WORD_MARGIN,
        span_candidates: SPAN_CANDIDATES,
        reading_weight: READING_WEIGHT,
        reading_cap: READING_CAP,
        choice_bonus: CHOICE_BONUS,
        choice_cap: CHOICE_CAP,
    };
}

impl Default for Interpolation {
    fn default() -> Self {
        Self::DEFAULT
    }
}
