//! 英文及领域别名词表的行为测试。

use super::*;

#[test]
fn looks_up_by_lowercase_code() {
    let list = WordList::parse("GitHub\tgithub\nhello\thello\niPhone\n").unwrap();
    assert_eq!(list.get("github"), Some("GitHub"));
    assert_eq!(list.get("iphone"), Some("iPhone"));
    assert_eq!(list.get("hello"), Some("hello"));
    assert_eq!(list.get("nope"), None);
}

#[test]
fn completes_prefixes_by_frequency() {
    let list = WordList::parse(
        "compass\tcompass\t300\ncompany\tcompany\t900\ncompare\tcompare\t500\ncom\tcom\t100\ncomma\tcomma\n",
    )
    .unwrap();
    assert_eq!(list.complete("comp", 2), ["company", "compare"]);
    assert_eq!(
        list.complete("com", 10),
        ["company", "compare", "compass", "comma"]
    );
    assert!(list.complete("zzz", 3).is_empty());
    assert!(list.complete("", 3).is_empty());
}

#[test]
fn explicit_aliases_preserve_symbols_and_mixed_names() {
    let dict = crate::Dictionary::parse("C++\t@cpp\t40\nC#\t@csharp\t40\nGit分支\t@gitfenzhi\t20\nAGENTS.md\t@agentsmd\t40\nC++\t@cplusplus\t40\n旧条目\tlegacy\t20\n").unwrap();
    let words = WordList::from_dictionaries(&[dict]);
    assert_eq!(words.get("cpp"), Some("C++"));
    assert_eq!(words.get("csharp"), Some("C#"));
    assert_eq!(words.get("cplusplus"), Some("C++"));
    assert_eq!(words.get("gitfenzhi"), Some("Git分支"));
    assert_eq!(words.complete("agents", 3), ["AGENTS.md"]);
    assert_eq!(words.get("legacy"), None);
}

#[test]
fn imported_invalid_aliases_are_ignored_and_first_dictionary_wins() {
    let first = crate::Dictionary::parse(
        "C++\t@cpp\t40\nbad\t@../x\t20\nempty\t@\t20\nupper\t@UPPER\t20\n",
    )
    .unwrap();
    let second = crate::Dictionary::parse("different\t@cpp\t80\n").unwrap();
    let words = WordList::from_dictionaries(&[first, second]);
    assert_eq!(words.len(), 1);
    assert_eq!(words.get("cpp"), Some("C++"));
}

#[test]
fn one_code_keeps_every_word_and_get_returns_the_first() {
    let list = WordList::parse(
        "well\twell\t5000\nwe'll\twell\t3000\nwell\twell\t1\nWell\twell\t2\nwelt\twelt\t10\n",
    )
    .unwrap();
    let words: Vec<&str> = list.get_all("well").iter().map(WordEntry::word).collect();
    // 文件先后；同码同词的第二个 `well` 去掉，大小写不同的 `Well` 是另一个词
    assert_eq!(words, ["well", "we'll", "Well"]);
    assert_eq!(list.get("well"), Some("well"));
    assert_eq!(list.frequency("well"), Some(5000));
    assert!(list.get_all("wel").is_empty());
    assert_eq!(list.len(), 4);
    // 补全按词去重，不因同码多词重复
    assert_eq!(list.complete("wel", 9), ["well", "we'll", "welt", "Well"]);
}

#[test]
fn apostrophe_words_load_with_or_without_a_code_column() {
    let list = WordList::parse("don't\tdont\t4000\nI'm\nit’s\n").unwrap();
    assert_eq!(list.get("dont"), Some("don't"));
    assert_eq!(list.get("im"), Some("I'm"));
    assert_eq!(list.get("its"), Some("it’s"));
    assert_eq!(english_code("Don't"), "dont");
    assert_eq!(english_code("it’s"), "its");
}

#[test]
fn prefix_range_edges() {
    let list = WordList::parse("ab\nabc\nabd\nb\nba\nzz\n").unwrap();
    // 空前缀是整张表
    assert_eq!(list.prefix_range("").len(), 6);
    // 含正好等于前缀的
    assert_eq!(codes(list.prefix_range("ab")), ["ab", "abc", "abd"]);
    assert_eq!(codes(list.prefix_range("abc")), ["abc"]);
    // 无命中：比所有都小、夹在中间、比所有都大
    assert!(list.prefix_range("a0").is_empty());
    assert!(list.prefix_range("aa").is_empty());
    assert!(list.prefix_range("c").is_empty());
    assert!(list.prefix_range("zzz").is_empty());
    // 最后一个前缀
    assert_eq!(codes(list.prefix_range("z")), ["zz"]);
    assert_eq!(list.prefix_span("b"), 3..5);
    assert!(WordList::default().prefix_range("a").is_empty());
}

