//! 检索：把一次查询要打分的词收进候选池，四路各有名额，总数有硬上限。
//!
//! | 来源 | 上限 | 超了怎么截 |
//! | --- | --- | --- |
//! | 正常前缀（随包词表） | [`PREFIX_BUDGET`] | 按词频取（1–2 个字母用加载时算好的前 512） |
//! | 上文的后继 | [`SUCCESSOR_BUDGET`] | 三元后继按次数取前 [`TRIGRAM_BUDGET`]，再用二元后继按次数补满 |
//! | 个人词 + 领域词 | [`PERSONAL_BUDGET`] | 个人词按次数取，领域词按词频补满 |
//! | 错字变体（全部合计） | [`TYPO_BUDGET`] | 按档分名额：误拼表 → 便宜的一处错误 → 其他一处错误 → 两处错误，同一档里按词频取 |
//!
//! 编码正好等于敲的串的词（`well` / `we'll`）不管名额总在池里，精确词规则要用它们。
//! 敲的串没有补全、又有纠错候选时，敲的串本身作为「原样读法」也进池子（见 `gather_literal`）。
//! 前缀里的词不到名额时全部收下，不截。整个池子最多 [`MAX_POOL`] 个词。

mod builder;
mod origin;
mod sources;
mod word;

pub use sources::Sources;
pub use word::PoolWord;

use glimmer_dictionary::{WordEntry, WordList, english_code};

use self::builder::PoolBuilder;
use self::origin::Origin;
use crate::english::{
    CostTier, ScoreParams, misspelling_hits, should_correct, should_try_two_edits, token_forms,
    valid_two_edit_variants, valid_variants,
};

/// 一次查询最多给多少个词打分。
pub const MAX_POOL: usize = 1024;

/// 正常前缀那一路的名额。
pub const PREFIX_BUDGET: usize = 512;

/// 上文后继那一路的名额（三元 + 二元）。
pub const SUCCESSOR_BUDGET: usize = 256;

/// 上文后继里三元先占的名额。
pub const TRIGRAM_BUDGET: usize = 128;

/// 个人词 + 领域词那一路的名额。
pub const PERSONAL_BUDGET: usize = 128;

/// 错字变体那一路的名额（所有变体合计）。
pub const TYPO_BUDGET: usize = 128;

/// 过了词表的敲错变体最多留几个（每个变体再按前缀取词）。
pub const VARIANT_LIMIT: usize = 64;

/// 收集候选池：`typed` 是敲的原串，`code` 是它的检索编码（小写、去撇号）。
/// 顺序固定：精确码 → 正常前缀 → 上文后继 → 个人 / 领域 → 错字变体 → 原样读法。
/// 返回的词还没有模型概率与个人次数（由 [`fill_personal`] 与调用方补）。
pub fn gather<'a>(
    sources: &Sources<'a>,
    typed: &'a str,
    code: &str,
    params: &ScoreParams,
) -> Vec<PoolWord<'a>> {
    let mut pool = PoolBuilder::default();
    gather_exact(&mut pool, sources, code);
    if let Some(main) = sources.main {
        for entry in main.prefix_top(code, PREFIX_BUDGET) {
            pool.add(entry.word(), 1.0, Some(entry.frequency()), false);
        }
    }
    gather_successors(&mut pool, sources, code);
    gather_personal(&mut pool, sources, code);
    gather_typos(&mut pool, sources, code, params);
    gather_literal(&mut pool, sources, typed, code, params);
    let mut words = pool.finish();
    fill_frequency(&mut words, sources.main);
    words
}

/// 原样读法：敲的串不是任何一张词表里任何词的前缀（没有补全）、又有纠错候选时，敲的串本身也进池子，
/// 敲错代价是 [`ScoreParams::literal`]、模型概率按未登录词（词频下限）算。
///
/// 为什么要它：没有它的话，词表里没有的词（人名、新词）整个敲完时第一个候选总是某个纠错，按空格就被换掉；
/// 有了它，纠错只在明显更可能时（常见词、便宜的错）才排到它前面。只在已有纠错候选时加：
/// 没有任何候选时数字键要当标识符的一部分（`xq1`），不能凭空多出一格。
fn gather_literal<'a>(
    pool: &mut PoolBuilder<'a>,
    sources: &Sources<'a>,
    typed: &'a str,
    code: &str,
    params: &ScoreParams,
) {
    let word_like = typed
        .chars()
        .all(|c| c.is_ascii_alphabetic() || c == '\'' || c == '\u{2019}');
    let has_completion = [sources.main, sources.personal, sources.domain]
        .into_iter()
        .flatten()
        .any(|list| !list.prefix_span(code).is_empty());
    if word_like && !has_completion && !pool.is_empty() {
        pool.add_literal(typed, params.literal);
    }
}

/// 各张词表里编码正好是 `code` 的词。
fn gather_exact<'a>(pool: &mut PoolBuilder<'a>, sources: &Sources<'a>, code: &str) {
    if let Some(main) = sources.main {
        for entry in main.get_all(code) {
            pool.add(entry.word(), 1.0, Some(entry.frequency()), false);
        }
    }
    if let Some(domain) = sources.domain {
        for entry in domain.get_all(code) {
            pool.add(entry.word(), 1.0, None, true);
        }
    }
    if let Some(personal) = sources.personal {
        for entry in personal.get_all(code) {
            pool.add(entry.word(), 1.0, None, false);
        }
    }
}

