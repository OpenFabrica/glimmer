//! 一组词的击键累计与省键率。

use super::Keys;

/// 省键率 = 1 − 实际击键 / 词的字母数，按全文累计（不是每词算完再平均）。
#[derive(Debug, Clone, Copy, Default)]
pub struct Savings {
    /// 评了几个词。
    pub words: usize,

    /// 这些词的字母数之和（含撇号）。
    pub letters: usize,

    /// 文章口径的击键之和。
    pub article: usize,

    /// 实际按键的击键之和。
    pub actual: usize,
}

impl Savings {
    /// 记一个 `letters` 个字母的词花了 `keys`。
    pub fn add(&mut self, letters: usize, keys: Keys) {
        self.words += 1;
        self.letters += letters;
        self.article += keys.article;
        self.actual += keys.actual;
    }

    /// 文章口径的省键率；没有词为 `None`。
    pub fn article_rate(&self) -> Option<f64> {
        rate(self.article, self.letters)
    }

    /// 实际按键的省键率，可以为负；没有词为 `None`。
    pub fn actual_rate(&self) -> Option<f64> {
        rate(self.actual, self.letters)
    }
}

/// `1 − keys / letters`。
fn rate(keys: usize, letters: usize) -> Option<f64> {
    (letters > 0).then(|| 1.0 - keys as f64 / letters as f64)
}
