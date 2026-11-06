//! Engine 上更新英文上文的钩子：上屏、原样上屏、直通字符、断开。规则见 [`super`] 的模块文档。

use std::time::Instant;

use glimmer_dictionary::english_code;

use super::EnglishContext;
use crate::engine::Engine;
use crate::english::{NgramHistory, Token, char_break, word_token};

impl Engine {
    /// 壳告知英文上文断开了：直通了方向键、Home / End、翻页、Tab、Esc、Delete 这类功能键，或带 ⌘ / Ctrl 的组合键，
    /// 光标可能已经不在刚才上屏的地方。只把英文上文作废，不碰中文的上屏链与输入日志。
    pub fn break_english_context(&mut self) {
        self.english_context.touch(Instant::now());
        self.english_context.lose();
    }

    /// 英文候选现在用的上文。
    pub fn english_history(&self) -> NgramHistory<'_> {
        self.english_context.history()
    }

    /// 当前英文上文（测试与诊断看状态用）。
    pub fn english_context(&self) -> &EnglishContext {
        &self.english_context
    }

    /// 一次按键：只刷新时刻（停手太久就作废上文）。
    pub(in crate::engine) fn touch_english_context(&mut self) {
        self.english_context.touch(Instant::now());
    }

    /// 一段文字作为英文词上屏了（英文候选、英文模式回车原样上屏）：是随包词表里的词就记下，否则上文作废。
    pub(in crate::engine) fn note_english_word(&mut self, text: &str) {
        self.english_context.touch(Instant::now());
        match word_token(text).filter(|token| self.in_english_vocabulary(token)) {
            Some(token) => self.english_context.push_word(token),
            None => self.english_context.lose(),
        }
    }

    /// 上屏的不是英文词（中文、emoji、快捷候选）：上文作废。
    pub(in crate::engine) fn note_other_commit(&mut self) {
        self.english_context.touch(Instant::now());
        self.english_context.lose();
    }

    /// 组句之外一个字符直通给了应用（或转成了全角标点）：空格与句中标点不断开，句末标点与换行是句首，其余作废。
    /// 半角空格另记「刚敲过空格」，下一词预测看它。
    pub(in crate::engine) fn note_english_char(&mut self, c: char) {
        self.english_context.touch(Instant::now());
        if c == ' ' {
            self.english_context.note_space();
        }
        match char_break(c) {
            None => {}
            Some(Token::SentenceEnd) => self.english_context.start_sentence(),
            Some(_) => self.english_context.lose(),
        }
    }

    /// token 是不是随包英文词表里某个词的小写写法：语言模型只认这些词，训练时别的词都当断开。
    fn in_english_vocabulary(&self, token: &str) -> bool {
        self.english.as_ref().is_some_and(|list| {
            list.get_all(&english_code(token))
                .iter()
                .any(|entry| word_token(entry.word()).as_deref() == Some(token))
        })
    }
}
