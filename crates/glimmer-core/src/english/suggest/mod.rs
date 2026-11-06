//! 英文模式的候选：检索 → 按词去重 → 打分 → 排序 → 精确词规则 → 大小写 → 截断。

use std::borrow::Cow;
use std::collections::HashMap;

use glimmer_dictionary::english_code;

use super::adapt_case;
use crate::english::MIN_TWO_EDIT_LETTERS;
use crate::english::pool::{PoolWord, Sources, fill_personal, gather};
use crate::english::score::{ScoreParams, score, zipf_probability};

/// 按敲的字母给英文候选，最多 `limit` 条。打分见 [`crate::english::score`]，检索名额见 [`crate::english::pool`]。
///
/// 精确词规则（打分之后再排，保证正常敲字不会被空格改掉）：
/// - 敲的字母**原样**（区分大小写）正好是词表里的一个写法：这个写法第一（`or` 第一、`OR` 在后；`well` 第一、`we'll` 在后）；
///   它不是随包词表里「只差大小写」那一组的缺省写法时，缺省写法紧跟着排第二（`id` → id、ID；`api` → api、API），
///   见 [`promote_default_form`]；没有原样的，按敲的大小写调整后正好是敲的那串的同码写法第一（句首敲 `Or` 是 or 调成的 Or，不是 OR）。
/// - 不是，但有编码相同的词（`i` 只有 `I`，`dont` 只有 `don't`，`api` 只有 `API`）：同码的词排最前；几个之间先是
///   只差大小写的（`walter's` → Walter's 先于 Walters），再按分数。
/// - 其余按分数。敲的含大写时整串按 [`adapt_case`] 调大小写（`Compa` → Company，`COMPA` → COMPANY）。
pub fn suggest(
    sources: &Sources<'_>,
    typed: &str,
    params: &ScoreParams,
    limit: usize,
) -> Vec<String> {
    let code = english_code(typed);
    if code.is_empty() || limit == 0 {
        return Vec::new();
    }
    let mut pool = gather(sources, typed, &code, params);
    if pool.is_empty() {
        return Vec::new();
    }
    fill_personal(&mut pool, sources.personal);
    fill_model(&mut pool, sources, params);
    let personal_total = sources.personal.map_or(0.0, |p| p.prefix_mass("") as f64);
    let scores = score(&pool, personal_total, params);
    let mut order: Vec<usize> = (0..pool.len()).collect();
    // 同分（常见于都按词频下限算的冷僻词）时按词表里的原始词频，再按字节序，结果与检索先后无关
    order.sort_by(|&a, &b| {
        scores[b]
            .total_cmp(&scores[a])
            .then_with(|| pool[b].frequency.cmp(&pool[a].frequency))
            .then_with(|| pool[a].word.cmp(pool[b].word))
    });
    // 精确写法提到最前，其余照分数；没有精确写法时同码的几个稳定地挪到最前，
    // 同码里先是按敲的大小写调整后正好是敲的那串（句首 `Or` 是 or 调成的，不是 OR），再是只差大小写的
    // （`walter's` 是 Walter's，不是同码的 Walters），最后按分数
    match order
        .iter()
        .position(|&i| !pool[i].literal && pool[i].word == typed)
    {
        Some(at) => {
            let exact = order.remove(at);
            order.insert(0, exact);
            promote_default_form(&mut order, &pool, sources, typed, &code);
        }
        None => order.sort_by_key(|&i| {
            let word = pool[i].word;
            if pool[i].literal || !same_code(word, &code) {
                3
            } else if adapt_case(word, typed) == typed {
                0
            } else if word.eq_ignore_ascii_case(typed) {
                1
            } else {
                2
            }
        }),
    }
    let mut out: Vec<String> = Vec::with_capacity(limit);
    for index in order {
        if out.len() >= limit {
            break;
        }
        let word = adapt_case(pool[index].word, typed);
        if !out.contains(&word) {
            out.push(word);
        }
    }
    out
}

/// 缺省写法提到第二位（[`promote_default_form`]）至少要几个字母的编码。
///
/// 单字母的大写写法（F、W、C）基本是字母名或缩写，用户真要它的时候很少，却在几乎每个词的第一键占掉一个候选位：
/// 调参集上单字母占了这条规则挤掉常用词（`f` 挤掉 from、`w` 挤掉 was）的约九成，前三省键率掉约 1 个点。
pub const MIN_PROMOTED_DEFAULT_LETTERS: usize = 2;

