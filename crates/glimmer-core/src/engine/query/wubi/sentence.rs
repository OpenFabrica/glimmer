//! 五笔整句：连着打的编码（`wqvbkhlg`）不按空格，词图切词出整句，首选就是它。`[wubi] sentence` 开着才做。

use crate::candidate::{Candidate, CandidateKind};
use crate::engine::rescoring::PathUnit;
use crate::engine::{Engine, Learner, MAX_CANDIDATES, MIN_RESCORE_PATHS};
use crate::sentence::{self, Conversion};
use crate::wubi::{MAX_CODE_LEN, is_code_key};

impl Engine {
    /// 整句输入开着（`[wubi] sentence`）。
    pub(in crate::engine) fn wubi_sentence_enabled(&self) -> bool {
        self.wubi
            .as_ref()
            .is_some_and(|scheme| scheme.options().sentence)
    }

    /// 把 `items`（[`Self::wubi_candidates`] 的结果）换成整句的候选：首选是整句，后面跟「开头那段编码」的词
    /// （先四码、再三码……），选了只吃那一段、剩下的接着出整句。返回拼音行的显示串（按整句的切法用 `'` 分开）。
    ///
    /// 不超过四码且码表有命中（全码或前缀）时什么都不动：那是普通五笔，简码位置是肌肉记忆。
    /// 整句没开、作用域里有不是编码键的字符、路径走不通（有占位）时也不动。
    pub(super) fn insert_wubi_sentence(
        &self,
        keys: &str,
        items: &mut Vec<Candidate>,
    ) -> Option<String> {
        if !self.wubi_sentence_enabled() || !keys.chars().all(is_code_key) {
            return None;
        }
        if keys.len() <= MAX_CODE_LEN && !items.is_empty() {
            return None;
        }
        let conversion = self.convert_wubi_sentence(keys)?;
        if conversion.has_placeholder() {
            return None;
        }
        let display = typed_codes(&conversion);
        let mut sentence_items = vec![Candidate {
            text: conversion.text,
            kind: CandidateKind::Sentence,
            // 整句吃掉整段作用域（`wubi_consumed_by` 按第一个「音节」的长度吃）
            syllables: vec![keys.to_owned()],
            reading: None,
            translation: None,
        }];
        for len in (1..=MAX_CODE_LEN.min(keys.len())).rev() {
            let head = &keys[..len];
            // 只要全码命中：前缀命中的编码比这一段长，吃的键数对不上
            let full = self
                .wubi_candidates(head)
                .into_iter()
                .filter(|c| c.syllables.first().is_some_and(|code| code == head));
            for candidate in full {
                if sentence_items.len() >= MAX_CANDIDATES {
                    break;
                }
                if !sentence_items.iter().any(|c| c.text == candidate.text) {
                    sentence_items.push(candidate);
                }
            }
        }
        *items = sentence_items;
        Some(display)
    }

    /// 连着打的编码转成整句：码表 + 用户词建词图，静态语言模型与个人 n-gram 插值，用户选择次数加分，上文取上屏链。
    /// 接了神经重打分器时取前几条路径重排，与拼音的 [`Self::convert_sentence_with`] 同一套：
    /// 语言模型与神经模型都按文本算，不管字是怎么打出来的。路径数跟着句长涨：拼音是每个音节两条，
    /// 一个字的编码平均两个字母上下，所以按字母数给，至少 [`MIN_RESCORE_PATHS`] 条、到引擎的上限封顶。
    pub(in crate::engine) fn convert_wubi_sentence(&self, keys: &str) -> Option<Conversion> {
        let dictionaries = self.wubi_dictionaries();
        let paths = if self.has_sentence_scorer() {
            keys.len().max(MIN_RESCORE_PATHS).min(self.rescore_paths)
        } else {
            1
        };
        let search = sentence::Search {
            keep_partial: false,
            paths,
            initial: self.context(),
            protected_extra: 0.0,
            // 五笔编码不是读音
            reading: None,
            whole_word: false,
        };
        let mut paths = sentence::convert_codes(
            &dictionaries,
            keys,
            search,
            &*self.language_model,
            self.personal(),
            |text| self.learner.weight(text),
            &mut self.span_cache.borrow_mut(),
        );
        // 与最优路径差得太远的不参与重排，同拼音
        if paths.len() > 1 {
            let floor = paths[0].score - self.neural_margin;
            paths.retain(|p| p.score >= floor);
            // 五笔一个词只有一条编码，几条路径要按字母对齐才能拼（见 [`PathUnit`]）
            self.rescore_paths(&mut paths, PathUnit::CodeLetters);
        }
        paths.into_iter().next()
    }
}

/// 拼音行的显示串：敲的编码按整句路径上的词用 `'` 分开（`wqvb'khlg`）。路径上每个词的「音节」就是敲的那段字母。
fn typed_codes(conversion: &Conversion) -> String {
    conversion.syllables.join("'")
}
