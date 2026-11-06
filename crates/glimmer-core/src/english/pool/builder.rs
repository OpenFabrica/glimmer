//! 候选池的收集器：按词去重、守住总上限。

use std::collections::HashMap;

use super::{MAX_POOL, PoolWord};

/// 往候选池里加词：同一个词只留一份（代价取便宜的那个，来源标记合起来），总数到 [`MAX_POOL`] 就不再收新词。
#[derive(Debug, Default)]
pub(super) struct PoolBuilder<'a> {
    /// 已收的词，按收进来的先后。
    words: Vec<PoolWord<'a>>,

    /// 词 → 在 `words` 里的下标。
    index: HashMap<&'a str, usize>,
}

impl<'a> PoolBuilder<'a> {
    /// 加一个词；是新词并且收下了返回 `true`（各路名额只数新词），已在池里只更新代价与来源。
    pub(super) fn add(
        &mut self,
        word: &'a str,
        cost: f64,
        frequency: Option<u32>,
        domain: bool,
    ) -> bool {
        if let Some(&at) = self.index.get(word) {
            let kept = &mut self.words[at];
            kept.cost = kept.cost.max(cost);
            kept.frequency = kept.frequency.or(frequency);
            kept.domain |= domain;
            return false;
        }
        if self.words.len() >= MAX_POOL {
            return false;
        }
        self.index.insert(word, self.words.len());
        self.words.push(PoolWord {
            word,
            cost,
            frequency,
            personal: 0,
            domain,
            literal: false,
            model: 0.0,
        });
        true
    }

    /// 加原样读法（敲的串本身，未登录词）；同一个串已作为词在池里时不加。
    pub(super) fn add_literal(&mut self, typed: &'a str, cost: f64) {
        if self.add(typed, cost, None, false) {
            self.words.last_mut().expect("刚加进去").literal = true;
        }
    }

    /// 池子是不是空的。
    pub(super) fn is_empty(&self) -> bool {
        self.words.is_empty()
    }

    /// 收完，交出池子。
    pub(super) fn finish(self) -> Vec<PoolWord<'a>> {
        self.words
    }
}
