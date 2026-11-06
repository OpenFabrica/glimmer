//! 生成敲错变体：已敲的串 → 「本来想敲的」前缀，四种错误（替换、多敲、漏敲、相邻对调）。

use std::collections::HashMap;

use super::{CostTier, Variant};
use crate::english::keyboard::is_adjacent;

/// 变体串 → 最便宜的档，生成过程中去重用。
pub(super) type VariantMap = HashMap<Vec<u8>, CostTier>;

/// 26 个小写字母的位集：第 `c - b'a'` 位为 1 表示字母 `c` 可用。
pub(super) const ALL_LETTERS: u32 = (1 << 26) - 1;

/// 不剪枝：任何开头后面都能接任何字母。
pub(super) fn any_letter(_: &[u8]) -> u32 {
    ALL_LETTERS
}

/// `code` 的全部「一处错误」变体，按（档、串）排好；`code` 不是纯小写字母（或为空）时为空。
///
/// 不含 `code` 本身，也不含以 `code` 开头的串（在末尾补字母就是普通补全，不算纠错）。
/// 这里不管长度门槛，什么时候纠由 [`super::should_correct`] 定。
pub fn one_edit_variants(code: &str) -> Vec<Variant> {
    if !letters_only(code) {
        return Vec::new();
    }
    let code = code.as_bytes();
    let mut found = VariantMap::new();
    edits(code, 0, code.len(), &any_letter, &mut |text, tier, _| {
        keep_cheapest(&mut found, text, tier)
    });
    sorted(found, code)
}

/// `code` 的全部「两处错误」变体（档都是 [`CostTier::TwoEdits`]），按串排好；不含一处错误就能得到的串。
///
/// 数量大（10 个字母约 30 万个），只给测试与离线工具对照用；运行时走 [`super::valid_two_edit_variants`]，
/// 它边生成边按词表剪枝。
pub fn two_edit_variants(code: &str) -> Vec<Variant> {
    if !letters_only(code) {
        return Vec::new();
    }
    let code = code.as_bytes();
    let first = first_steps(code, code.len(), &any_letter);
    let mut found = VariantMap::new();
    for text in first.keys() {
        edits(text, 0, text.len(), &any_letter, &mut |second, _, _| {
            if !first.contains_key(second) {
                found.insert(second.to_vec(), CostTier::TwoEdits);
            }
        });
    }
    sorted(found, code)
}

/// 只在位置 `0..=max_pos` 上、只用 `allowed` 允许的字母做一处编辑的变体（去重、取最便宜档，未去掉以 `code` 开头的）。
pub(super) fn one_edit_map(
    code: &[u8],
    max_pos: usize,
    allowed: &impl Fn(&[u8]) -> u32,
) -> VariantMap {
    let mut found = VariantMap::new();
    edits(code, 0, max_pos, allowed, &mut |text, tier, _| {
        keep_cheapest(&mut found, text, tier)
    });
    found
}

/// 两处错误的第一步：位置 `0..=max_pos` 上一处编辑的变体 → 得到它的最小编辑位置。
///
/// 记位置是为了第二步只在这个位置及以后改（见 `valid.rs` 的说明），同一个串由几处编辑得到时取最小的，范围最宽。
pub(super) fn first_steps(
    code: &[u8],
    max_pos: usize,
    allowed: &impl Fn(&[u8]) -> u32,
) -> HashMap<Vec<u8>, usize> {
    let mut found: HashMap<Vec<u8>, usize> = HashMap::new();
    edits(
        code,
        0,
        max_pos,
        allowed,
        &mut |text, _, pos| match found.get_mut(text) {
            Some(p) => *p = (*p).min(pos),
            None => {
                found.insert(text.to_vec(), pos);
            }
        },
    );
    found
}

