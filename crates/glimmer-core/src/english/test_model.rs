//! 测试用的小英文语言模型：概率与后继次数都直接写在表里，三种上文按 trait 文档的规则查。

use std::collections::HashMap;

use crate::english::NgramHistory;
use crate::sentence::{Context, LanguageModel};

/// 手写概率的 n-gram 模型：一元概率、二元 / 三元的（概率, 次数）。查不到的接续退到下一层。
#[derive(Debug, Default)]
pub(crate) struct TableModel {
    /// token → 一元概率。
    unigram: HashMap<String, f64>,

    /// (前一个词或 `<s>`, 词) → (概率, 次数)。
    bigram: HashMap<(String, String), (f64, u32)>,

    /// (再前一个词或 `<s>`, 前一个词, 词) → (概率, 次数)。
    trigram: HashMap<(String, String, String), (f64, u32)>,
}

impl TableModel {
    /// 一元概率表。
    pub(crate) fn new(unigram: &[(&str, f64)]) -> Self {
        Self {
            unigram: unigram.iter().map(|(w, p)| ((*w).to_owned(), *p)).collect(),
            ..Self::default()
        }
    }

    /// 加一条二元。
    pub(crate) fn bigram(
        mut self,
        previous: &str,
        word: &str,
        probability: f64,
        count: u32,
    ) -> Self {
        self.bigram
            .insert((previous.to_owned(), word.to_owned()), (probability, count));
        self
    }

    /// 加一条三元。
    pub(crate) fn trigram(
        mut self,
        earlier: &str,
        previous: &str,
        word: &str,
        probability: f64,
        count: u32,
    ) -> Self {
        self.trigram.insert(
            (earlier.to_owned(), previous.to_owned(), word.to_owned()),
            (probability, count),
        );
        self
    }

    /// 前一个词的全部二元后继。
    fn bigram_after(&self, previous: &str) -> Vec<(String, u32)> {
        self.bigram
            .iter()
            .filter(|((p, _), _)| p == previous)
            .map(|((_, w), (_, c))| (w.clone(), *c))
            .collect()
    }

    /// 只知道前一个词时的概率：二元，没有就一元。
    fn after(&self, previous: &str, word: &str) -> Option<f64> {
        let unigram = *self.unigram.get(word)?;
        Some(
            self.bigram
                .get(&(previous.to_owned(), word.to_owned()))
                .map_or(unigram, |(p, _)| *p),
        )
    }
}

impl LanguageModel for TableModel {
    fn log_prob(&self, context: Context<'_>, word: &str) -> Option<f64> {
        let previous = context.previous.unwrap_or("<s>");
        if let Some(prev) = context.previous {
            let earlier = context.earlier.unwrap_or("<s>");
            if let Some((p, _)) =
                self.trigram
                    .get(&(earlier.to_owned(), prev.to_owned(), word.to_owned()))
            {
                return Some(p.ln());
            }
        }
        self.after(previous, word).map(f64::ln)
    }

    fn log_probs(&self, history: NgramHistory<'_>, words: &[&str]) -> Vec<Option<f64>> {
        words
            .iter()
            .map(|word| match history {
                NgramHistory::Unknown => self.unigram.get(*word).map(|p| p.ln()),
                NgramHistory::Previous(previous) => self.after(previous, word).map(f64::ln),
                NgramHistory::Context(context) => self.log_prob(context, word),
            })
            .collect()
    }

    fn successors(
        &self,
        history: NgramHistory<'_>,
        keep: &mut dyn FnMut(&str) -> bool,
    ) -> Vec<(String, u32)> {
        let mut out: Vec<(String, u32)> = match history {
            NgramHistory::Unknown => Vec::new(),
            NgramHistory::Previous(previous) => self.bigram_after(previous),
            NgramHistory::Context(context) => match context.previous {
                None => self.bigram_after("<s>"),
                Some(previous) => {
                    let earlier = context.earlier.unwrap_or("<s>");
                    self.trigram
                        .iter()
                        .filter(|((e, p, _), _)| e == earlier && p == previous)
                        .map(|((_, _, w), (_, c))| (w.clone(), *c))
                        .collect()
                }
            },
        };
        out.retain(|(word, _)| keep(word));
        out
    }
}
