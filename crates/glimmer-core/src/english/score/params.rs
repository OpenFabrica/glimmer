//! 英文候选打分的常数，缺省值见 [`ScoreParams::DEFAULT`]；`glimmer-cli --tune en-…` 扫参时整组换掉。

use crate::english::CostTier;

/// 打分公式（见 [`super`] 的模块文档）里的全部常数。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ScoreParams {
    /// 误拼表命中的敲错代价（先验概率）。
    pub misspelling: f64,

    /// 便宜的一处错误（邻键、对调、双写多按少按）的敲错代价。
    pub near: f64,

    /// 其他一处错误的敲错代价。
    pub edit: f64,

    /// 两处错误的敲错代价。
    pub two_edits: f64,

    /// 原样读法（敲的串本身当未登录词，只在它没有任何补全时进池）的代价；正常补全是 1。
    pub literal: f64,

    /// 纠错后还要再补字母的词额外乘的折扣（变体正好是整词的不打）：`recieve` 纠成 receive 是 1，纠成 received 再乘它。
    /// 用户已经敲了和那个整词一样长的字母，敲完了的可能性比「敲错了而且还没敲完」大。
    pub typo_completion: f64,

    /// 个人份额混合权重 λ 的上限。
    pub lambda_cap: f64,

    /// λ = 个人总次数 ÷ (个人总次数 + 这个数)：总次数到这个数时 λ 是 0.5（再被上限截住）。
    pub lambda_half: f64,

    /// 个人份额分母上加的常数 K：池子里只有一个个人词时它也拿不满 1。
    pub personal_k: f64,

    /// 领域词库里的词加的分（nat）。
    pub domain_bonus: f64,

    /// 词频换算概率时的 Zipf 下限：没有词频（或低于它）的词按它算。
    pub zipf_floor: f64,
}

impl ScoreParams {
    /// 缺省值。
    ///
    /// 敲错代价：计划给定一处编辑 0.003（`ln 0.003 ≈ −5.8` nat）；邻键 / 对调这类物理上容易犯的错大约多三倍，取 0.01；
    /// 误拼表是人真写错的词形，比任何猜测都可靠，取 0.05；两处错误按「两次一处编辑」量级取 0.0001。
    /// 领域加分 ln 100 ≈ 4.6：Zipf 3 的领域名称压得过 Zipf 5 以下的通用词，与改之前「开了领域词库补全优先领域名称」的效果一致。
    ///
    /// 原样读法 0.01、纠错再补字母的折扣 0.1 是在调参集上扫出来的（原样读法 0.1 / 0.03 / 0.01 / 0.003 / 0.001 / 0 ×
    /// 折扣 1 / 0.3 / 0.1 / 0.03）：原样读法的模型概率按词频下限算、不看上文，取 0.1 时它常压过上文里概率更低的正确纠错，
    /// 整词敲错的首选比改之前低；0.01 加上折扣 0.1，三类文本错字组的首选都回到改之前以上、前三也升，
    /// 错前缀组略好，代价是词表里没有的词按空格被换掉的多了约三成。
    pub const DEFAULT: Self = Self {
        misspelling: 0.05,
        near: 0.01,
        edit: 0.003,
        two_edits: 0.0001,
        literal: 0.01,
        typo_completion: 0.1,
        lambda_cap: 0.3,
        lambda_half: 50.0,
        personal_k: 2.0,
        domain_bonus: 4.605_170_185_988_091,
        zipf_floor: 3.0,
    };

    /// 一个代价档的敲错代价。
    pub fn tier_cost(&self, tier: CostTier) -> f64 {
        match tier {
            CostTier::Misspelling => self.misspelling,
            CostTier::Near => self.near,
            CostTier::Edit => self.edit,
            CostTier::TwoEdits => self.two_edits,
        }
    }
}

impl Default for ScoreParams {
    fn default() -> Self {
        Self::DEFAULT
    }
}
