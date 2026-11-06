//! `--english-next`：手看英文下一词预测。把一串词逐个当英文候选上屏、每个后面敲空格，打印预测与概率，
//! 再连按几次 Tab，看接受之后接着出什么。

use std::time::Instant;

use glimmer_core::{Candidate, CandidateKind, Engine};

/// 最多连按几次 Tab。
const TAB_PRESSES: usize = 5;

/// 打印 `phrase`（空格分开的几个词）之后的下一词预测，以及连按 Tab 接受的结果。
pub fn show(engine: &mut Engine, phrase: &str) {
    engine.set_english_mode(true);
    // 每个输入各自从「不知道上文」开始，不接上一个输入
    engine.break_english_context();
    for word in phrase.split_whitespace() {
        engine.set_input(&word.to_ascii_lowercase());
        engine.commit(&Candidate {
            text: word.to_owned(),
            kind: CandidateKind::English,
            syllables: Vec::new(),
            reading: None,
            translation: None,
        });
        engine.note_passthrough(' ');
    }
    // 第一次取是真算（扫后继 + 算分），第二次是缓存
    let started = Instant::now();
    let words = engine.english_next_words();
    let first = started.elapsed();
    let started = Instant::now();
    engine.english_next_words();
    let cached = started.elapsed();
    println!(
        "  上文 {:?} · 首次 {:.3} ms · 缓存 {:.4} ms",
        engine.english_history(),
        first.as_secs_f64() * 1000.0,
        cached.as_secs_f64() * 1000.0
    );
    if words.is_empty() {
        println!("  （不出预测）");
        return;
    }
    for (index, word) in words.iter().enumerate() {
        println!("  {}. {}  {:.3}", index + 1, word.text, word.probability);
    }
    let mut line = format!("{} ", phrase.trim());
    let mut accepted = Vec::new();
    for _ in 0..TAB_PRESSES {
        let Some(probability) = engine
            .english_next_words()
            .first()
            .map(|word| word.probability)
        else {
            break;
        };
        let Some(text) = engine.accept_english_prediction(0) else {
            break;
        };
        accepted.push(format!("{}({probability:.2})", text.trim_end()));
        line.push_str(&text);
    }
    println!(
        "  Tab ×{}: {} → {:?}",
        accepted.len(),
        accepted.join(" "),
        line
    );
}
