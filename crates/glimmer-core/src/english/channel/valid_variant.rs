//! 过了词表、可以拿去检索的敲错变体。

use super::CostTier;

/// 至少是一张词表里某个编码前缀的变体，带这个前缀下的词频总和，供检索按名额取。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidVariant {
    /// 变体串（小写字母），拿它当前缀去 [`glimmer_dictionary::WordList::prefix_range`] 取词。
    pub text: String,

    /// 代价档。
    pub tier: CostTier,

    /// 各词表里以它开头的词的词频之和（[`glimmer_dictionary::WordList::prefix_mass`] 相加）。
    /// 词频是各表自己的单位（随包表是 Zipf × 1000），只用来在同档里排先后。
    pub mass: u64,
}
