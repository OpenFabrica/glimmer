//! 英文模式候选的基本行为：补全、纠错、大小写、几张词表合起来。打分、检索、精确词规则另有各自的测试。

use super::*;
use glimmer_dictionary::WordList;

/// 测试用词表，词频是 Zipf × 1000（与随包词表同一单位）。
fn words() -> WordList {
    WordList::parse(
        "company\tcompany\t5900\ncompare\tcompare\t5500\ncompass\tcompass\t4300\ncomma\tcomma\t4100\n\
         iPhone\tiphone\t4800\nhello\thello\t5000\nhollow\thollow\t3200\nhelp\thelp\t5700\n",
    )
    .unwrap()
}

/// 只有随包词表的候选。
fn plain(list: &WordList, typed: &str, limit: usize) -> Vec<String> {
    suggest(
        &Sources::lists(Some(list), None, None),
        typed,
        &ScoreParams::DEFAULT,
        limit,
    )
}

#[test]
fn completions_rank_by_frequency_and_typos_are_corrected() {
    let list = words();
    // comma 不以 comp 开头，是 comp 多敲了 p（com 的补全）这一处错误的纠正，代价高排最后
    assert_eq!(
        plain(&list, "comp", 9),
        ["company", "compare", "compass", "comma"]
    );
    // 精确词第一，词表写法优先
    assert_eq!(plain(&list, "iphone", 9), ["iPhone"]);
    // 拼错一个字母：helo → help / hello（邻键替换与漏敲双写同档，按词频），hollow 差两处不算；
    // helo 没有补全，敲的串本身作为原样读法排在纠错后面
    assert_eq!(plain(&list, "helo", 9), ["help", "hello", "helo"]);
    // 三个字母以内不纠正；带数字的是标识符，也不纠正
    assert!(plain(&list, "hlo", 9).is_empty());
    assert!(plain(&list, "hel1", 9).is_empty());
    assert!(plain(&list, "", 9).is_empty());
}

#[test]
fn case_follows_the_typed_letters() {
    let list = words();
    assert_eq!(plain(&list, "Comp", 1), ["Company"]);
    assert_eq!(plain(&list, "COMP", 1), ["COMPANY"]);
    assert_eq!(plain(&list, "Iphone", 1), ["IPhone"]);
    assert_eq!(adapt_case("hello", "H"), "Hello");
}

#[test]
fn lists_pool_their_words_and_exact_words_come_first() {
    let user = WordList::parse("gist\tgist\t3\n").unwrap();
    let main = WordList::parse("giant\tgiant\t4900\ngift\tgift\t4800\n").unwrap();
    let sources = Sources::lists(Some(&main), Some(&user), None);
    // 补全从几张表合起来排：gist 只在个人表里，次数 3（λ 很小），排在常用词后面
    assert_eq!(
        suggest(&sources, "gi", &ScoreParams::DEFAULT, 9),
        ["giant", "gift", "gist"]
    );
    // 个人表里的精确词第一，后面跟着差一处编辑的 gift
    assert_eq!(
        suggest(&sources, "gist", &ScoreParams::DEFAULT, 9),
        ["gist", "gift"]
    );
}

#[test]
fn enabled_domain_names_beat_generic_words_but_not_exact_words() {
    let main = WordList::parse("springboard\tspringboard\t4000\nspring\tspring\t5500\n").unwrap();
    let domain = WordList::parse("Spring Boot\tspringboot\t40\n").unwrap();
    let sources = Sources::lists(Some(&main), None, Some(&domain));
    assert_eq!(
        suggest(&sources, "springb", &ScoreParams::DEFAULT, 2),
        ["Spring Boot", "springboard"]
    );
    assert_eq!(
        suggest(&sources, "spring", &ScoreParams::DEFAULT, 1),
        ["spring"]
    );
}
