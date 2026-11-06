//! 一行文本里的原样写法：词与句子边界全由 `glimmer_core::english::tokenize` 定，这里只把它的小写词对回原文的写法。

use glimmer_core::english::{Token, tokenize, word_token};

/// 一行里该计数的原样写法，以及跳过的词数。
#[derive(Debug, Default, PartialEq, Eq)]
pub(super) struct LineWords {
    /// 计数的写法（撇号统一成 `'`），按出现先后。
    pub(super) counted: Vec<String>,

    /// 句首（行首、句末之后、断开之后）跳过的词数。
    pub(super) initial: usize,

    /// 整行都是首字母大写（全大写行、标题）而跳过的词数。
    pub(super) heading: usize,

    /// 紧挨着另一个大写词（多词专名的一部分）而跳过的大写写法数。
    pub(super) in_name: usize,

    /// 原文写法与 `tokenize` 的词对不上（不该发生）：整行不计。
    pub(super) mismatched: bool,
}

/// 一个空白分隔的片段若是词，取它的原样写法：`tokenize` 去掉的两边标点都不是字母、词的首尾一定是字母，
/// 所以原样写法就是从第一个 ASCII 字母到最后一个 ASCII 字母的那一段；`’` 归一成 `'`（与 `tokenize` 一致）。
pub(super) fn surface(chunk: &str) -> Option<String> {
    word_token(chunk)?;
    let start = chunk.find(|c: char| c.is_ascii_alphabetic())?;
    let end = chunk.rfind(|c: char| c.is_ascii_alphabetic())? + 1;
    Some(chunk[start..end].replace('\u{2019}', "'"))
}

/// 切一行：`tokenize` 给出词与断开，片段按同样的空白切出原样写法，两边一一对上。
///
/// - 行首、`SentenceEnd` 之后的第一个词不计：句首大写不说明它本来是大写写法。
/// - `Gap` 之后的第一个词也不计：`Done. 3 Cats` 里句末与数字合成了一个 Gap，Cats 仍可能因为句首而大写；
///   不计只少了些样本，不会让哪种写法占便宜。
/// - 一整行（至少两个词）每个词都首字母大写时整行不计：全大写行（`THE AND`）与标题（`How To Train Your Dog`）。
/// - 首字母大写的写法，同一句里紧挨着另一个（非句首的）首字母大写词时不计：它是多词专名的一部分
///   （`South Africa`、`Hyde Park`、`Martin Luther King`），说明不了单独敲这个词时该用哪种写法；
///   不计的话 `park`、`south`、`king` 都会被专名顶成大写。单独出现的专名（`in Japan`、`use Google`）照计。
pub(super) fn line_words(line: &str) -> LineWords {
    let mut surfaces = line.split_whitespace().filter_map(surface);
    // (原样写法, 不在句首, 第几段)：段号在每个断开处加一，相邻判断不跨断开
    let mut words: Vec<(String, bool, usize)> = Vec::new();
    let mut result = LineWords::default();
    let mut at_start = true;
    let mut segment = 0usize;
    for token in tokenize(line) {
        match token {
            Token::Word(word) => {
                let Some(raw) = surfaces.next() else {
                    result.mismatched = true;
                    break;
                };
                if raw.to_ascii_lowercase() != word {
                    result.mismatched = true;
                    break;
                }
                words.push((raw, !at_start, segment));
                at_start = false;
            }
            Token::SentenceEnd | Token::Gap => {
                at_start = true;
                segment += 1;
            }
        }
    }
    if result.mismatched || surfaces.next().is_some() {
        return LineWords {
            mismatched: true,
            ..LineWords::default()
        };
    }
    let capital = |raw: &str| raw.starts_with(|c: char| c.is_ascii_uppercase());
    let heading = words.len() >= 2 && words.iter().all(|(raw, _, _)| capital(raw));
    for (index, (raw, counted, segment)) in words.iter().enumerate() {
        if heading {
            result.heading += 1;
            continue;
        }
        if !counted {
            result.initial += 1;
            continue;
        }
        // 前一个词要不在句首才算：`The South` 里的 The 是因为句首才大写
        let previous = index
            .checked_sub(1)
            .map(|i| &words[i])
            .is_some_and(|(p, counted, s)| s == segment && *counted && capital(p));
        let next = words
            .get(index + 1)
            .is_some_and(|(n, _, s)| s == segment && capital(n));
        if capital(raw) && (previous || next) {
            result.in_name += 1;
            continue;
        }
        result.counted.push(raw.clone());
    }
    result
}
