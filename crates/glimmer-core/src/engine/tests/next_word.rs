//! 英文下一词预测：什么时候出、什么时候不出，Tab 接受之后上文怎么走，会话不串，同一个上文不重扫。

use std::collections::HashMap;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use super::{CountingLearner, MemoryLogger, engine};
use crate::engine::input_log::{InputLogEntry, InputSource};
use crate::english::test_model::TableModel;
use crate::english::{NgramHistory, PredictParams};
use crate::sentence::{Context, LanguageModel};
use crate::{Candidate, CandidateKind, Engine, EngineSession};
use glimmer_dictionary::WordList;

const WORDS: &str = "thank\tthank\t5000\nyou\tyou\t6800\nso\tso\t6200\nmuch\tmuch\t6000\n\
                     want\twant\t5600\nto\tto\t7300\nI\ti\t7000\nhello\thello\t5000\n";

/// thank → you → so → much 一路接下去；want → to；so 后面单独看是 i；hello 后面什么都拿不准。
fn model() -> TableModel {
    TableModel::new(&[
        ("thank", 0.001),
        ("you", 0.02),
        ("so", 0.01),
        ("much", 0.005),
        ("want", 0.003),
        ("to", 0.03),
        ("i", 0.03),
        ("hello", 0.001),
    ])
    .bigram("thank", "you", 0.9, 900)
    .bigram("you", "so", 0.2, 100)
    .bigram("so", "i", 0.35, 200)
    .bigram("so", "much", 0.25, 150)
    .bigram("want", "to", 0.7, 700)
    .bigram("hello", "you", 0.05, 5)
    .trigram("thank", "you", "so", 0.5, 300)
    .trigram("you", "so", "much", 0.8, 250)
}

/// 英文模式、装了词表与模型的引擎。
fn predicting_engine() -> Engine {
    let mut e = engine()
        .with_english(WordList::parse(WORDS).unwrap())
        .with_english_lm(Box::new(model()));
    e.set_english_mode(true);
    e
}

/// 选中一个英文候选上屏（不经查询，直接造候选）。
fn commit_word(e: &mut Engine, word: &str) {
    e.set_input(&word.to_ascii_lowercase());
    e.commit(&Candidate {
        text: word.to_owned(),
        kind: CandidateKind::English,
        syllables: Vec::new(),
        reading: None,
        translation: None,
    });
}

/// 当前预测的词。
fn predicted(e: &Engine) -> Vec<String> {
    e.english_predictions()
        .items
        .into_iter()
        .map(|c| c.text)
        .collect()
}

#[test]
fn predictions_appear_only_right_after_a_space() {
    let mut e = predicting_engine();
    commit_word(&mut e, "thank");
    // 词刚上屏、还没敲空格
    assert!(predicted(&e).is_empty());
    e.note_passthrough(' ');
    assert_eq!(predicted(&e), ["you"]);
    let list = e.english_predictions();
    assert_eq!(list.items[0].kind, CandidateKind::English);
    // 连敲两个空格仍然算
    e.note_passthrough(' ');
    assert_eq!(predicted(&e), ["you"]);
    // 敲了一个字母：组句不空，预测收起；Esc 清掉组句之后也不回来（最后一个事件是那个字母）
    e.push('y');
    assert!(predicted(&e).is_empty());
    e.clear();
    assert!(predicted(&e).is_empty());
}

#[test]
fn a_comma_keeps_the_context_but_a_sentence_end_or_a_break_does_not() {
    let mut e = predicting_engine();
    commit_word(&mut e, "thank");
    e.note_passthrough(',');
    // 逗号之后还没敲空格
    assert!(predicted(&e).is_empty());
    e.note_passthrough(' ');
    assert_eq!(predicted(&e), ["you"]);
    // 句号之后是句首，这次不出
    commit_word(&mut e, "thank");
    e.note_passthrough('.');
    e.note_passthrough(' ');
    assert!(predicted(&e).is_empty());
    // 方向键这类功能键（壳调 break_english_context）、组句外退格之后都不出
    commit_word(&mut e, "thank");
    e.note_passthrough(' ');
    e.break_english_context();
    assert!(predicted(&e).is_empty());
    commit_word(&mut e, "thank");
    e.note_passthrough(' ');
    e.note_backspace();
    e.note_passthrough(' ');
    assert!(predicted(&e).is_empty());
    // 什么都不知道的时候敲空格也不出
    let mut fresh = predicting_engine();
    fresh.note_passthrough(' ');
    assert!(predicted(&fresh).is_empty());
}