#[test]
fn prefix_mass_matches_brute_force() {
    let list = sample();
    for prefix in ["", "a", "co", "com", "comp", "zz", "q", "hello", "x"] {
        let brute: u64 = list
            .entries()
            .filter(|(code, _, _)| code.starts_with(prefix))
            .map(|(_, _, f)| u64::from(f))
            .sum();
        assert_eq!(list.prefix_mass(prefix), brute, "{prefix}");
    }
    assert_eq!(WordList::default().prefix_mass(""), 0);
}

#[test]
fn matched_prefix_len_is_the_longest_known_beginning() {
    let list = WordList::parse("hello\nhelp\nworld\n").unwrap();
    assert_eq!(list.matched_prefix_len("hel"), 3);
    assert_eq!(list.matched_prefix_len("helxo"), 3);
    assert_eq!(list.matched_prefix_len("hello"), 5);
    assert_eq!(list.matched_prefix_len("hellos"), 5);
    assert_eq!(list.matched_prefix_len("worf"), 3);
    assert_eq!(list.matched_prefix_len("zzz"), 0);
    assert_eq!(list.matched_prefix_len(""), 0);
    assert_eq!(WordList::default().matched_prefix_len("a"), 0);
}

#[test]
fn short_prefix_top_matches_brute_force() {
    let list = sample();
    let brute = |prefix: &str, limit: usize| {
        let mut hits: Vec<(usize, &WordEntry)> = list
            .as_slice()
            .iter()
            .enumerate()
            .filter(|(_, e)| e.code().starts_with(prefix))
            .collect();
        hits.sort_by_key(|(i, e)| (std::cmp::Reverse(e.frequency()), *i));
        hits.into_iter()
            .take(limit)
            .map(|(_, e)| e.word())
            .collect::<Vec<_>>()
    };
    // 「a」「co」走预计算，「com」「comp」现挑；limit 超过 512 时预计算不够，也要现挑
    for prefix in ["a", "c", "co", "cz", "com", "comp", "", "zz"] {
        for limit in [0, 1, 7, SHORT_PREFIX_TOP, SHORT_PREFIX_TOP + 300, 5000] {
            assert_eq!(
                words(list.prefix_top(prefix, limit)),
                brute(prefix, limit),
                "{prefix} {limit}"
            );
        }
    }
}

/// 切片里各条的编码。
fn codes(entries: &[WordEntry]) -> Vec<&str> {
    entries.iter().map(WordEntry::code).collect()
}

/// 各条的原样词。
fn words(entries: Vec<&WordEntry>) -> Vec<&str> {
    entries.into_iter().map(WordEntry::word).collect()
}

/// 造一张够大的表：`a` 与 `co` 下都超过 512 条，词频有重复，考验截断与同分排序。
fn sample() -> WordList {
    let mut tsv = String::new();
    for i in 0..1500u32 {
        let first = ["a", "co", "com", "comp", "b"][i as usize % 5];
        tsv.push_str(&format!(
            "{first}{i:x}w\t{first}{}\t{}\n",
            word_code(i),
            i * 37 % 211
        ));
    }
    tsv.push_str("hello\thello\t900\n");
    WordList::parse(&tsv).unwrap()
}

/// 把数字映射成小写字母串，给 `sample` 造编码。
fn word_code(mut i: u32) -> String {
    let mut s = String::new();
    loop {
        s.push(char::from(b'a' + (i % 26) as u8));
        i /= 26;
        if i == 0 {
            return s;
        }
    }
}

/// 构造耗时（release 下跑：`cargo test --release -p glimmer-dictionary construction_time -- --ignored --nocapture`），
/// 数据不在就跳过。
#[test]
#[ignore]
fn construction_time() {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../data/generated/english.tsv"
    );
    if !std::path::Path::new(path).exists() {
        return;
    }
    let source = std::fs::read_to_string(path).unwrap();
    for _ in 0..5 {
        let start = std::time::Instant::now();
        let list = WordList::from_path(path).unwrap();
        let from_path = start.elapsed();
        let start = std::time::Instant::now();
        let parsed = WordList::parse(&source).unwrap();
        eprintln!(
            "from_path {from_path:?}，parse {:?}（{} 条）",
            start.elapsed(),
            list.len().max(parsed.len())
        );
    }
}

#[test]
fn next_bytes_lists_what_can_follow_a_prefix() {
    let list = WordList::parse("he\nhello\nhelp\nhex\nhi\nzoo\n").unwrap();
    assert_eq!(list.next_bytes("he"), b"lx");
    assert_eq!(list.next_bytes("h"), b"ei");
    assert_eq!(list.next_bytes(""), b"hz");
    assert_eq!(list.next_bytes("hel"), b"lp");
    assert!(list.next_bytes("hello").is_empty());
    assert!(list.next_bytes("q").is_empty());
    let big = sample();
    for prefix in ["", "a", "co", "com", "b"] {
        let mut brute: Vec<u8> = big
            .entries()
            .filter(|(code, _, _)| code.len() > prefix.len() && code.starts_with(prefix))
            .map(|(code, _, _)| code.as_bytes()[prefix.len()])
            .collect();
        brute.dedup();
        assert_eq!(big.next_bytes(prefix), brute, "{prefix}");
    }
}
