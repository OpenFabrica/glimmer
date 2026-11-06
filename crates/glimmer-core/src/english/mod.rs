//! 英文：英文模式的候选，以及给它用的积木（切词、键位、敲错变体、误拼表、上文、检索、打分）。
//!
//! 英文模式是单击 Shift（mac 也可用 Caps Lock）切出来的持久模式：敲的字母不当拼音，
//! 候选来自英文词表的补全、敲错纠正（噪声信道：变体当前缀查，误拼表命中最便宜）与个人词，
//! 在一个候选池里按「语言模型分 × 敲错代价 + 封顶的个人份额」统一排（[`score`]）。
//! 选词与中文一致：空格选高亮、数字选当前页第 N 个；回车与标点原样上屏敲的字母（词表里没有的写法靠回车），
//! 见 `docs/design/candidate-ui.md`「英文模式候选」。

mod channel;
mod dead_ends;
mod edit;
mod history;
mod keyboard;
mod misspelling;
pub mod pool;
pub mod predict;
pub mod score;
mod successors;
mod suggest;
mod token;
mod vocabulary;

pub use channel::{
    CostTier, MAX_TWO_EDIT_LETTERS, MIN_CORRECTION_LETTERS, MIN_TWO_EDIT_LETTERS, ValidVariant,
    Variant, one_edit_variants, should_correct, should_try_two_edits, two_edit_variants,
    valid_two_edit_variants, valid_variants,
};
pub use dead_ends::DeadEnds;
pub use edit::within_one_edit;
pub use history::NgramHistory;
pub use keyboard::{is_adjacent, neighbors};
pub use misspelling::{MisspellingHit, misspelling_hits};
pub use pool::Sources;
pub use predict::{NextWord, PredictParams, PredictionCache, predict};
pub use score::ScoreParams;
pub use successors::{SuccessorCache, SuccessorEntry};
pub use suggest::{stays_empty, suggest};
pub use token::{Token, Tokens, char_break, english_code, tokenize, word_token};
pub use vocabulary::token_forms;

/// 词表里的写法按敲的大小写调整：全大写就全大写，首字母大写就首字母大写，其余照词表（iPhone 仍是 iPhone）。
pub fn adapt_case(word: &str, typed: &str) -> String {
    let mut chars = typed.chars();
    let first_upper = chars.next().is_some_and(|c| c.is_ascii_uppercase());
    let rest_upper = typed.len() >= 2 && typed.chars().all(|c| !c.is_ascii_lowercase());
    if first_upper && rest_upper {
        word.to_ascii_uppercase()
    } else if first_upper {
        let mut out = String::with_capacity(word.len());
        let mut word_chars = word.chars();
        if let Some(c) = word_chars.next() {
            out.extend(c.to_uppercase());
        }
        out.push_str(word_chars.as_str());
        out
    } else {
        word.to_owned()
    }
}

#[cfg(test)]
pub(crate) mod test_model;

#[cfg(test)]
mod tests;