#[test]
fn predictions_expire_with_the_context() {
    let mut e = predicting_engine();
    commit_word(&mut e, "thank");
    e.note_passthrough(' ');
    e.english_context.backdate(Duration::from_secs(4));
    assert_eq!(predicted(&e), ["you"]);
    e.english_context.backdate(Duration::from_secs(2));
    assert!(predicted(&e).is_empty());
    assert_eq!(e.accept_english_prediction(0), None);
}

#[test]
fn accepting_commits_the_word_with_a_space_and_predicts_again() {
    let log = Arc::new(Mutex::new(Vec::new()));
    let mut e = engine()
        .with_english(WordList::parse(WORDS).unwrap())
        .with_english_lm(Box::new(model()))
        .with_learner(Box::new(CountingLearner(HashMap::new())))
        .with_input_logger(Box::new(MemoryLogger(log.clone())));
    e.set_english_mode(true);
    commit_word(&mut e, "thank");
    e.note_passthrough(' ');
    // 序号越界：什么都不做
    assert_eq!(e.accept_english_prediction(1), None);
    assert_eq!(
        e.english_history(),
        NgramHistory::Previous("thank"),
        "越界不动上文"
    );
    assert_eq!(e.accept_english_prediction(0).as_deref(), Some("you "));
    // 上文前移一个词，处于刚敲过空格：马上有下一个，可以连按
    assert_eq!(
        e.english_history(),
        NgramHistory::Context(Context::after_two("thank", "you"))
    );
    assert!(e.english_context().after_space());
    assert_eq!(predicted(&e), ["so"]);
    assert_eq!(e.accept_english_prediction(0).as_deref(), Some("so "));
    assert_eq!(e.accept_english_prediction(0).as_deref(), Some("much "));
    // 个人词频记了一次，和选中英文候选一样
    assert_eq!(e.learner().weight("you"), 1);
    assert_eq!(e.learner().weight("so"), 1);
    // 输入日志：来源是预测、没有键，后面跟着直通的空格
    let entries = log.lock().unwrap();
    let commits: Vec<_> = entries
        .iter()
        .filter_map(|entry| match entry {
            InputLogEntry::Commit(commit) if commit.source == InputSource::EnglishPrediction => {
                Some(commit)
            }
            _ => None,
        })
        .collect();
    assert_eq!(commits.len(), 3);
    assert_eq!(commits[0].text, "you");
    assert!(commits[0].keys.is_empty());
    assert!(commits[0].top.is_empty());
}

#[test]
fn predicted_words_use_the_word_list_spelling() {
    let mut e = predicting_engine();
    commit_word(&mut e, "hello");
    e.note_passthrough(' ');
    commit_word(&mut e, "so");
    e.note_passthrough(' ');
    // (hello, so) 没有三元，退到 so 的二元：i 比 much 高，显示成 I
    assert_eq!(predicted(&e), ["I"]);
    assert_eq!(e.accept_english_prediction(0).as_deref(), Some("I "));
    assert_eq!(
        e.english_history(),
        NgramHistory::Context(Context::after_two("so", "i"))
    );
}

