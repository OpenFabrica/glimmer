//! 先过词表再限额：变体里只留至少是一个词前缀的，按（档、前缀下词频和）取前 N 个。

use std::cell::RefCell;
use std::collections::HashMap;
use std::ops::Range;

use glimmer_dictionary::{WordEntry, WordList};

use super::generate::{VariantMap, ascii, edits, first_steps, keep_cheapest, one_edit_map};
use super::{CostTier, MIN_TWO_EDIT_LETTERS, ValidVariant, should_correct};

/// `code` 的「一处错误」变体里，是某张词表里某个编码前缀的那些，按（档、前缀下词频和降序、串）排好取前 `limit` 个。
///
/// `code` 须是小写检索编码；[`should_correct`] 不通过（少于 4 个字母、带数字或符号）时为空。
/// 结果与 `one_edit_variants` 全生成后逐个查词表完全一样，只是生成时就跳过了不可能的位置：
/// `code[..j]` 已不是任何编码的前缀时，第 `j` 位及以后的编辑都留不下来。
pub fn valid_variants(code: &str, lists: &[&WordList], limit: usize) -> Vec<ValidVariant> {
    if limit == 0 || !should_correct(code) {
        return Vec::new();
    }
    let bytes = code.as_bytes();
    let found = one_edit_map(bytes, reach(lists, bytes), &|head| {
        next_letters(lists, head)
    });
    rank(found, bytes, lists, limit)
}

/// `code` 的「两处错误」变体里过了词表的，排法同 [`valid_variants`]，档都是 [`CostTier::TwoEdits`]；
/// 一处错误就能得到的串不在里面。少于 [`MIN_TWO_EDIT_LETTERS`] 个字母或不该纠时为空。
///
/// 两步都剪枝，结果仍与全生成再过词表一样（有测试对拍）：两处错误的变体几乎都能按「先改靠前的、再改靠后的」两步得到，
/// 第一步改第 `p1` 位时 `code[..p1]` 就是结果的开头，第二步改第 `p2 ≥ p1` 位时中间串的 `[..p2]` 也是；
/// 结果是词的前缀，这两段也都是。所以第一步只试 `p1 ≤ reach(code)`，第二步只试 `p1 ≤ p2 ≤ reach(中间串)`。
/// 第一步把开头改坏了的中间串 `reach` 正好停在 `p1`，第二步只剩一个位置可试，这是省下大部分工作量的地方。
/// 两步放进的字母还按「开头后能接什么」筛：两步落在同一位置时（同一处连漏两个字母）换个先后就成了 `p2 = p1 + 1`，
/// 所以第一步放进的字母总能接在 `code[..p1]` 后面，筛它不会漏。
///
/// 唯一换不了先后的：第二步是跨在第一步边上的对调（第 `p1 − 1` 与第 `p1` 位）。第二步在 `p1 − 1` 之前的编辑
/// 只碰没改过的开头，总能挪到前面先做；替换或漏敲再对调，等于两处按顺序的替换或一处漏敲；剩下的「删掉一个再对调」
/// （`definitlye` 删 y 成 `definitle`，再对调 l、e 成 `definitel`）只能这样得到。所以第二步对每个中间串
/// 另试一次第 `p1 − 1` 位的对调；结果开头 `code[..p1 − 1]` 是词的前缀，第一步要试到 `p1 ≤ reach(code) + 1`。
pub fn valid_two_edit_variants(code: &str, lists: &[&WordList], limit: usize) -> Vec<ValidVariant> {
    if limit == 0 || code.len() < MIN_TWO_EDIT_LETTERS || !should_correct(code) {
        return Vec::new();
    }
    let bytes = code.as_bytes();
    // 第二步的开头几乎都是 `code[..p1]` 这十来种（第一步只改第 p1 位及以后），同一个开头的「后面能接什么」
    // 每次都要几十次二分；记下来，两处错误的耗时少一大半
    let memo: RefCell<HashMap<Vec<u8>, u32>> = RefCell::new(HashMap::new());
    let allowed = |head: &[u8]| {
        if let Some(&mask) = memo.borrow().get(head) {
            return mask;
        }
        let mask = next_letters(lists, head);
        memo.borrow_mut().insert(head.to_vec(), mask);
        mask
    };
    let first = first_steps(bytes, reach(lists, bytes) + 1, &allowed);
    let mut found = VariantMap::new();
    for (text, &p1) in &first {
        // 跨在第一步边上的对调（见上面的说明）：每个中间串最多一个，直接整表查
        if p1 > 0 && p1 < text.len() && text[p1 - 1] != text[p1] {
            let mut swapped = text.clone();
            swapped.swap(p1 - 1, p1);
            if reach(lists, &swapped) == swapped.len() && !first.contains_key(&swapped) {
                keep_cheapest(&mut found, &swapped, CostTier::TwoEdits);
            }
        }
        let to = reach(lists, text);
        if to < p1 {
            continue;
        }
        // 第二步的串绝大多数不是词的前缀，要当场扔掉。在第 j 位改的串开头 `text[..j]` 不变，
        // 它若是词的前缀，那个词一定在 `text[..j]` 的区间里：先把各位置的区间逐字收窄求好，
        // 每个串只在小区间里二分，不再每次在整张表里二分（这一步是两处错误的大头，每次一万多个串）
        let heads = head_spans(lists, text, p1, to);
        edits(text, p1, to, &allowed, &mut |second, _, pos| {
            if is_prefix_within(lists, &heads[pos - p1], pos, second) && !first.contains_key(second)
            {
                keep_cheapest(&mut found, second, CostTier::TwoEdits);
            }
        });
    }
    rank(found, bytes, lists, limit)
}

