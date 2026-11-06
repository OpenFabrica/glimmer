//! 扫语料攒一元 / 二元 / 三元计数。
//!
//! 二元表全放内存（几千万条，撑得住），三元表放不下：同样的语料三元的种类是二元的十几倍，
//! 而且绝大多数只出现一次。所以三元表设一个条数上限，到了就把「不超过阈值」的条目整批扔掉、阈值再加一，
//! 直到降到上限一半以下。代价是语料后半段才热起来的三元可能被前半段的阈值误杀——
//! 反正输出还要过一遍 `min_trigram_count`，被误杀的都是计数远低于阈值的。

use std::collections::HashMap;

/// 语料里的 n-gram 计数。词用编号（中文是 [`Vocabulary`] 的编号，英文是 `english-ngram` 的词表编号），0 号是句首标记。
///
/// [`Vocabulary`]: super::Vocabulary
pub(crate) struct Counts {
    /// 三元表最多留多少条，到了就剪（`--max-trigram-entries`）。
    capacity: usize,

    /// 编号 → 一元计数。
    pub(crate) unigram: Vec<u64>,

    /// (前词 << 32 | 后词) → 二元计数。
    pub(crate) bigram: HashMap<u64, u32>,

    /// (前二词 << 64 | 前词 << 32 | 后词) → 三元计数。
    pub(crate) trigram: HashMap<u128, u32>,

    /// 三元表上一次整批剪枝用的阈值：不超过它的条目已经被扔过一次。
    threshold: u32,

    /// 剪过几次，只用来记日志。
    prunes: usize,
}

impl Counts {
    /// 词数定下一元表的长度，`capacity` 是三元表的条数上限。
    pub(crate) fn new(words: usize, capacity: usize) -> Self {
        Self {
            capacity: capacity.max(2),
            unigram: vec![0; words],
            bigram: HashMap::new(),
            trigram: HashMap::new(),
            threshold: 0,
            prunes: 0,
        }
    }

    /// 记一个句子（一段连续汉字切出来的 token，`None` 是词库里没有的字）。
    ///
    /// 句首标记自己算一次一元，第一个词的前词是它；三元从第二个词起记（上文 `(<s>, 第一个词)`），
    /// 与模型里「句首词没有前二词」对得上。遇到 `None` 前后的 n-gram 关系都断开。
    pub(crate) fn record(&mut self, tokens: &[Option<u32>]) {
        self.unigram[0] += 1;
        let mut previous: Option<u32> = Some(0);
        let mut earlier: Option<u32> = None;
        for token in tokens {
            let Some(&id) = token.as_ref() else {
                previous = None;
                earlier = None;
                continue;
            };
            self.unigram[id as usize] += 1;
            if let Some(p) = previous {
                *self.bigram.entry(bigram_key(p, id)).or_insert(0) += 1;
                if let Some(e) = earlier {
                    *self.trigram.entry(trigram_key(e, p, id)).or_insert(0) += 1;
                }
            }
            earlier = previous;
            previous = Some(id);
        }
    }

    /// 三元表超了就整批剪：扔掉计数不超过阈值的，还不够就抬阈值再扔。
    pub(crate) fn prune_trigrams(&mut self) {
        if self.trigram.len() <= self.capacity {
            return;
        }
        let before = self.trigram.len();
        while self.trigram.len() > self.capacity / 2 {
            self.threshold += 1;
            let threshold = self.threshold;
            self.trigram.retain(|_, count| *count > threshold);
        }
        self.prunes += 1;
        tracing::info!(
            before,
            after = self.trigram.len(),
            threshold = self.threshold,
            prunes = self.prunes,
            "三元表超上限，已整批剪枝"
        );
    }

    /// 剪枝抬到的阈值：输出时的 `min_trigram_count` 不该低于它，否则表里留下的和扔掉的不是一个标准。
    pub(crate) fn threshold(&self) -> u32 {
        self.threshold
    }
}

/// 二元的键。
pub(super) fn bigram_key(first: u32, second: u32) -> u64 {
    (u64::from(first) << 32) | u64::from(second)
}

/// 三元的键。
pub(super) fn trigram_key(earlier: u32, previous: u32, word: u32) -> u128 {
    (u128::from(earlier) << 64) | u128::from(bigram_key(previous, word))
}

/// 三元的键拆回三个编号。
pub(super) fn trigram_parts(key: u128) -> (u32, u32, u32) {
    (
        (key >> 64) as u32,
        ((key >> 32) & 0xFFFF_FFFF) as u32,
        (key & 0xFFFF_FFFF) as u32,
    )
}