#[test]
fn a_weak_top_word_or_any_switch_off_predicts_nothing() {
    // hello 之后最好的 you 只有 0.05，不到缺省下限 0.3
    let mut e = predicting_engine();
    commit_word(&mut e, "hello");
    e.note_passthrough(' ');
    assert!(predicted(&e).is_empty());
    e.set_english_predict_params(PredictParams {
        min_probability: 0.01,
        ..PredictParams::DEFAULT
    });
    assert_eq!(predicted(&e), ["you"]);

    // 开关关掉
    let mut e = predicting_engine();
    commit_word(&mut e, "thank");
    e.note_passthrough(' ');
    e.set_english_prediction(false);
    assert!(predicted(&e).is_empty());
    assert_eq!(e.accept_english_prediction(0), None);
    e.set_english_prediction(true);
    assert_eq!(predicted(&e), ["you"]);
    // 中文模式（`english_candidates` 关着时壳也不开 Engine 的英文模式，是同一种情形）
    e.set_english_mode(false);
    assert!(predicted(&e).is_empty());

    // 没有英文模型：永远不出
    let mut plain = engine().with_english(WordList::parse(WORDS).unwrap());
    plain.set_english_mode(true);
    commit_word(&mut plain, "thank");
    plain.note_passthrough(' ');
    assert!(predicted(&plain).is_empty());
    assert_eq!(plain.accept_english_prediction(0), None);

    // 显示几个由参数定
    let mut e = predicting_engine();
    e.set_english_predict_params(PredictParams {
        count: 3,
        ..PredictParams::DEFAULT
    });
    commit_word(&mut e, "you");
    e.note_passthrough(' ');
    commit_word(&mut e, "so");
    e.note_passthrough(' ');
    assert_eq!(predicted(&e), ["much", "I"]);
}

#[test]
fn sessions_keep_their_own_predictions() {
    let mut e = predicting_engine();
    commit_word(&mut e, "thank");
    e.note_passthrough(' ');
    let mut other = EngineSession::default();
    e.swap_session(&mut other);
    // 另一个应用：没有上文
    assert!(predicted(&e).is_empty());
    e.set_english_mode(true);
    commit_word(&mut e, "want");
    e.note_passthrough(' ');
    assert_eq!(predicted(&e), ["to"]);
    e.swap_session(&mut other);
    assert_eq!(predicted(&e), ["you"]);
    // 丢掉会话输入之后也没有
    e.discard_input();
    e.set_english_mode(true);
    assert!(predicted(&e).is_empty());
}

/// 数后继被扫了几次的模型，计数放在外面，装进引擎之后还看得到。
struct Counting {
    /// 真正给数据的模型。
    inner: TableModel,

    /// `successors` 被调了几次。
    calls: Arc<AtomicUsize>,
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
fn polling_the_same_context_scans_successors_once() {
    let calls = Arc::new(AtomicUsize::new(0));
    let mut e = engine()
        .with_english(WordList::parse(WORDS).unwrap())
        .with_english_lm(Box::new(Counting {
            inner: model(),
            calls: calls.clone(),
        }));
    e.set_english_mode(true);
    commit_word(&mut e, "thank");
    e.note_passthrough(' ');
    for _ in 0..10 {
        assert_eq!(predicted(&e), ["you"]);
    }
    assert_eq!(calls.load(Ordering::Relaxed), 1);
    // 接受之后是新上文（三元 + 二元两次扫），接着轮询不再扫
    e.accept_english_prediction(0);
    for _ in 0..10 {
        assert_eq!(predicted(&e), ["so"]);
    }
    assert_eq!(calls.load(Ordering::Relaxed), 3);
}

#[test]
fn private_input_predicts_nothing_and_resets_the_context_both_ways() {
    let mut e = predicting_engine();
    commit_word(&mut e, "thank");
    e.note_passthrough(' ');
    // 进私密：不出、Tab 不接，进去之前的词也作废
    e.set_private(true);
    assert!(predicted(&e).is_empty());
    assert_eq!(e.accept_english_prediction(0), None);
    assert_eq!(e.english_history(), NgramHistory::Unknown);
    // 私密里照样敲词加空格：还是不出
    commit_word(&mut e, "want");
    e.note_passthrough(' ');
    assert!(predicted(&e).is_empty());
    // 出私密：里面敲的 want 不当上文，要重新敲词加空格才出
    e.set_private(false);
    assert_eq!(e.english_history(), NgramHistory::Unknown);
    e.note_passthrough(' ');
    assert!(predicted(&e).is_empty());
    commit_word(&mut e, "thank");
    e.note_passthrough(' ');
    assert_eq!(predicted(&e), ["you"]);
    // 值没变的重复调用（壳每次聚焦都报）不打断
    e.set_private(false);
    assert_eq!(predicted(&e), ["you"]);
    assert_eq!(e.accept_english_prediction(0).as_deref(), Some("you "));
}
