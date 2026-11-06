//! 批量打分与逐词打分对拍、三种上文的区别、按上文列后继。

use glimmer_core::english::NgramHistory;
use glimmer_core::sentence::{Context, LanguageModel};

use crate::model::NgramModel;
use crate::smoothing::{BackoffMode, Smoothing};

const UNIGRAM: &str = "<s>\t100\ni\t60\nthink\t30\nso\t25\nwe\t20\nthe\t80\ncat\t5\n";
const BIGRAM: &str = "<s>\ti\t40\n<s>\twe\t10\ni\tthink\t20\nthink\tso\t12\nwe\tthink\t6\nthe\tcat\t4\nthink\tthe\t3\n";
const TRIGRAM: &str = "<s>\ti\tthink\t15\ni\tthink\tso\t9\nwe\tthink\tthe\t2\n";

/// 测试用小模型。
fn model(smoothing: Smoothing) -> NgramModel {
    let mut model = NgramModel::parse(UNIGRAM, BIGRAM, TRIGRAM).unwrap();
    model.set_smoothing(smoothing);
    model
}

const WORDS: [&str; 8] = ["i", "think", "so", "we", "the", "cat", "dog", "<s>"];

#[test]
fn batch_scores_equal_word_by_word_scores_for_every_context() {
    for smoothing in [
        Smoothing::DEFAULT,
        Smoothing::LEGACY,
        Smoothing {
            mode: BackoffMode::Absolute,
            ..Smoothing::DEFAULT
        },
        Smoothing {
            use_trigram: false,
            ..Smoothing::DEFAULT
        },
    ] {
        let model = model(smoothing);
        for context in [
            Context::START,
            Context::after("i"),
            Context::after("think"),
            Context::after_two("i", "think"),
            Context::after_two("we", "think"),
            Context::after_two("mars", "think"),
            Context::after("mars"),
        ] {
            let batch = model.log_probs(NgramHistory::Context(context), &WORDS);
            for (word, got) in WORDS.iter().zip(batch) {
                let want = model.log_prob(context, word);
                match (got, want) {
                    (Some(a), Some(b)) => assert!((a - b).abs() < 1e-12, "{context:?} {word}"),
                    (a, b) => assert_eq!(a, b, "{context:?} {word}"),
                }
            }
        }
    }
}

#[test]
fn unknown_history_is_plain_unigram_and_differs_from_sentence_start() {
    let model = model(Smoothing::DEFAULT);
    let unknown = model.log_probs(NgramHistory::Unknown, &["i", "the", "cat", "dog"]);
    // 不知道上文：最大似然一元 c(w)/N，N 不含句首标记
    let total = 60.0 + 30.0 + 25.0 + 20.0 + 80.0 + 5.0;
    assert!((unknown[0].unwrap() - (60.0_f64 / total).ln()).abs() < 1e-12);
    assert!((unknown[1].unwrap() - (80.0_f64 / total).ln()).abs() < 1e-12);
    assert_eq!(unknown[3], None);
    // 不知道上文时 the 比 i 常见；按句首算 i 远强于 the
    assert!(unknown[1] > unknown[0]);
    let start = model.log_probs(NgramHistory::START, &["i", "the"]);
    assert!(start[0] > start[1]);
}

#[test]
fn previous_only_history_uses_the_bigram_without_a_start_marker() {
    let full = model(Smoothing::DEFAULT);
    // 只知道 think：只走二元
    let previous = full.log_probs(NgramHistory::Previous("think"), &["so"]);
    let bigram_only = model(Smoothing {
        use_trigram: false,
        ..Smoothing::DEFAULT
    });
    assert_eq!(
        previous[0],
        bigram_only.log_prob(Context::after("think"), "so")
    );
    // 「think 在句首」会补句首标记走三元 (<s>, think)，那条二元不存在，结果同样是二元；
    // 「i think」的三元比二元高
    let after_i = full.log_probs(
        NgramHistory::Context(Context::after_two("i", "think")),
        &["so"],
    );
    assert!(after_i[0] > previous[0]);
    // 前一个词模型不认识：只剩一元（接续概率那种）
    let mars = full.log_probs(NgramHistory::Previous("mars"), &["so"]);
    assert_eq!(mars[0], full.log_prob(Context::after("mars"), "so"));
}

#[test]
fn successors_list_the_addressed_table_and_filter_by_the_caller() {
    let model = model(Smoothing::DEFAULT);
    let mut all = |_: &str| true;
    let sorted = |mut list: Vec<(String, u32)>| {
        list.sort();
        list
    };
    // 句首的二元后继
    assert_eq!(
        sorted(model.successors(NgramHistory::START, &mut all)),
        [("i".to_owned(), 40), ("we".to_owned(), 10)]
    );
    // 只知道前一个词：二元
    assert_eq!(
        sorted(model.successors(NgramHistory::Previous("think"), &mut all)),
        [("so".to_owned(), 12), ("the".to_owned(), 3)]
    );
    // 前两个词：只看三元那层
    assert_eq!(
        model.successors(
            NgramHistory::Context(Context::after_two("i", "think")),
            &mut all
        ),
        [("so".to_owned(), 9)]
    );
    // 前一个词在句首：三元 (<s>, i)
    assert_eq!(
        model.successors(NgramHistory::Context(Context::after("i")), &mut all),
        [("think".to_owned(), 15)]
    );
    // 调用方的过滤：只要 t 开头的
    let mut t = |word: &str| word.starts_with('t');
    assert_eq!(
        model.successors(NgramHistory::Previous("think"), &mut t),
        [("the".to_owned(), 3)]
    );
    assert!(model.successors(NgramHistory::Unknown, &mut all).is_empty());
    assert!(
        model
            .successors(NgramHistory::Previous("mars"), &mut all)
            .is_empty()
    );
    assert!(
        model
            .successors(
                NgramHistory::Context(Context::after_two("cat", "think")),
                &mut all
            )
            .is_empty()
    );
}
