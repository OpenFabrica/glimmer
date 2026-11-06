//! 英文评测的最简切词：按空白切、去掉两边标点、只留纯字母或带撇号的词，其余跳过并断开上文。
//!
//! 这是第一批「只加尺子」时的临时写法，后续会换成 Core 的共用切词函数（训练语言模型与运行时共用的那一份），
//! 换了之后基线要重量。弯撇号 `’` 当成直撇号 `'`（聊天文本里常见，输入法里敲的是直撇号）。

use super::Piece;

/// 把整段文本切成词与跳过的原文，顺序照原文。空行不产生任何东西；每行最后一段的分隔是换行。
pub fn split(text: &str) -> Vec<Piece> {
    let mut pieces = Vec::new();
    for line in text.lines() {
        let tokens: Vec<&str> = line.split_whitespace().collect();
        for (index, raw) in tokens.iter().enumerate() {
            let separator = if index + 1 == tokens.len() { "\n" } else { " " };
            pieces.push(piece(raw, separator));
        }
    }
    pieces
}

/// 一个空白分隔的片段：去掉两边的非字母数字，剩下的是纯 ASCII 字母或带撇号的就是词，否则整段跳过。
fn piece(raw: &str, separator: &str) -> Piece {
    let raw = raw.replace('\u{2019}', "'");
    let start = raw.find(|c: char| c.is_alphanumeric());
    let end = raw
        .char_indices()
        .rev()
        .find(|(_, c)| c.is_alphanumeric())
        .map(|(index, c)| index + c.len_utf8());
    let (Some(start), Some(end)) = (start, end) else {
        return Piece::Gap(format!("{raw}{separator}"));
    };
    let core = &raw[start..end];
    // 两头已经是字母数字，中间只许字母与撇号：`don't` 留，`well-known`、`3rd`、`café` 跳过
    if !core.chars().all(|c| c.is_ascii_alphabetic() || c == '\'') {
        return Piece::Gap(format!("{raw}{separator}"));
    }
    Piece::Word {
        head: raw[..start].to_owned(),
        text: core.to_owned(),
        tail: format!("{}{separator}", &raw[end..]),
    }
}
