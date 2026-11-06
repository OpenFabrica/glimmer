//! 下一词预测：候选来源、写法、阈值、个数、截断不改第一名、缓存。

use std::sync::atomic::{AtomicUsize, Ordering};

use glimmer_dictionary::WordList;

use super::{NextWord, PredictParams, PredictionCache, predict, predict_within};
use crate::english::NgramHistory;
use crate::english::test_model::TableModel;
use crate::sentence::{Context, LanguageModel};

/// 小词表：`I` 的 token 是 `i`；`us` / `US` 同码，`us` 词频高。
const WORDS: &str = "I\ti\t7000\nthink\tthink\t5500\nwant\twant\t5600\nto\tto\t7300\nso\tso\t6200\n\
                     you\tyou\t6800\nthat\tthat\t6900\nus\tus\t6000\nUS\tus\t4500\n";

/// 前词 `want` 的二元后继 to / you / so / i / us，加上 (`i`, `want`) 的三元后继 to / that；`zzz` 是词表外的 token。
fn model() -> TableModel {
    TableModel::new(&[
        ("to", 0.02),
        ("you", 0.01),
        ("so", 0.005),
        ("i", 0.01),
        ("us", 0.001),
        ("that", 0.01),
        ("zzz", 0.3),
    ])
    .bigram("want", "to", 0.5, 900)
    .bigram("want", "you", 0.1, 200)
    .bigram("want", "so", 0.01, 10)
    .bigram("want", "us", 0.05, 40)
    .bigram("so", "i", 0.4, 300)
    .bigram("so", "zzz", 0.6, 999)
    .trigram("i", "want", "to", 0.7, 500)
    .trigram("i", "want", "that", 0.15, 30)
}

/// 词表。
fn list() -> WordList {
    WordList::parse(WORDS).unwrap()
}

/// 前 `count` 个、概率不设下限的参数。
fn params(count: usize) -> PredictParams {
    PredictParams {
        min_probability: 0.0,
        count,
    }
}

/// 只看词。
fn texts(words: &[NextWord]) -> Vec<&str> {
    words.iter().map(|w| w.text.as_str()).collect()
}

#[test]
fn two_words_merge_trigram_and_bigram_successors_ranked_by_probability() {
    let history = NgramHistory::Context(Context::after_two("i", "want"));
    let words = predict(&model(), &list(), history, &params(10));
    // to 两层都有只算一次；that 只在三元，you / us / so 只在二元；概率按带上文的打分（三元优先）
    assert_eq!(texts(&words), ["to", "that", "you", "us", "so"]);
    assert!((words[0].probability - 0.7).abs() < 1e-9);
    // 只知道前一个词：只有二元后继，that 不在里面
    let words = predict(
        &model(),
        &list(),
        NgramHistory::Previous("want"),
        &params(10),
    );
    assert_eq!(texts(&words), ["to", "you", "us", "so"]);
}

#[test]
fn tokens_come_back_in_the_word_list_spelling_and_unknown_tokens_are_skipped() {
    // so 之后：zzz 概率最高但不在词表里，跳过；i 用词表写法 I
    let words = predict(&model(), &list(), NgramHistory::Previous("so"), &params(3));
    assert_eq!(texts(&words), ["I"]);
    // 只差大小写的写法有两个：取词频高的 us
    let words = predict(
        &model(),
        &list(),
        NgramHistory::Previous("want"),
        &params(10),
    );
    assert!(texts(&words).contains(&"us"));
    assert!(!texts(&words).contains(&"US"));
}

#[test]
fn the_top_word_must_reach_the_threshold_and_count_caps_the_list() {
    let history = NgramHistory::Previous("want");
    let defaults = PredictParams::DEFAULT;
    assert_eq!(
        texts(&predict(&model(), &list(), history, &defaults)),
        ["to"]
    );
    let strict = PredictParams {
        min_probability: 0.6,
        ..defaults
    };
    assert!(predict(&model(), &list(), history, &strict).is_empty());
    // 下限只看第一名：后面的低于它照样给
    let three = PredictParams { count: 3, ..strict };
    let lenient = PredictParams {
        min_probability: 0.5,
        ..three
    };
    assert_eq!(
        texts(&predict(&model(), &list(), history, &lenient)),
        ["to", "you", "us"]
    );
    assert!(predict(&model(), &list(), history, &params(0)).is_empty());
}

#[test]
fn unknown_history_or_no_successors_predict_nothing() {
    let model = model();
    let list = list();
    assert!(predict(&model, &list, NgramHistory::Unknown, &params(3)).is_empty());
    assert!(predict(&model, &list, NgramHistory::Previous("think"), &params(3)).is_empty());
}

#[test]
fn truncating_successors_by_count_keeps_the_top_word() {
    // 四百条二元后继，次数越高概率越高（真模型里大体如此）；第一名在三元里、二元次数很低
    let mut words = String::new();
    let mut unigram: Vec<(String, f64)> = Vec::new();
    for index in 0..400 {
        let word = format!("w{index:03}");
        words.push_str(&format!("{word}\t{word}\t4000\n"));
        unigram.push((word, 0.001));
    }
    words.push_str("rare\trare\t3000\n");
    unigram.push(("rare".to_owned(), 0.0001));
    let refs: Vec<(&str, f64)> = unigram.iter().map(|(w, p)| (w.as_str(), *p)).collect();
    let mut model = TableModel::new(&refs);
    for index in 0..400u32 {
        let word = format!("w{index:03}");
        model = model.bigram(
            "the",
            &word,
            f64::from(400 - index) / 100_000.0,
            400 - index,
        );
    }
    model = model
        .bigram("the", "rare", 0.000_01, 1)
        .trigram("of", "the", "rare", 0.4, 50);
    let list = WordList::parse(&words).unwrap();
    for history in [
        NgramHistory::Previous("the"),
        NgramHistory::Context(Context::after_two("of", "the")),
    ] {
        let full = predict_within(&model, &list, history, &params(3), usize::MAX);
        let cut = predict_within(&model, &list, history, &params(3), 8);
        assert_eq!(full, cut, "{history:?}");
    }
    let top = predict(
        &model,
        &list,
        NgramHistory::Context(Context::after_two("of", "the")),
        &params(1),
    );
    assert_eq!(texts(&top), ["rare"]);
}

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

    fn log_probs(&self, history: NgramHistory<'_>, words: &[&str]) -> Vec<Option<f64>> {
        self.inner.log_probs(history, words)
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

#[test]
fn the_cache_scans_once_per_history() {
    let model = Counting {
        inner: model(),
        calls: AtomicUsize::new(0),
    };
    let list = list();
    let mut cache = PredictionCache::default();
    let history = NgramHistory::Previous("want");
    for _ in 0..5 {
        assert_eq!(texts(cache.get(&model, &list, history, &params(1))), ["to"]);
    }
    assert_eq!(model.calls.load(Ordering::Relaxed), 1);
    // 换了上文重算；清空后同一个上文也重算
    assert_eq!(
        texts(cache.get(&model, &list, NgramHistory::Previous("so"), &params(1))),
        ["I"]
    );
    cache.clear();
    cache.get(&model, &list, NgramHistory::Previous("so"), &params(1));
    assert_eq!(model.calls.load(Ordering::Relaxed), 3);
}
