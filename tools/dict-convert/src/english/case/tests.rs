//! 大小写统计的测试：原样写法与 `tokenize` 的词一一对上，句首、标题行不计。

use glimmer_core::english::{Token, tokenize};

use super::surface::{line_words, surface};

/// `tokenize` 给出的词序列。
fn tokenized(text: &str) -> Vec<String> {
    tokenize(text)
        .filter_map(|t| match t {
            Token::Word(w) => Some(w),
            _ => None,
        })
        .collect()
}

/// 一行里全部原样写法（不管计不计）转小写后的序列。
fn surfaces_lowercased(text: &str) -> Vec<String> {
    text.split_whitespace()
        .filter_map(surface)
        .map(|w| w.to_ascii_lowercase())
        .collect()
}

#[test]
fn surfaces_match_tokenize_word_for_word() {
    // 与 tokenize 自己的测试同一批难例：边缘标点、弯撇号、非词片段、缩写、开头的标点
    for text in [
        "Hello, (World)",
        "\"Quoted\" *bold* «x»",
        "'tis dogs'",
        "Don't I’m rock'n'roll",
        "x don''t y",
        "I came. You saw! He won? Yes… ok",
        "one, two; three: four — five",
        "\"Stop.\" Go",
        "wait ... what ?! ,",
        "I have 3 cats",
        "see https://x.com now",
        "call foo_bar() then",
        "ping @user #tag ok",
        "a café here",
        "a self-driving car",
        "42 is it",
        "at 5. Then",
        "Done. 3 cats",
        "Mr. Smith",
        "e.g. this",
        "the U.S. army",
        "use .NET now",
        "Google’s API, NASA's iPhone.",
    ] {
        assert_eq!(surfaces_lowercased(text), tokenized(text), "{text}");
        assert!(!line_words(text).mismatched, "{text}");
    }
}

#[test]
fn keeps_the_original_casing_and_folds_curly_apostrophes() {
    assert_eq!(surface("(Google’s"), Some("Google's".to_owned()));
    assert_eq!(surface("\"OK.\""), Some("OK".to_owned()));
    assert_eq!(surface("foo_bar"), None);
}

#[test]
fn skips_sentence_initial_words() {
    let words = line_words("The cat met Google. It ran to 3 Oxford streets!");
    // The 在行首、It 在句末之后、Oxford 在 Gap 之后，都不计
    assert_eq!(
        words.counted,
        ["cat", "met", "Google", "ran", "to", "streets"]
    );
    assert_eq!(words.initial, 3);
}

#[test]
fn skips_headings_and_all_caps_lines() {
    let title = line_words("How To Train Your Dog");
    assert!(title.counted.is_empty());
    assert_eq!(title.heading, 5);
    let caps = line_words("THE END AND MORE");
    assert!(caps.counted.is_empty());
    // 有一个小写词就不是标题
    let normal = line_words("We use Google and Bing");
    assert_eq!(normal.counted, ["use", "Google", "and", "Bing"]);
}