/// 对 `code` 在位置 `from..=to` 上的每一处编辑调一次 `emit(变体, 档, 位置)`，同一个串可能来好几次。
/// 变体借给 `emit` 看（同一块缓冲区反复用），要留下的由 `emit` 自己复制：两处错误时生成的串上万个、绝大多数当场扔掉。
///
/// 「位置 j 上的编辑」只改 `j` 及以后的字节，`code[..j]` 不变：替换 / 删掉第 j 个、在第 j 个前插入、对调 j 与 j+1。
/// 剪枝靠的就是这一点：`code[..j]` 已经不是任何词的前缀时，`j` 及以后怎么改都没用（见 `valid.rs`）。
/// 在末尾插入（`j == len`）不生成：那是普通补全。
///
/// `allowed(开头)` 给出这个开头后面能接哪些字母（位集）：替换与漏敲在第 j 位放进的字母只试其中的，
/// 词表剪枝时由它挡掉绝大多数注定不是前缀的串；不剪枝时传 [`any_letter`]。
pub(super) fn edits(
    code: &[u8],
    from: usize,
    to: usize,
    allowed: &impl Fn(&[u8]) -> u32,
    emit: &mut impl FnMut(&[u8], CostTier, usize),
) {
    let n = code.len();
    if n == 0 {
        return;
    }
    let mut text: Vec<u8> = Vec::with_capacity(n + 1);
    for j in from..=to.min(n - 1) {
        let letters = allowed(&code[..j]);
        let usable = |c: u8| letters & (1 << (c - b'a')) != 0;
        // 多敲：用户多按了 code[j]，本来想敲的是去掉它（只剩空串时不算，空前缀什么都能补）
        if n > 1 {
            text.clear();
            text.extend_from_slice(&code[..j]);
            text.extend_from_slice(&code[j + 1..]);
            emit(&text, extra_tier(code, j), j);
        }
        // 替换：code[j] 按错了，本来是 c
        text.clear();
        text.extend_from_slice(code);
        for c in b'a'..=b'z' {
            if c != code[j] && usable(c) {
                text[j] = c;
                let tier = if is_adjacent(code[j], c) {
                    CostTier::Near
                } else {
                    CostTier::Edit
                };
                emit(&text, tier, j);
            }
        }
        // 漏敲：code[j] 前面少按了一个 c（先放一个占位，循环里只改这一位）
        text.clear();
        text.extend_from_slice(&code[..j]);
        text.push(b'a');
        text.extend_from_slice(&code[j..]);
        for c in (b'a'..=b'z').filter(|&c| usable(c)) {
            text[j] = c;
            // 补回的字母与左右相同，是双写字母少按了一下
            let doubled = c == code[j] || (j > 0 && c == code[j - 1]);
            let tier = if doubled {
                CostTier::Near
            } else {
                CostTier::Edit
            };
            emit(&text, tier, j);
        }
        // 相邻对调
        if j + 1 < n && code[j] != code[j + 1] {
            text.clear();
            text.extend_from_slice(code);
            text.swap(j, j + 1);
            emit(&text, CostTier::Near, j);
        }
    }
}

/// 多敲的 code[j] 与左右某个字母相同（双写多按）或是它的邻键（一指按到两个键）时便宜。
fn extra_tier(code: &[u8], j: usize) -> CostTier {
    let x = code[j];
    let close = |other: u8| other == x || is_adjacent(other, x);
    let near = (j > 0 && close(code[j - 1])) || code.get(j + 1).is_some_and(|&b| close(b));
    if near { CostTier::Near } else { CostTier::Edit }
}

/// 记下变体，同一个串留最便宜的档：生成顺序不影响结果。已经记过的不再复制。
pub(super) fn keep_cheapest(found: &mut VariantMap, text: &[u8], tier: CostTier) {
    match found.get_mut(text) {
        Some(t) => *t = (*t).min(tier),
        None => {
            found.insert(text.to_vec(), tier);
        }
    }
}

/// 去掉以 `code` 开头的（含等于它的），按（档、串）排成 `Variant`。
fn sorted(found: VariantMap, code: &[u8]) -> Vec<Variant> {
    let mut out: Vec<Variant> = found
        .into_iter()
        .filter(|(text, _)| !text.starts_with(code))
        .map(|(text, tier)| Variant {
            text: ascii(text),
            tier,
        })
        .collect();
    out.sort_by(|a, b| a.tier.cmp(&b.tier).then_with(|| a.text.cmp(&b.text)));
    out
}

/// 变体只由小写字母组成，按字节转成 `String`。
pub(super) fn ascii(text: Vec<u8>) -> String {
    text.into_iter().map(char::from).collect()
}

/// 非空且只有小写 ASCII 字母。
pub(super) fn letters_only(code: &str) -> bool {
    !code.is_empty() && code.bytes().all(|b| b.is_ascii_lowercase())
}
