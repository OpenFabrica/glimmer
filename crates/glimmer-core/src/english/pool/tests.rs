//! 检索名额：便宜的变体不被贵的挤掉、正确变体生成得再晚也进池、各路与总数的上限。

use glimmer_dictionary::WordList;

use super::*;
use crate::english::{DeadEnds, ScoreParams, Sources};

/// 只用随包词表收池。
fn pool_of<'a>(list: &'a WordList, code: &'a str) -> Vec<PoolWord<'a>> {
    gather(
        &Sources::lists(Some(list), None, None),
        code,
        code,
        &ScoreParams::DEFAULT,
    )
}

#[test]
fn cheap_variants_are_not_crowded_out_by_expensive_ones() {
    // 敲 thws：w 是 e 的邻键，thes → thesis 是 Near 档；w → z 不相邻，thzs → 200 个高频词是 Edit 档，比名额还多
    let mut tsv = String::from("thesis\tthesis\t3100\n");
    for i in 0..200 {
        tsv.push_str(&format!("thzs{i:03}\tthzs{i:03}\t6000\n"));
    }
    let list = WordList::parse(&tsv).unwrap();
    let pool = pool_of(&list, "thws");
    let thesis = pool.iter().find(|w| w.word == "thesis").expect("thesis");
    // thes 之后还要补字母才是 thesis：便宜的一处错误再乘补全折扣
    let expected = ScoreParams::DEFAULT.near * ScoreParams::DEFAULT.typo_completion;
    assert!((thesis.cost - expected).abs() < 1e-12);
    // 错字那一路正好收满名额，贵的那档被截掉一部分；thws 没有补全，敲的串本身作为原样读法多一个
    assert_eq!(pool.len(), TYPO_BUDGET + 1);
    assert!(pool.iter().any(|w| w.literal && w.word == "thws"));
}

#[test]
fn the_right_variant_enters_the_pool_even_when_generated_last() {
    // 正确变体 abcz 只差在最后一位（生成顺序最后）；诱饵改的是第一位、词频高得多，每个诱饵前缀下还有好几个词
    let mut tsv = String::new();
    for c in (b'b'..=b'z').filter(|&c| !b"qwsz".contains(&c)) {
        for k in 0..8 {
            tsv.push_str(&format!("{0}bcs{1}\t{0}bcs{1}\t6000\n", c as char, k));
        }
    }
    tsv.push_str("abcz\tabcz\t1000\n");
    let list = WordList::parse(&tsv).unwrap();
    let pool = pool_of(&list, "abcs");
    assert!(pool.len() <= TYPO_BUDGET + 1);
    assert!(pool.iter().any(|w| w.word == "abcz"));
}

#[test]
fn prefix_budget_and_pool_cap_hold() {
    let tsv: String = (0..3000)
        .map(|i| format!("a{i:04}\ta{i:04}\t{}\n", 3000 + i))
        .collect();
    let list = WordList::parse(&tsv).unwrap();
    let pool = pool_of(&list, "a");
    assert_eq!(pool.len(), PREFIX_BUDGET);
    // 按词频取的前 512：最低的那个是第 2488 号
    assert!(pool.iter().all(|w| w.frequency.unwrap() >= 3000 + 2488));
    const { assert!(MAX_POOL >= PREFIX_BUDGET + SUCCESSOR_BUDGET + PERSONAL_BUDGET + TYPO_BUDGET) };
}

#[test]
fn personal_counts_are_filled_case_insensitively() {
    let main = WordList::parse("hello\thello\t5000\n").unwrap();
    let personal = WordList::parse("Hello\thello\t7\n").unwrap();
    let mut pool = gather(
        &Sources::lists(Some(&main), Some(&personal), None),
        "hel",
        "hel",
        &ScoreParams::DEFAULT,
    );
    fill_personal(&mut pool, Some(&personal));
    let hello = pool.iter().find(|w| w.word == "hello").unwrap();
    assert_eq!(hello.personal, 7);
    assert_eq!(hello.frequency, Some(5000));
}