/// 敲的原样是一个写法、却不是随包词表里与它「只差大小写」那一组的缺省写法（组里排第一的）时，把缺省写法挪到第二位。
/// 编码短于 [`MIN_PROMOTED_DEFAULT_LETTERS`] 的不挪。
///
/// 缺省写法是按规范书面语料里的实际次数定的：敲的不是它，说明另一个写法更常用，用户多半是懒得按 Shift，
/// 给它第二位，数字键 2 就能选到。敲的就是缺省写法时（`or`、`may`），另一个写法更少见，不占候选位，照分数排。
/// 撇号词（`I'd`）、组里其它大小写变体也照分数排。敲的含大写时按 [`adapt_case`] 调过再比：调完与敲的相同就不挪
/// （`OR` 原样是 OR，or 调成的也是 OR，挪上来只会重复）。`order[0]` 已是敲的原样。
fn promote_default_form(
    order: &mut Vec<usize>,
    pool: &[PoolWord<'_>],
    sources: &Sources<'_>,
    typed: &str,
    code: &str,
) {
    let Some(main) = sources
        .main
        .filter(|_| code.len() >= MIN_PROMOTED_DEFAULT_LETTERS)
    else {
        return;
    };
    let Some(default) = main
        .get_all(code)
        .iter()
        .map(|entry| entry.word())
        .find(|word| word.eq_ignore_ascii_case(typed))
    else {
        return;
    };
    if default == typed || adapt_case(default, typed) == typed {
        return;
    }
    if let Some(at) = order
        .iter()
        .skip(1)
        .position(|&i| !pool[i].literal && pool[i].word == default)
    {
        let index = order.remove(at + 1);
        order.insert(1, index);
    }
}

/// 填 `P_模型`：有模型时按 token 批量问模型，同 token 的几种写法按词频分这份概率；模型不认识的词、没有模型时用词频换算。
fn fill_model(pool: &mut [PoolWord<'_>], sources: &Sources<'_>, params: &ScoreParams) {
    let floor = params.zipf_floor;
    let Some(model) = sources.model else {
        for word in pool.iter_mut() {
            word.model = zipf_probability(word.frequency, floor);
        }
        return;
    };
    let tokens: Vec<Cow<'_, str>> = pool.iter().map(|w| token_of(w.word)).collect();
    let refs: Vec<&str> = tokens.iter().map(|t| t.as_ref()).collect();
    let log_probs = model.log_probs(sources.history, &refs);
    // 同一个 token 在池里的几种写法（`us` / `US`）共分模型给 token 的概率，按各自词频的比例
    let mut form_mass: HashMap<&str, f64> = HashMap::new();
    for ((word, token), log_prob) in pool.iter().zip(&refs).zip(&log_probs) {
        if log_prob.is_some() {
            *form_mass.entry(token).or_default() += zipf_probability(word.frequency, floor);
        }
    }
    for ((word, token), log_prob) in pool.iter_mut().zip(&refs).zip(&log_probs) {
        let own = zipf_probability(word.frequency, floor);
        word.model = match log_prob {
            Some(log_prob) => log_prob.exp() * own / form_mass[token],
            None => own,
        };
    }
}

/// 候选词 → 模型 token：ASCII 转小写，弯撇号归一成直撇号（与训练时的切词一致）。已经是小写的不分配。
fn token_of(word: &str) -> Cow<'_, str> {
    if word
        .bytes()
        .all(|b| !b.is_ascii_uppercase() && b.is_ascii())
    {
        Cow::Borrowed(word)
    } else {
        Cow::Owned(
            word.chars()
                .map(|c| match c {
                    '\u{2019}' => '\'',
                    c => c.to_ascii_lowercase(),
                })
                .collect(),
        )
    }
}

/// 编码 `code` 下一个候选都没有时，以它开头的更长编码是不是也一定没有（调用方可以记下它、之后直接跳过）。
///
/// 补全、误拼表、个人与领域词都按前缀查，前缀下没有，更长的也没有。纠错按长度分段（4 个字母起纠一处，
/// [`MIN_TWO_EDIT_LETTERS`] 到 [`crate::english::MAX_TWO_EDIT_LETTERS`] 个字母一处没结果再试两处，再长的不试两处）：
/// - 8 到 12 个字母：一个候选都没有说明一处没结果，所以这次试过两处，至多两处错误的变体都没有。
///   更长串的任一个至多两处错误的变体，对应这个串的那段开头就是这个串至多两处错误的变体、又是词的前缀，所以也没有。
/// - 13 个字母起：这次和往后都不试两处，只看一处错误；更长串的一处错误变体对应的开头是这个串至多一处错误的变体，也没有。
///   这个串本身可能有两处错误的变体，但不记它时那些键本来也不试两处，跳过与不跳过结果一样。
/// - 少于 8 个字母不成立：往后到 8 个字母会开始试两处，可能有结果。
///
/// 带数字或符号的串不纠错，只看补全，也成立。
pub fn stays_empty(code: &str) -> bool {
    !code.bytes().all(|b| b.is_ascii_lowercase()) || code.len() >= MIN_TWO_EDIT_LETTERS
}

/// `word` 的检索编码（小写、去撇号）是否正好是 `code`，逐字符比，不分配。
fn same_code(word: &str, code: &str) -> bool {
    let mut code = code.bytes();
    for c in word.chars().filter(|&c| c != '\'' && c != '\u{2019}') {
        let mut buffer = [0; 4];
        for byte in c.to_ascii_lowercase().encode_utf8(&mut buffer).bytes() {
            if code.next() != Some(byte) {
                return false;
            }
        }
    }
    code.next().is_none()
}

#[cfg(test)]
mod tests;
