//! 英文候选的统一打分：补全、纠错、个人词、领域词放进同一个候选池，用一个分数排。
//!
//! 所有项先统一成概率，最后取一次对数（单位 nat）：
//!
//! ```text
//! 权重(词)   = P_模型(词 | 上文) · 敲错代价(词)
//! 模型份额   = 权重(词) ÷ 候选池里所有词的权重之和
//! 个人份额   = max(次数 − 1, 0) · 敲错代价(词) ÷ (候选池里各词的这一项之和 + K)
//! λ          = min(λ上限, 个人总次数 ÷ (个人总次数 + λ半值))
//! P_词       = (1 − λ) · 模型份额(词) + λ · 个人份额(词)
//! 分数       = ln P_词 + 领域加分(是领域词时)
//! ```
//!
//! - `P_模型`：英文语言模型给的 P(token | 上文)，token 是词转小写（保留撇号）；同一个 token 在池里有几种写法时
//!   （`us` / `US`）按各写法的词频分这份概率。模型不认识的词、没装模型时，用词频换算 `10^(Zipf − 9)`
//!   （词表词频是 Zipf × 1000，Zipf 最低按 [`ScoreParams::zipf_floor`]），量纲与模型概率相同。
//! - 敲错代价：补全是 1；误拼表、便宜的一处错误、其他一处错误、两处错误各一个先验（[`ScoreParams::tier_cost`]）；
//!   敲的串没有任何补全时，它本身作为原样读法进池，代价 [`ScoreParams::literal`]、模型概率按未登录词（词频下限）。
//!   **先乘代价再归一化**：几乎不可能的纠错候选权重极小，加进池子也几乎不改变别的词的份额。
//! - 个人份额只在这一次的候选池里算：只选过一次的词是 0（误选一次不起作用），份额只和同前缀的词比，不被总次数稀释。
//! - λ 封顶：个人词最多拿 λ上限 的概率。模型对另一个词很有把握（份额超过约 λ上限 ÷ (1 − λ上限)）时个人词排不到它前面，
//!   这是有意的：人名、项目名用几次就到第一，`the` 这类词仍由模型定。
//! - 领域加分是固定的 nat 数，开着领域词库时领域名称优先于同前缀的通用词。

mod params;

pub use params::ScoreParams;

use crate::english::pool::PoolWord;

/// 词频（Zipf × 1000）换算成概率 `10^(Zipf − 9)`，Zipf 低于 `floor`（或没有词频）按 `floor`。
pub fn zipf_probability(frequency: Option<u32>, floor: f64) -> f64 {
    let zipf = frequency.map_or(floor, |f| (f64::from(f) / 1000.0).max(floor));
    10f64.powf(zipf - 9.0)
}

/// 个人份额的混合权重 λ。
pub fn personal_lambda(total: f64, params: &ScoreParams) -> f64 {
    if total <= 0.0 {
        return 0.0;
    }
    (total / (total + params.lambda_half)).min(params.lambda_cap)
}

/// 给候选池里的每个词打分（与 `pool` 一一对应），式子见模块文档。`personal_total` 是个人英文词的总次数。
pub fn score(pool: &[PoolWord<'_>], personal_total: f64, params: &ScoreParams) -> Vec<f64> {
    let weights: Vec<f64> = pool.iter().map(|w| w.model * w.cost).collect();
    let weight_sum: f64 = weights.iter().sum();
    let personal: Vec<f64> = pool
        .iter()
        .map(|w| f64::from(w.personal.saturating_sub(1)) * w.cost)
        .collect();
    let personal_sum: f64 = personal.iter().sum();
    let lambda = personal_lambda(personal_total, params);
    pool.iter()
        .zip(weights.iter().zip(&personal))
        .map(|(word, (&weight, &own))| {
            let model_share = if weight_sum > 0.0 {
                weight / weight_sum
            } else {
                0.0
            };
            let personal_share = own / (personal_sum + params.personal_k);
            let probability = (1.0 - lambda) * model_share + lambda * personal_share;
            let bonus = if word.domain {
                params.domain_bonus
            } else {
                0.0
            };
            probability.max(f64::MIN_POSITIVE).ln() + bonus
        })
        .collect()
}

#[cfg(test)]
mod tests;
