//! 打分公式：词频换算、λ、个人份额、往池里加无关低分词不改前几名。

use super::*;

/// 池里的一个词：模型概率、代价、个人次数。
fn word(word: &'static str, model: f64, cost: f64, personal: u32) -> PoolWord<'static> {
    PoolWord {
        word,
        cost,
        frequency: None,
        personal,
        domain: false,
        literal: false,
        model,
    }
}

/// 按分数从高到低的词。
fn ranked(pool: &[PoolWord<'_>], personal_total: f64) -> Vec<String> {
    let scores = score(pool, personal_total, &ScoreParams::DEFAULT);
    let mut order: Vec<usize> = (0..pool.len()).collect();
    order.sort_by(|&a, &b| scores[b].total_cmp(&scores[a]));
    order.into_iter().map(|i| pool[i].word.to_owned()).collect()
}

#[test]
fn frequency_converts_to_probability_with_a_floor() {
    assert!((zipf_probability(Some(6000), 3.0) - 1e-3).abs() < 1e-15);
    assert!((zipf_probability(Some(1200), 3.0) - 1e-6).abs() < 1e-18);
    assert!((zipf_probability(None, 3.0) - 1e-6).abs() < 1e-18);
}

#[test]
fn lambda_grows_with_history_and_is_capped() {
    let params = ScoreParams::DEFAULT;
    assert_eq!(personal_lambda(0.0, &params), 0.0);
    assert!((personal_lambda(10.0, &params) - 10.0 / 60.0).abs() < 1e-12);
    assert_eq!(personal_lambda(5000.0, &params), 0.3);
}

#[test]
fn adding_unrelated_low_scoring_words_keeps_the_leaders_in_order() {
    let base = vec![
        word("the", 0.05, 1.0, 0),
        word("they", 0.004, 1.0, 0),
        word("then", 0.003, 1.0, 5),
        word("their", 0.003, 0.01, 0),
    ];
    let before = ranked(&base, 40.0);
    let mut grown = base.clone();
    let junk: Vec<&'static str> = (0..300)
        .map(|i| &*Box::leak(format!("junk{i}").into_boxed_str()))
        .collect();
    for text in junk {
        grown.push(word(text, 1e-6, 0.003, 0));
    }
    let after = ranked(&grown, 40.0);
    assert_eq!(after[..4], before[..]);
}

#[test]
fn a_word_chosen_once_gets_no_personal_share() {
    let pool = vec![
        word("hello", 1e-4, 1.0, 0),
        word("helloWorld", 1e-6, 1.0, 1),
    ];
    assert_eq!(ranked(&pool, 2000.0), ["hello", "helloWorld"]);
    // 选过 20 次、对手不强（模型份额不到约 0.43）：个人词第一
    let pool = vec![
        word("open", 1e-6, 0.003, 0),
        word("openfabrica", 1e-6, 1.0, 20),
    ];
    assert_eq!(ranked(&pool, 2000.0), ["openfabrica", "open"]);
    // 模型很有把握的对手：个人词封顶 0.3，排不到它前面
    let pool = vec![word("the", 1e-2, 1.0, 0), word("thx", 1e-6, 1.0, 20)];
    assert_eq!(ranked(&pool, 2000.0), ["the", "thx"]);
}

#[test]
fn domain_words_get_a_fixed_bonus() {
    let mut pool = vec![
        word("springboard", 1e-5, 1.0, 0),
        word("Spring Boot", 1e-6, 1.0, 0),
    ];
    assert_eq!(ranked(&pool, 0.0), ["springboard", "Spring Boot"]);
    pool[1].domain = true;
    assert_eq!(ranked(&pool, 0.0), ["Spring Boot", "springboard"]);
}
