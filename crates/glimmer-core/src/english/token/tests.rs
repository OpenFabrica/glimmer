//! 切词规则的测试：每条规则至少一个例子，写法照模块文档。

use super::*;

/// 把事件写成短串方便比：词原样，`.` 是句末，`|` 是 Gap。
fn events(text: &str) -> Vec<String> {
    tokenize(text)
        .map(|t| match t {
            Token::Word(w) => w,
            Token::SentenceEnd => ".".to_owned(),
            Token::Gap => "|".to_owned(),
        })
        .collect()
}

#[test]
fn strips_edge_punctuation_and_lowercases() {
    assert_eq!(events("Hello, (World)"), ["hello", "world", "."]);
    assert_eq!(
        events("\"Quoted\" *bold* «x»"),
        ["quoted", "bold", "x", "."]
    );
    assert_eq!(events("'tis dogs'"), ["tis", "dogs", "."]);
}

#[test]
fn keeps_inner_apostrophes_and_folds_curly_ones() {
    assert_eq!(
        events("Don't I’m rock'n'roll"),
        ["don't", "i'm", "rock'n'roll", "."]
    );
    // 连着两个撇号不是词
    assert_eq!(events("x don''t y"), ["x", "|", "y", "."]);
    // 没有词的流什么也不出
    assert!(events("don''t").is_empty());
}

#[test]
fn sentence_ends_break_and_mid_sentence_punctuation_does_not() {
    assert_eq!(
        events("I came. You saw! He won? Yes… ok"),
        [
            "i", "came", ".", "you", "saw", ".", "he", "won", ".", "yes", ".", "ok", "."
        ]
    );
    assert_eq!(
        events("one, two; three: four — five"),
        ["one", "two", "three", "four", "five", "."]
    );
    // 引号在句号外面也认
    assert_eq!(events("\"Stop.\" Go"), ["stop", ".", "go", "."]);
    // 整段都是标点
    assert_eq!(events("wait ... what ?! ,"), ["wait", ".", "what", "."]);
}

#[test]
fn newlines_and_text_end_break() {
    assert_eq!(events("hello\nworld"), ["hello", ".", "world", "."]);
    assert_eq!(events("hello\r\n\r\nworld\n"), ["hello", ".", "world", "."]);
    assert!(events("").is_empty());
    assert!(events("  \n ... \n").is_empty());
}

#[test]
fn non_words_break_instead_of_joining_neighbors() {
    assert_eq!(events("I have 3 cats"), ["i", "have", "|", "cats", "."]);
    assert_eq!(events("see https://x.com now"), ["see", "|", "now", "."]);
    assert_eq!(events("call foo_bar() then"), ["call", "|", "then", "."]);
    assert_eq!(events("ping @user #tag ok"), ["ping", "|", "ok", "."]);
    assert_eq!(events("a café here"), ["a", "|", "here", "."]);
    assert_eq!(events("a self-driving car"), ["a", "|", "car", "."]);
    // 开头就是非词：上文未知，要给出 Gap
    assert_eq!(events("42 is it"), ["|", "is", "it", "."]);
}

#[test]
fn consecutive_breaks_collapse_to_the_last_one() {
    // 数字后跟句号：句首
    assert_eq!(events("at 5. Then"), ["at", ".", "then", "."]);
    // 句号后跟数字：上文未知
    assert_eq!(events("Done. 3 cats"), ["done", "|", "cats", "."]);
    assert_eq!(events("x 1 2 3 y"), ["x", "|", "y", "."]);
    // 开头的句末省掉
    assert_eq!(events("\n\n. Hi"), ["hi", "."]);
}

#[test]
fn abbreviations_behave_deterministically() {
    assert_eq!(events("Mr. Smith"), ["mr", ".", "smith", "."]);
    // 开头的 Gap 与句末合并成句末，又因为在开头而省掉
    assert_eq!(events("e.g. this"), ["this", "."]);
    assert_eq!(events("so e.g. this"), ["so", ".", "this", "."]);
    assert_eq!(events("the U.S. army"), ["the", ".", "army", "."]);
    assert_eq!(events("use .NET now"), ["use", "net", "now", "."]);
}

#[test]
fn english_code_is_lowercase_without_apostrophes() {
    assert_eq!(english_code("don't"), "dont");
    assert_eq!(english_code("I’m"), "im");
    assert_eq!(english_code("We'll"), "well");
    assert_eq!(english_code("hello"), "hello");
}

#[test]
fn runtime_helpers_follow_the_same_rules() {
    assert_eq!(word_token("Hello"), Some("hello".to_owned()));
    assert_eq!(word_token("don’t"), Some("don't".to_owned()));
    assert_eq!(word_token("\"well,"), Some("well".to_owned()));
    assert_eq!(word_token("Spring Boot"), None);
    assert_eq!(word_token("C++"), None);
    assert_eq!(word_token("foo1"), None);
    assert_eq!(word_token("..."), None);
    assert_eq!(char_break(' '), None);
    assert_eq!(char_break(','), None);
    assert_eq!(char_break('"'), None);
    assert_eq!(char_break('.'), Some(Token::SentenceEnd));
    assert_eq!(char_break('?'), Some(Token::SentenceEnd));
    assert_eq!(char_break('\n'), Some(Token::SentenceEnd));
    assert_eq!(char_break('。'), Some(Token::SentenceEnd));
    assert_eq!(char_break('-'), Some(Token::Gap));
    assert_eq!(char_break('7'), Some(Token::Gap));
    assert_eq!(char_break('a'), Some(Token::Gap));
}
