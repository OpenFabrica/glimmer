//! 后继缓存：同一上文同一首字母只扫一次，换了就重扫；取的时候三元先占名额。

use std::sync::atomic::{AtomicUsize, Ordering};

use super::*;
use crate::english::test_model::TableModel;
use crate::sentence::Context;

/// 数一数后继被扫了几次的模型。
struct Counting {
    /// 真正给数据的模型。
    inner: TableModel,

    /// `successors` 被调了几次。
    calls: AtomicUsize,
}

impl LanguageModel for Counting {
    fn log_prob(&self, context: Context<'_>, word: &str) -> Option<f64> {
        self.inner.log_prob(context, word)
    }

    fn successors(
        &self,
        history: NgramHistory<'_>,
        keep: &mut dyn FnMut(&str) -> bool,
    ) -> Vec<(String, u32)> {
        self.calls.fetch_add(1, Ordering::Relaxed);
        self.inner.successors(history, keep)
    }
}

/// the 后面接 then / there / they / a，i 后面接 think。
fn model() -> Counting {
    Counting {
        inner: TableModel::new(&[("then", 0.1), ("there", 0.1), ("they", 0.1), ("a", 0.1)])
            .bigram("the", "then", 0.1, 5)
            .bigram("the", "there", 0.1, 9)
            .bigram("the", "they", 0.1, 7)
            .bigram("the", "a", 0.1, 3)
            .bigram("i", "think", 0.1, 20)
            .trigram("over", "the", "they", 0.1, 4),
        calls: AtomicUsize::new(0),
    }
}

#[test]
fn one_scan_per_history_and_first_letter() {
    let model = model();
    let mut cache = SuccessorCache::default();
    let history = NgramHistory::Previous("the");
    for code in ["t", "th", "the", "ther"] {
        cache.prepare(&model, history, code);
    }
    assert_eq!(model.calls.load(Ordering::Relaxed), 1);
    let tokens = |cache: &SuccessorCache, prefix: &str| -> Vec<String> {
        cache
            .top(prefix)
            .iter()
            .map(|s| s.token().to_owned())
            .collect()
    };
    // 按次数排；首字母对不上的没进缓存
    assert_eq!(tokens(&cache, "th"), ["there", "they", "then"]);
    assert_eq!(tokens(&cache, "they"), ["they"]);
    assert!(tokens(&cache, "a").is_empty());
    // 换了首字母、换了上文都重扫
    cache.prepare(&model, history, "a");
    assert_eq!(tokens(&cache, "a"), ["a"]);
    cache.prepare(&model, NgramHistory::Previous("i"), "t");
    assert_eq!(tokens(&cache, "t"), ["think"]);
    assert_eq!(model.calls.load(Ordering::Relaxed), 3);
}

#[test]
fn trigram_successors_come_first_then_bigram_ones_without_repeats() {
    let model = model();
    let mut cache = SuccessorCache::default();
    let history = NgramHistory::Context(Context::after_two("over", "the"));
    cache.prepare(&model, history, "t");
    let tokens: Vec<&str> = cache.top("t").iter().map(|s| s.token()).collect();
    assert_eq!(tokens, ["they", "there", "then"]);
    // 不知道上文：没有后继
    cache.prepare(&model, NgramHistory::Unknown, "t");
    assert!(cache.top("t").is_empty());
}