/// 上文的后继：模型的 token 映射回随包词表里同 token 的写法（`i` → `I`）。
fn gather_successors<'a>(pool: &mut PoolBuilder<'a>, sources: &Sources<'a>, code: &str) {
    let (Some(cache), Some(main)) = (sources.successors, sources.main) else {
        return;
    };
    for successor in cache.top(code) {
        for form in token_forms(main, successor.code(), successor.token()) {
            pool.add(form.word(), 1.0, Some(form.frequency()), false);
        }
    }
}

/// 个人词按次数取前几个，领域词按词频补满这一路的名额。
fn gather_personal<'a>(pool: &mut PoolBuilder<'a>, sources: &Sources<'a>, code: &str) {
    let mut budget = PERSONAL_BUDGET;
    if let Some(personal) = sources.personal {
        for entry in top_by_frequency(personal.prefix_range(code), budget) {
            if pool.add(entry.word(), 1.0, None, false) {
                budget -= 1;
            }
        }
    }
    if let Some(domain) = sources.domain {
        for entry in top_by_frequency(domain.prefix_range(code), budget) {
            pool.add(entry.word(), 1.0, None, true);
        }
    }
}

/// 错字变体：误拼表命中最便宜，然后一处错误的变体按档分名额（变体当前缀用），一处错误一个都没有的长串再试两处。
/// 两处也没有时记进 [`Sources::dead_ends`]，同一段组句往后敲不再搜（理由见 [`crate::english::DeadEnds`]）。
fn gather_typos<'a>(
    pool: &mut PoolBuilder<'a>,
    sources: &Sources<'a>,
    code: &str,
    params: &ScoreParams,
) {
    if !should_correct(code) {
        return;
    }
    let mut budget = TYPO_BUDGET;
    if let Some(table) = sources.misspellings {
        for hit in misspelling_hits(table, code, budget) {
            if pool.add(hit.word, params.misspelling, None, false) {
                budget -= 1;
            }
        }
    }
    // 领域与个人词表也参与：拼错的项目名同样能纠回来
    let lists: Vec<(&'a WordList, Origin)> = [
        sources.main.map(|l| (l, Origin::Main)),
        sources.domain.map(|l| (l, Origin::Domain)),
        sources.personal.map(|l| (l, Origin::Personal)),
    ]
    .into_iter()
    .flatten()
    .collect();
    if lists.is_empty() {
        return;
    }
    if sources.dead_ends.is_some_and(|d| d.edits_exhausted(code)) {
        return;
    }
    let plain: Vec<&WordList> = lists.iter().map(|(list, _)| *list).collect();
    let mut variants = valid_variants(code, &plain, VARIANT_LIMIT);
    if should_try_two_edits(code, variants.len()) {
        variants = valid_two_edit_variants(code, &plain, VARIANT_LIMIT);
        if variants.is_empty()
            && let Some(dead_ends) = sources.dead_ends
        {
            dead_ends.mark_edits_exhausted(code);
        }
    }
    for tier in [CostTier::Near, CostTier::Edit, CostTier::TwoEdits] {
        if budget == 0 {
            break;
        }
        // 这一档所有变体下的词合起来按词频排，再按剩下的名额收：(词, 排序用词频, 随包词频, 领域词, 变体正好是整词)
        let mut found: Vec<(&'a str, u32, Option<u32>, bool, bool)> = Vec::new();
        for variant in variants.iter().filter(|v| v.tier == tier) {
            for &(list, origin) in &lists {
                for entry in list.prefix_top(&variant.text, budget) {
                    let main_frequency = (origin == Origin::Main).then(|| entry.frequency());
                    let rank = main_frequency.unwrap_or(0);
                    let whole = entry.code() == variant.text;
                    found.push((
                        entry.word(),
                        rank,
                        main_frequency,
                        origin == Origin::Domain,
                        whole,
                    ));
                }
            }
        }
        found.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(b.0)));
        let cost = params.tier_cost(tier);
        for (word, _, frequency, domain, whole) in found {
            if budget == 0 {
                break;
            }
            // 变体正好是整词的不打折；要再补字母的乘 typo_completion（同一个词从几个变体来时 add 留代价最低的那份）
            let cost = if whole {
                cost
            } else {
                cost * params.typo_completion
            };
            if pool.add(word, cost, frequency, domain) {
                budget -= 1;
            }
        }
    }
}

/// 词条里词频最高的 `limit` 条（不到就全给）。
fn top_by_frequency(entries: &[WordEntry], limit: usize) -> Vec<&WordEntry> {
    let mut sorted: Vec<&WordEntry> = entries.iter().collect();
    sorted.sort_by(|a, b| {
        b.frequency()
            .cmp(&a.frequency())
            .then_with(|| a.word().cmp(b.word()))
    });
    sorted.truncate(limit);
    sorted
}

/// 只从个人 / 领域 / 误拼表来的词，到随包词表里补上词频（它可能也在随包表里，只是没进前缀名额）。
fn fill_frequency(words: &mut [PoolWord<'_>], main: Option<&WordList>) {
    let Some(main) = main else {
        return;
    };
    for word in words.iter_mut().filter(|w| w.frequency.is_none()) {
        word.frequency = main
            .get_all(&english_code(word.word))
            .iter()
            .find(|e| e.word() == word.word)
            .map(WordEntry::frequency);
    }
}

/// 个人英文词表里的次数填进池子；个人表的编码与随包表相同（[`english_code`]），只差大小写的写法共用一个次数。
pub fn fill_personal(words: &mut [PoolWord<'_>], personal: Option<&WordList>) {
    let Some(personal) = personal else {
        return;
    };
    for word in words.iter_mut() {
        word.personal = personal
            .get_all(&english_code(word.word))
            .iter()
            .find(|e| e.word().eq_ignore_ascii_case(word.word))
            .map_or(0, WordEntry::frequency);
    }
}

#[cfg(test)]
mod tests;
