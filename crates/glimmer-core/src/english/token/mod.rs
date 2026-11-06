//! 英文切词：训练语料（`dict-convert english-ngram`）与运行时共用的唯一一份规则。
//!
//! 文本按空白切成片段，每个片段：
//!
//! 1. 两边去掉「边缘标点」：`. , ; : ! ? " ' ( ) [ ] { } *`、弯引号 `“ ” ‘ ’`、`« »`、`…`、`—` `–`，
//!    以及全角的 `，。！？；：（）「」『』、`。`-` `_` `@` `#` `/` 等不算边缘标点，留在片段里。
//! 2. 剩下的核心只由 ASCII 字母与词内撇号（`'` 或 `’`，不连着两个）组成 → 一个词（转小写、`’` 归一成 `'`）。
//! 3. 否则（数字、URL、`foo_bar`、`@user`、`self-driving`、`café`、`hello,world`）→ [`Token::Gap`]，
//!    不删掉它再把两边接起来。
//! 4. 去掉的尾部标点里有句末标点（`.` `!` `?` `…` `。` `！` `？`）→ 词之后一个 [`Token::SentenceEnd`]；
//!    整个片段都是边缘标点（`...`、`?!`）时同理，没有句末标点（`,`、`—`）就什么也不出。
//!    开头的标点不看（`.NET` 是词 `net`，`...so` 是词 `so`）。
//! 5. 空白里有换行 → [`Token::SentenceEnd`]；文本结尾也是。
//!
//! 缩写里的点不特殊处理，行为是确定的：`Mr.` 是词 `mr` 加句末；`e.g.`、`U.S.` 核心里夹着点，是 Gap 加句末。
//! 「词表里没有的词 = 断开」由调用方拿词表判断（见 [`Token::Word`]），切词本身不依赖词表。

mod event;
mod tokens;

pub use event::Token;
pub use glimmer_dictionary::english_code;
pub use tokens::Tokens;
use tokens::is_line_break;

/// 把一段英文文本切成词与断开，规则见模块文档。
pub fn tokenize(text: &str) -> Tokens<'_> {
    Tokens::new(text)
}

/// 运行时：上屏的一段文本（一个候选、回车原样上屏的字母）当作一个词时的 token，规则与 [`tokenize`] 对一个片段的处理相同
/// （去两边标点、只认 ASCII 字母加词内撇号、转小写）；不是一个词（带空白、数字、符号）为 `None`。
pub fn word_token(text: &str) -> Option<String> {
    if text.chars().any(char::is_whitespace) {
        return None;
    }
    let core = text.trim_matches(is_edge);
    if core.is_empty() {
        return None;
    }
    normalize(core)
}

/// 运行时：组句之外直通给应用的一个字符对上文的作用，规则与 [`tokenize`] 一致：
/// `None` 是不断开（空格、逗号引号这类边缘标点）；[`Token::SentenceEnd`] 是句末标点与换行；
/// 其余（字母、数字、`-` `@` `/` 这类会让片段变成 Gap 的字符）是 [`Token::Gap`]。
///
/// 直通的字母也算 Gap：那是不经候选敲出来的字，引擎不知道它拼成了什么词。
pub fn char_break(c: char) -> Option<Token> {
    if is_line_break(c) || is_sentence_end(c) {
        Some(Token::SentenceEnd)
    } else if c.is_whitespace() || is_edge(c) {
        None
    } else {
        Some(Token::Gap)
    }
}

/// 两边要去掉的标点。
fn is_edge(c: char) -> bool {
    matches!(
        c,
        '.' | ','
            | ';'
            | ':'
            | '!'
            | '?'
            | '"'
            | '\''
            | '('
            | ')'
            | '['
            | ']'
            | '{'
            | '}'
            | '*'
            | '\u{201C}' // “
            | '\u{201D}' // ”
            | '\u{2018}' // ‘
            | '\u{2019}' // ’
            | '«'
            | '»'
            | '…'
            | '—'
            | '–'
            | '，'
            | '。'
            | '！'
            | '？'
            | '；'
            | '：'
            | '（'
            | '）'
            | '「'
            | '」'
            | '『'
            | '』'
            | '、'
    )
}

/// 句末标点。
fn is_sentence_end(c: char) -> bool {
    matches!(c, '.' | '!' | '?' | '…' | '。' | '！' | '？')
}

/// 一个非空白片段 → 事件，记进 `tokens`。
fn classify(chunk: &str, tokens: &mut Tokens<'_>) {
    let trimmed_start = chunk.trim_start_matches(is_edge);
    let core = trimmed_start.trim_end_matches(is_edge);
    let tail = &trimmed_start[core.len()..];
    if core.is_empty() {
        if chunk.chars().any(is_sentence_end) {
            tokens.push_break(Token::SentenceEnd);
        }
        return;
    }
    match normalize(core) {
        Some(word) => tokens.push_word(word),
        None => tokens.push_break(Token::Gap),
    }
    if tail.chars().any(is_sentence_end) {
        tokens.push_break(Token::SentenceEnd);
    }
}

/// 核心是「ASCII 字母 + 词内撇号」时转成词（小写、`’` → `'`），否则 `None`。
/// 首尾一定是字母：撇号是边缘标点，已经被去掉了。
fn normalize(core: &str) -> Option<String> {
    let mut word = String::with_capacity(core.len());
    let mut after_apostrophe = false;
    for c in core.chars() {
        if c.is_ascii_alphabetic() {
            word.push(c.to_ascii_lowercase());
            after_apostrophe = false;
        } else if matches!(c, '\'' | '\u{2019}') && !after_apostrophe {
            word.push('\'');
            after_apostrophe = true;
        } else {
            return None;
        }
    }
    Some(word)
}

#[cfg(test)]
mod tests;
