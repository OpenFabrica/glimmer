//! 下一词预测组：命中计 1 键且上文前移、不中时与补全组一样、没有模型时整组与补全组一样、连续命中怎么分段。

use glimmer_core::english::NgramHistory;
use glimmer_core::sentence::{Context, LanguageModel};

use super::{WORDS, engine, options, temp_file};
use crate::eval::english::predict::Streak;
use crate::eval::english::{Group, Report, run};

/// 只认几条二元接续的语言模型：(前一个词, 下一个词, 概率)，别的一律不认识（引擎按词频兜底）。
struct Bigrams(&'static [(&'static str, &'static str, f64)]);

impl LanguageModel for Bigrams {
    /// 只看前一个词。
    fn log_prob(&self, context: Context<'_>, word: &str) -> Option<f64> {
        self.0
            .iter()
            .find(|(previous, next, _)| context.previous == Some(*previous) && *next == word)
            .map(|(_, _, probability)| probability.ln())
    }

    /// 三元、二元两层都给同一份：前一个词的接续。
    fn successors(
        &self,
        history: NgramHistory<'_>,
        keep: &mut dyn FnMut(&str) -> bool,
    ) -> Vec<(String, u32)> {
        let Some(previous) = history.previous() else {
            return Vec::new();
        };
        self.0
            .iter()
            .filter(|(from, next, _)| *from == previous && keep(next))
            .map(|(_, next, _)| ((*next).to_owned(), 10))
            .collect()
    }
}

/// 在小词表引擎上跑补全组与预测组；`model` 给了就装上英文语言模型。
fn simulate(text: &str, model: Option<Bigrams>, name: &str) -> Report {
    let (engine, list) = engine(WORDS);
    let mut engine = match model {
        Some(model) => engine.with_english_lm(Box::new(model)),
        None => engine,
    };
    let path = temp_file(name, text);
    let report = run(
        &mut engine,
        Some(&list),
        &options(vec![path.clone()], vec![Group::Complete, Group::Predict]),
    )
    .unwrap();
    std::fs::remove_file(path).ok();
    report
}

/// 含预测的两行与补全组的两行逐项相同。
fn assert_same_as_complete(report: &Report) {
    let predict = &report.predict;
    for (with, without) in [(predict.top1, report.top1), (predict.top3, report.top3)] {
        assert_eq!(
            (with.words, with.letters, with.article, with.actual),
            (
                without.words,
                without.letters,
                without.article,
                without.actual
            )
        );
    }
}

#[test]
fn a_hit_costs_one_key_and_moves_the_context_on() {
    // hello 之后猜 world、world 之后猜 hello：第二次能猜中，说明接受之后上文真的前移到了 world
    let model = Bigrams(&[("hello", "world", 0.8), ("world", "hello", 0.8)]);
    let report = simulate("hello world hello\n", Some(model), "hit.txt");
    let predict = &report.predict;
    assert_eq!(predict.positions, 3);
    // 第一个词之前不知道上文，不出；后两个都出、都中
    assert_eq!((predict.eligible, predict.shown, predict.hits), (2, 2, 2));
    assert_eq!(predict.top3_hits, 2);
    // 第一个 hello 照补全组敲（敲 1 个字母就是首选：2 键），后两个各 1 键
    assert_eq!(predict.top1.letters, 15);
    assert_eq!((predict.top1.article, predict.top1.actual), (4, 4));
    assert_eq!((predict.top3.article, predict.top3.actual), (4, 4));
    // 中间只隔 Tab 带出的空格：一段长 2
    assert_eq!(predict.streaks.get(&2), Some(&1));
    assert_eq!(predict.streaks.len(), 1);
    assert_eq!(predict.timing.count(), 2);
    let text = report.to_string();
    assert!(
        text.contains("第一名命中  2 / 2（精确率 100.0%）"),
        "{text}"
    );
}

#[test]
fn a_miss_counts_keys_like_the_completion_group() {
    // 猜的是 word，原文是 world：出了、不中，照敲，不额外计键
    let model = Bigrams(&[("hello", "word", 0.9)]);
    let report = simulate("hello world\n", Some(model), "miss.txt");
    let predict = &report.predict;
    assert_eq!((predict.shown, predict.hits, predict.top3_hits), (1, 0, 0));
    assert!(predict.streaks.is_empty());
    assert_same_as_complete(&report);
}

#[test]
fn without_a_model_the_group_matches_the_completion_group() {
    let report = simulate("hello world, help held word hello\n", None, "no-model.txt");
    assert_eq!(report.predict.shown, 0);
    assert_eq!(report.predict.positions, report.words);
    assert_same_as_complete(&report);
}

#[test]
fn streaks_join_only_across_the_tab_space() {
    let mut streak = Streak::default();
    assert_eq!(streak.accept(true, true), None);
    assert_eq!(streak.accept(true, false), None);
    // 上一个词后面跟了标点：接不上，交出长 2 的一段，再从 1 数
    assert_eq!(streak.accept(true, true), Some(2));
    // 这个词前面有标点：同样接不上
    assert_eq!(streak.accept(false, true), Some(1));
    assert_eq!(streak.end(), Some(1));
    assert_eq!(streak.end(), None);
}