/// 各词表里 `text` 最长的、仍是编码前缀的开头长度取最大：编辑位置超过它就不可能留下。
fn reach(lists: &[&WordList], text: &[u8]) -> usize {
    // 变体只含 ASCII 小写字母，一定是合法 UTF-8
    let text = std::str::from_utf8(text).unwrap_or_default();
    lists
        .iter()
        .map(|list| list.matched_prefix_len(text))
        .max()
        .unwrap_or(0)
}

/// `heads[k][i]`：第 i 张词表里编码以 `text[..from + k]` 开头的词目区间，`k` 从 0 到 `to - from`。
///
/// 先二分出 `text[..from]` 的区间，之后每多一个字节只在上一个区间里二分收窄。
fn head_spans(lists: &[&WordList], text: &[u8], from: usize, to: usize) -> Vec<Vec<Range<usize>>> {
    // 变体只含 ASCII 小写字母，一定是合法 UTF-8
    let start = std::str::from_utf8(&text[..from]).unwrap_or_default();
    let mut current: Vec<Range<usize>> = lists.iter().map(|list| list.prefix_span(start)).collect();
    let mut heads = Vec::with_capacity(to - from + 1);
    for (depth, &byte) in text.iter().enumerate().take(to).skip(from) {
        let next = lists
            .iter()
            .zip(&current)
            .map(|(list, span)| narrow(list, span.clone(), depth, byte))
            .collect();
        heads.push(std::mem::replace(&mut current, next));
    }
    heads.push(current);
    heads
}

/// 区间 `span` 里的编码开头都相同、长 `depth`，取其中第 `depth` 个字节是 `byte` 的那一段。
fn narrow(list: &WordList, span: Range<usize>, depth: usize, byte: u8) -> Range<usize> {
    let block = &list.as_slice()[span.clone()];
    // 区间里按第 depth 个字节有序，正好 depth 长的（没有这个字节）排最前
    let at = |e: &WordEntry| e.code().as_bytes().get(depth).copied();
    let low = block.partition_point(|e| at(e) < Some(byte));
    let high = block.partition_point(|e| at(e) <= Some(byte));
    span.start + low..span.start + high
}

/// `text` 是不是某张词表里某个编码的前缀；`spans` 是各表里一定包含答案的区间（[`head_spans`] 的一项），
/// 区间里的编码与 `text` 的前 `depth` 个字节都相同，所以只比后面的。
fn is_prefix_within(
    lists: &[&WordList],
    spans: &[Range<usize>],
    depth: usize,
    text: &[u8],
) -> bool {
    let rest = &text[depth..];
    lists.iter().zip(spans).any(|(list, span)| {
        let block = &list.as_slice()[span.clone()];
        let at = block.partition_point(|e| less(tail(e, depth), rest));
        block
            .get(at)
            .is_some_and(|e| tail(e, depth).starts_with(rest))
    })
}

/// 词目编码第 `depth` 个字节往后的部分。
fn tail(entry: &WordEntry, depth: usize) -> &[u8] {
    &entry.code().as_bytes()[depth..]
}

/// 字节序 `a < b`。编码只有十来个字节，逐字节比比调 `memcmp` 快（两处错误时每次查询要比几十万次）。
fn less(a: &[u8], b: &[u8]) -> bool {
    for (x, y) in a.iter().zip(b) {
        if x != y {
            return x < y;
        }
    }
    a.len() < b.len()
}

/// 各词表里 `head` 后面能接的小写字母，合成位集。
fn next_letters(lists: &[&WordList], head: &[u8]) -> u32 {
    let head = std::str::from_utf8(head).unwrap_or_default();
    lists
        .iter()
        .flat_map(|list| list.next_bytes(head))
        .filter(u8::is_ascii_lowercase)
        .fold(0, |mask, c| mask | (1 << (c - b'a')))
}

/// 查词表、排序、截断。
///
/// - 不是任何编码前缀的扔掉；以 `code` 开头的（含等于）扔掉，那是普通补全，不算纠错。
/// - 是 `code` 真前缀的（删掉末尾多敲的字母，`goodd` → `good`）留着：它可能正是想要的整词。
///   它补出的词与普通补全有重叠，由调用方按词去重、留代价低的那份。
/// - 变体之间互为前缀（`tom` 与 `tomo`）也都留着：长的更具体，检索时按名额取词不容易被宽前缀里的高频词挤掉。
/// - 排序键是（档、词频和降序、串），只看变体本身，与生成先后无关。
fn rank(found: VariantMap, code: &[u8], lists: &[&WordList], limit: usize) -> Vec<ValidVariant> {
    let mut out: Vec<ValidVariant> = Vec::new();
    for (text, tier) in found {
        if text.is_empty() || text.starts_with(code) {
            continue;
        }
        let text = ascii(text);
        let spans: Vec<_> = lists.iter().map(|list| list.prefix_span(&text)).collect();
        if spans.iter().all(|span| span.is_empty()) {
            continue;
        }
        let mass = lists
            .iter()
            .zip(spans)
            .map(|(list, span)| list.span_mass(span))
            .sum();
        out.push(ValidVariant { text, tier, mass });
    }
    out.sort_by(|a, b| {
        a.tier
            .cmp(&b.tier)
            .then_with(|| b.mass.cmp(&a.mass))
            .then_with(|| a.text.cmp(&b.text))
    });
    out.truncate(limit);
    out
}
