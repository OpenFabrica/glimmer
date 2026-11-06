//! 英文上文：上屏、直通、断开、超时、会话交换各自把上文变成什么，以及上文真的改了英文候选的排序。

use std::time::Duration;

use super::{engine, texts_of};
use crate::engine::english_context::{ContextState, Earlier};
use crate::english::NgramHistory;
use crate::english::test_model::TableModel;
use crate::sentence::Context;
use crate::{Candidate, CandidateKind, Engine, EngineSession};
use glimmer_dictionary::WordList;

const WORDS: &str = "hello\thello\t5000\nworld\tworld\t5300\nthen\tthen\t5800\nthere\tthere\t5900\n\
                     they\tthey\t6000\nand\tand\t7000\nover\tover\t5600\nI\ti\t7000\n";

/// 英文模式、装了小词表的引擎。
fn english_engine() -> Engine {
    let mut e = engine().with_english(WordList::parse(WORDS).unwrap());
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

/// 当前上文里的词：(再前一个, 前一个)。
fn words_of(e: &Engine) -> Option<(Earlier, String)> {
    match e.english_context().state() {
        ContextState::Words { earlier, previous } => Some((earlier.clone(), previous.clone())),
        _ => None,
    }
}

#[test]
fn committed_words_shift_and_keep_the_last_two() {
    let mut e = english_engine();
    assert_eq!(e.english_history(), NgramHistory::Unknown);
    commit_word(&mut e, "hello");
    // 前面不知道：只走二元
    assert_eq!(e.english_history(), NgramHistory::Previous("hello"));
    e.note_passthrough(' ');
    commit_word(&mut e, "World");
    e.note_passthrough(',');
    e.note_passthrough(' ');
    commit_word(&mut e, "and");
    // 逗号与空格不断开；连续三个词，上文是后两个（token 小写）
    assert_eq!(
        e.english_history(),
        NgramHistory::Context(Context::after_two("world", "and"))
    );
}

#[test]
fn sentence_end_and_newline_start_a_sentence() {
    let mut e = english_engine();
    commit_word(&mut e, "hello");
    e.note_passthrough('.');
    assert_eq!(*e.english_context().state(), ContextState::SentenceStart);
    assert_eq!(e.english_history(), NgramHistory::START);
    // 句首之后的第一个词：再前一个是句首标记
    commit_word(&mut e, "I");
    assert_eq!(words_of(&e), Some((Earlier::Start, "i".to_owned())));
    assert_eq!(
        e.english_history(),
        NgramHistory::Context(Context::after("i"))
    );
    e.note_passthrough('\n');
    assert_eq!(*e.english_context().state(), ContextState::SentenceStart);
}

#[test]
fn passing_through_function_keys_or_other_characters_loses_the_context() {
    let mut e = english_engine();
    commit_word(&mut e, "hello");
    // 壳直通了一个方向键
    e.break_english_context();
    assert_eq!(e.english_history(), NgramHistory::Unknown);
    commit_word(&mut e, "hello");
    e.note_passthrough('7');
    assert_eq!(e.english_history(), NgramHistory::Unknown);
    commit_word(&mut e, "hello");
    e.note_backspace();
    assert_eq!(e.english_history(), NgramHistory::Unknown);
    commit_word(&mut e, "hello");
    e.break_chain();
    assert_eq!(e.english_history(), NgramHistory::Unknown);
    // 切中英模式也断开；壳每键都调一次，没变时不断
    commit_word(&mut e, "hello");
    e.set_english_mode(true);
    assert_eq!(e.english_history(), NgramHistory::Previous("hello"));
    e.set_english_mode(false);
    assert_eq!(e.english_history(), NgramHistory::Unknown);
}

#[test]
fn a_pause_longer_than_the_timeout_loses_the_context() {
    let mut e = english_engine();
    commit_word(&mut e, "hello");
    e.note_passthrough(' ');
    e.english_context.backdate(Duration::from_secs(4));
    e.push('w');
    assert_eq!(e.english_history(), NgramHistory::Previous("hello"));
    e.clear();
    e.english_context.backdate(Duration::from_secs(6));
    e.push('w');
    assert_eq!(e.english_history(), NgramHistory::Unknown);
}

#[test]
fn unknown_words_and_non_word_commits_lose_the_context() {
    let mut e = english_engine();
    commit_word(&mut e, "hello");
    // 回车原样上屏一个词表里没有的词：训练时它是断开，上文不知道
    e.set_input("zzqx");
    assert_eq!(e.take_raw(), "zzqx");
    assert_eq!(e.english_history(), NgramHistory::Unknown);
    // 原样上屏词表里的词：记下
    e.set_input("hello");
    e.take_raw();
    assert_eq!(e.english_history(), NgramHistory::Previous("hello"));
    // emoji、中文都断开
    e.set_input("smile");
    e.commit(&Candidate {
        text: "😀".to_owned(),
        kind: CandidateKind::Emoji,
        syllables: Vec::new(),
        reading: Some("smile".to_owned()),
        translation: None,
    });
    assert_eq!(e.english_history(), NgramHistory::Unknown);
}

#[test]
fn clear_keeps_the_context_and_discard_input_drops_it() {
    let mut e = english_engine();
    commit_word(&mut e, "hello");
    // Esc 放弃一段带错的组句：上文还是刚才那个词（评测里先敲错串再清掉，靠的就是这一条）
    e.set_input("wrld");
    e.clear();
    assert_eq!(e.english_history(), NgramHistory::Previous("hello"));
    e.discard_input();
    assert_eq!(e.english_history(), NgramHistory::Unknown);
    let mut session = EngineSession::default();
    commit_word(&mut e, "hello");
    e.swap_session(&mut session);
    session.discard_input();
    e.swap_session(&mut session);
    assert_eq!(e.english_history(), NgramHistory::Unknown);
}

#[test]
fn sessions_keep_their_own_context() {
    let mut e = english_engine();
    commit_word(&mut e, "hello");
    let mut other = EngineSession::default();
    // 切到另一个应用：那边没有上文
    e.swap_session(&mut other);
    assert_eq!(e.english_history(), NgramHistory::Unknown);
    e.set_english_mode(true);
    commit_word(&mut e, "over");
    // 切回来：还是 hello，不串
    e.swap_session(&mut other);
    assert_eq!(e.english_history(), NgramHistory::Previous("hello"));
    e.swap_session(&mut other);
    assert_eq!(e.english_history(), NgramHistory::Previous("over"));
}

#[test]
fn the_context_changes_english_candidates_and_the_engine_works_without_a_model() {
    // 没有英文模型：照常出候选（按词频）
    let mut plain = english_engine();
    plain.set_input("th");
    assert_eq!(texts_of(&plain)[0], "they");
    let model = TableModel::new(&[
        ("then", 0.003),
        ("there", 0.004),
        ("they", 0.005),
        ("and", 0.02),
        ("over", 0.002),
    ])
    .bigram("and", "then", 0.08, 400)
    .bigram("over", "there", 0.2, 300);
    let mut e = english_engine().with_english_lm(Box::new(model));
    assert!(e.has_english_lm());
    e.set_input("th");
    assert_eq!(texts_of(&e)[0], "they");
    e.clear();
    commit_word(&mut e, "and");
    e.note_passthrough(' ');
    e.set_input("th");
    assert_eq!(texts_of(&e)[0], "then");
    e.clear();
    e.note_passthrough('.');
    e.note_passthrough(' ');
    commit_word(&mut e, "over");
    e.note_passthrough(' ');
    for c in "th".chars() {
        e.push(c);
    }
    assert_eq!(texts_of(&e)[0], "there");
}
