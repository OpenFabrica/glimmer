//! 切词迭代器：逐段扫文本，产出词与断开，并把连续的断开合并。

use std::collections::VecDeque;

use super::{Token, classify};

/// [`super::tokenize`] 返回的迭代器。
#[derive(Debug)]
pub struct Tokens<'a> {
    /// 还没扫到的文本。
    rest: &'a str,

    /// 已经定下、等着交出去的事件。
    out: VecDeque<Token>,

    /// 上一个词之后见到的断开，只记最后一个（「句末 + 数字」之后是 Gap，「数字 + 句末」之后是句首）。
    pending: Option<Token>,

    /// 已经交出过词。
    started: bool,

    /// 结尾的句末已经补过。
    done: bool,
}

impl<'a> Tokens<'a> {
    /// 从文本开头开始切。
    pub(super) fn new(text: &'a str) -> Self {
        Self {
            rest: text,
            out: VecDeque::new(),
            pending: None,
            started: false,
            done: false,
        }
    }

    /// 记一个词：先把攒着的断开放出去（开头的句末省掉），再放词。
    pub(super) fn push_word(&mut self, word: String) {
        if let Some(pending) = self.pending.take()
            && (self.started || pending == Token::Gap)
        {
            self.out.push_back(pending);
        }
        self.out.push_back(Token::Word(word));
        self.started = true;
    }

    /// 记一处断开：只覆盖攒着的那个，等下一个词来了才放出去。
    pub(super) fn push_break(&mut self, token: Token) {
        self.pending = Some(token);
    }

    /// 扫下一段（一串空白，加上后面一个非空白片段），把事件记进 `out`；文本扫完返回 `false`。
    fn scan(&mut self) -> bool {
        let text = self.rest;
        let start = text
            .find(|c: char| !c.is_whitespace())
            .unwrap_or(text.len());
        if text[..start].chars().any(is_line_break) {
            self.push_break(Token::SentenceEnd);
        }
        if start == text.len() {
            self.rest = "";
            return false;
        }
        let chunk_len = text[start..]
            .find(char::is_whitespace)
            .unwrap_or(text.len() - start);
        let chunk = &text[start..start + chunk_len];
        self.rest = &text[start + chunk_len..];
        classify(chunk, self);
        true
    }
}

impl Iterator for Tokens<'_> {
    type Item = Token;

    /// 先交攒好的事件；不够就接着扫；扫完了、交出过词就补一个结尾的句末。
    fn next(&mut self) -> Option<Token> {
        loop {
            if let Some(token) = self.out.pop_front() {
                return Some(token);
            }
            if !self.scan() {
                if self.started && !self.done {
                    self.done = true;
                    return Some(Token::SentenceEnd);
                }
                return None;
            }
        }
    }
}

/// 换行类空白：`\n`、`\r`、Unicode 行 / 段分隔符。
pub(super) fn is_line_break(c: char) -> bool {
    matches!(c, '\n' | '\r' | '\u{2028}' | '\u{2029}')
}