#[test]
fn the_literal_reading_joins_only_when_there_is_no_completion_but_some_correction() {
    let list = WordList::parse("critical\tcritical\t4700\nxqz\txqzw\t3000\n").unwrap();
    // kritica：没有以它开头的词，有纠错（k → c）：原样读法进池
    let pool = pool_of(&list, "kritica");
    assert!(pool.iter().any(|w| w.literal && w.word == "kritica"));
    assert!(pool.iter().any(|w| w.word == "critical"));
    // criti：有补全，不加
    assert!(pool_of(&list, "criti").iter().all(|w| !w.literal));
    // qqq：什么候选都没有，也不凭空加一格
    assert!(pool_of(&list, "qqqq").is_empty());
}

#[test]
fn edits_found_exhausted_are_skipped_on_later_keys_with_the_same_pool() {
    let list = WordList::parse("receive\treceive\t4800\nbelieve\tbelieve\t4600\n").unwrap();
    // 误拼表里有个长误拼：敲它的开头时池子里有它，纠错变体却一个都没有
    let table = glimmer_dictionary::Misspellings::parse("qwxzqwxzqwxz\treceive\n").unwrap();
    let dead_ends = DeadEnds::default();
    let with = Sources {
        misspellings: Some(&table),
        dead_ends: Some(&dead_ends),
        ..Sources::lists(Some(&list), None, None)
    };
    let without = Sources {
        dead_ends: None,
        ..with
    };
    let words = |sources: &Sources<'_>, code: &'static str| -> Vec<(String, bool)> {
        gather(sources, code, code, &ScoreParams::DEFAULT)
            .into_iter()
            .map(|w| (w.word.to_owned(), w.literal))
            .collect()
    };
    assert!(!dead_ends.edits_exhausted("qwxzqwxz"));
    let first = words(&with, "qwxzqwxz");
    assert!(first.iter().any(|(w, _)| w == "receive"));
    // 试过两处错误也没有：记下，往后的键不再搜，池子与不记时一样
    assert!(dead_ends.edits_exhausted("qwxzqwxzq"));
    assert_eq!(words(&with, "qwxzqwxzq"), words(&without, "qwxzqwxzq"));
    // 退回更短的串不受影响
    assert!(!dead_ends.edits_exhausted("qwxzqwx"));
    dead_ends.clear();
    assert!(!dead_ends.edits_exhausted("qwxzqwxzq"));
}

#[test]
fn corrections_that_still_need_letters_pay_the_completion_discount() {
    let list = WordList::parse("receive\treceive\t4800\nreceived\treceived\t5200\n").unwrap();
    let params = ScoreParams {
        typo_completion: 0.1,
        ..ScoreParams::DEFAULT
    };
    // recieve 对调成 receive 正好是整词：代价就是便宜的一处错误；received 还要再补一个 d，再乘折扣
    let pool = gather(
        &Sources::lists(Some(&list), None, None),
        "recieve",
        "recieve",
        &params,
    );
    let cost = |word: &str| pool.iter().find(|w| w.word == word).unwrap().cost;
    assert!((cost("receive") - params.near).abs() < 1e-12);
    assert!((cost("received") - params.near * 0.1).abs() < 1e-12);
}

#[test]
fn long_codes_without_one_edit_variants_are_not_marked_as_edits_exhausted() {
    // 13 个字母：一处错误没结果、不试两处，不知道两处有没有，不能记成「试过两处也没有」
    let list = WordList::parse("abcdefghijklmnop\tabcdefghijklmnop\t3000\n").unwrap();
    let table =
        glimmer_dictionary::Misspellings::parse("abxdeyghijklmz\tabcdefghijklmnop\n").unwrap();
    let dead_ends = DeadEnds::default();
    let sources = Sources {
        misspellings: Some(&table),
        dead_ends: Some(&dead_ends),
        ..Sources::lists(Some(&list), None, None)
    };
    let pool = gather(
        &sources,
        "abxdeyghijklm",
        "abxdeyghijklm",
        &ScoreParams::DEFAULT,
    );
    // 只有误拼表命中（池子不空），但纠错那一路没有记录
    assert!(pool.iter().any(|w| w.word == "abcdefghijklmnop"));
    assert!(!dead_ends.edits_exhausted("abxdeyghijklm"));
}
