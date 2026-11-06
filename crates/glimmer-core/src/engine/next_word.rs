//! 英文下一词预测的对外入口：取预测、Tab 接受。算法在 [`crate::english::predict`]，这里只管「什么时候出」与上屏。
//!
//! 出预测要同时满足：英文模式（壳只在 `english_candidates` 开着时打开它）、`english_prediction` 开着、
//! 不在私密输入里（[`Engine::set_private`]）、组句为空、装了英文语言模型与词表、
//! 英文上文记着词（不是不知道、不是句首）、最近一次事件是直通空格、上文没过期，
//! 且第一名概率不低于 [`crate::english::PredictParams::min_probability`]。

use std::time::Instant;

use super::Engine;
use super::input_log::InputSource;
use crate::candidate::{Candidate, CandidateKind, CandidateList};
use crate::english::NextWord;

impl Engine {
    /// 当前该显示的下一词预测，概率从高到低（缺省最多一个）；条件不满足为空。
    /// 同一个上文反复调只在第一次扫后继（壳可以在轮询里调），不改任何状态。
    /// 候选是 [`CandidateKind::English`]、`text` 是词表写法，要释义照常调 [`Self::annotate`]。
    pub fn english_predictions(&self) -> CandidateList {
        let items = self
            .english_next_words()
            .into_iter()
            .map(|word| english_candidate(word.text))
            .collect();
        CandidateList { items }
    }

    /// 同 [`Self::english_predictions`]，但带概率（CLI 手看与评测用）。
    pub fn english_next_words(&self) -> Vec<NextWord> {
        let (Some(model), Some(list)) = (self.english_lm.as_deref(), self.english.as_ref()) else {
            return Vec::new();
        };
        if !self.english_mode
            || !self.english_prediction
            || self.private
            || !self.composition.is_empty()
            || !self.english_context.predictable(Instant::now())
        {
            return Vec::new();
        }
        self.english_predictions
            .borrow_mut()
            .get(
                model,
                list,
                self.english_context.history(),
                &self.english_predict,
            )
            .to_vec()
    }

    /// Tab 接受第 `index` 个预测（0 起）：与选中这个英文候选上屏一样（个人词频 +1、记进英文上文），
    /// 再算一次直通空格，返回要交给应用的「词 + 空格」。之后上文前移一个词、处于「刚敲过空格」，马上能取下一个预测。
    /// 序号越界或条件已不满足（过期了、上文变了、进了私密输入）返回 `None`：什么都不做，壳把 Tab 照常交给应用。
    /// 壳不要再为这个空格调 [`Self::note_passthrough`]，这里已经记过了。
    pub fn accept_english_prediction(&mut self, index: usize) -> Option<String> {
        let word = self.english_next_words().into_iter().nth(index)?;
        // 上一次查询是上一个词的，它的候选摘要不该写进这次上屏的日志
        *self.last_query.borrow_mut() = None;
        let mut text = self.commit_with(
            &english_candidate(word.text),
            InputSource::EnglishPrediction,
            None,
        );
        self.note_passthrough(' ');
        text.push(' ');
        Some(text)
    }
}

/// 预测词 → 英文候选。
fn english_candidate(text: String) -> Candidate {
    Candidate {
        text,
        kind: CandidateKind::English,
        syllables: Vec::new(),
        reading: None,
        translation: None,
    }
}
