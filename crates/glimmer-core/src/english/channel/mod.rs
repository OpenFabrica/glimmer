//! 噪声信道纠错的「敲错模型」：把已敲的串变成「本来想敲的」前缀（变体），每个变体带代价档。
//!
//! 方向：用户敲的是错的，变体是他想要的。变体当**前缀**用，所以词没敲完也能纠（`tomm` → `tomo` → tomorrow）。
//! 用法：先 [`should_correct`] 判断该不该纠；[`valid_variants`] 生成、过词表、限额一步做完；
//! 一处错误没结果、且 [`should_try_two_edits`] 时再调 [`valid_two_edit_variants`]。
//! 误拼表是另一路（[`crate::english::misspelling_hits`]），档是 [`CostTier::Misspelling`]。

mod generate;
mod tier;
mod valid;
mod valid_variant;
mod variant;

pub use generate::{one_edit_variants, two_edit_variants};
pub use tier::CostTier;
pub use valid::{valid_two_edit_variants, valid_variants};
pub use valid_variant::ValidVariant;
pub use variant::Variant;

/// 拼错纠正至少要几个字母：三个字母以内一处编辑能变出太多词，只补全不纠正。
pub const MIN_CORRECTION_LETTERS: usize = 4;

/// 两处错误至少要几个字母：短词两处编辑几乎能变成任何词。
pub const MIN_TWO_EDIT_LETTERS: usize = 8;

/// 两处错误最多试到几个字母：每次约 1–1.4 ms，一长串一处错误都纠不回的字母从第 8 个起每键都要付。
/// 调参集上去掉 13 个字母以上的两处错误，各项命中率一个都不变，长错串每键 p95 从约 1 ms 降到 0.1 ms 以下。
pub const MAX_TWO_EDIT_LETTERS: usize = 12;

/// 这个检索编码（已小写）该不该纠错：至少 [`MIN_CORRECTION_LETTERS`] 个字母，且只有小写字母。
/// 带数字或符号的（`foo1`、`x_y`）是标识符不是拼错的词，不去猜；大写字母说明没转成编码，也不纠。
pub fn should_correct(code: &str) -> bool {
    code.len() >= MIN_CORRECTION_LETTERS && generate::letters_only(code)
}

/// 一处错误的变体过完词表后剩 `one_edit_found` 个时，要不要再试两处错误：
/// [`MIN_TWO_EDIT_LETTERS`] 到 [`MAX_TWO_EDIT_LETTERS`] 个字母、该纠、且一处错误一个都没留下。
pub fn should_try_two_edits(code: &str, one_edit_found: usize) -> bool {
    one_edit_found == 0
        && (MIN_TWO_EDIT_LETTERS..=MAX_TWO_EDIT_LETTERS).contains(&code.len())
        && should_correct(code)
}

#[cfg(test)]
mod tests;
