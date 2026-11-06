//! 逐字母敲一个串、每键查一次候选的记录。

use std::time::{Duration, Instant};

use glimmer_core::{Candidate, Engine};

/// 敲一个串的过程：每一键之后目标词在候选里排第几、这一键查询用了多久，以及敲完时的首选。
#[derive(Debug, Clone, Default)]
pub struct Trace {
    /// 第 i 项是敲完前 i + 1 个字符后目标词在候选列表里的下标；不在候选里为 `None`。
    pub ranks: Vec<Option<usize>>,

    /// 每一键（推键 + 查询）的耗时，与 `ranks` 一一对应。
    pub durations: Vec<Duration>,

    /// 敲完整个串时的第一个候选（空格会上屏它）；没有候选为 `None`。
    pub top: Option<Candidate>,

    /// 敲完整个串时的前三个候选，报告里举例用。
    pub top3: Vec<String>,
}

impl Trace {
    /// 从空缓冲区开始逐字符推 `typed`，每键查一次，记下 `target` 的名次。不上屏、不清缓冲区，由调用方决定。
    pub fn record(engine: &mut Engine, typed: &str, target: &str) -> Self {
        let mut trace = Self::default();
        for c in typed.chars() {
            let started = Instant::now();
            engine.push(c);
            let items = engine
                .query()
                .map(|query| query.candidates.items)
                .unwrap_or_default();
            trace.durations.push(started.elapsed());
            trace
                .ranks
                .push(items.iter().position(|c| c.text == target));
            trace.top3 = items.iter().take(3).map(|c| c.text.clone()).collect();
            trace.top = items.into_iter().next();
        }
        trace
    }

    /// 敲完整个串时目标词排第几（下标）。
    pub fn final_rank(&self) -> Option<usize> {
        self.ranks.last().copied().flatten()
    }

    /// 整个词敲完之后，上屏它要不要多按一键：首选就是它（空格选中）或根本没有候选（空格原样上屏）时不用，
    /// 否则空格会上屏别的词，只能按回车原样上屏（或数字选它，同样 1 键）。只对敲的就是目标词本身的记录有意义。
    pub fn needs_extra_key(&self) -> bool {
        self.final_rank() != Some(0) && self.top.is_some()
    }
}
