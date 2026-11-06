use std::path::{Path, PathBuf};
use std::time::Duration;

use glimmer_core::Engine;
use glimmer_dictionary::{Dictionary, WordList};

use super::personal::Rule;
use super::space::Change;
use super::*;

mod prediction;

/// 词的字母序列，注错测试用。
fn letters(word: &str) -> Vec<char> {
    word.chars().collect()
}

#[test]
fn split_keeps_words_and_skips_the_rest() {
    let pieces = token::split("\"Hello, world!\" don\u{2019}t 3rd well-known\n\nok");
    let word = |head: &str, text: &str, tail: &str| Piece::Word {
        head: head.to_owned(),
        text: text.to_owned(),
        tail: tail.to_owned(),
    };
    assert_eq!(
        pieces,
        [
            word("\"", "Hello", ", "),
            word("", "world", "!\" "),
            // 弯撇号当直撇号，撇号词留下
            word("", "don't", " "),
            Piece::Gap("3rd ".to_owned()),
            // 行尾的分隔是换行；空行什么都不产生
            Piece::Gap("well-known\n".to_owned()),
            word("", "ok", "\n"),
        ]
    );
    // 纯标点、非 ASCII 字母都跳过
    assert_eq!(
        token::split("-- café"),
        [
            Piece::Gap("-- ".to_owned()),
            Piece::Gap("café\n".to_owned())
        ]
    );
}

#[test]
fn four_typo_kinds() {
    let hello = letters("hello");
    // 邻键：h 的邻键按字母序是 b g j n u y，choice 0 取 b
    assert_eq!(TypoKind::Neighbor.apply(&hello, 0, 0), "bello");
    assert_eq!(TypoKind::Missing.apply(&hello, 1, 0), "hllo");
    // 多一个字母插在它前面，大小写跟着原字母
    assert_eq!(TypoKind::Extra.apply(&letters("Hi"), 0, 0), "BHi");
    assert_eq!(TypoKind::Transpose.apply(&hello, 1, 0), "hlelo");
    // 漏敲双写的前一个字母、对调两个相同字母都不算（敲出来的串错处对不上）
    assert!(!TypoKind::Missing.applies(&hello, 2));
    assert!(TypoKind::Missing.applies(&hello, 3));
    assert!(!TypoKind::Transpose.applies(&hello, 2));
    assert!(!TypoKind::Transpose.applies(&hello, 4));
    // 撇号不注错
    assert!(!TypoKind::Neighbor.applies(&letters("don't"), 3));
}

#[test]
fn picked_typos_are_stable_and_located() {
    for (index, word) in ["tomorrow", "receive", "a", "is", "hello", "don't"]
        .iter()
        .enumerate()
    {
        let typo = typo::pick(word, 7, index, false).unwrap();
        assert_ne!(typo.typed, *word);
        assert_eq!(typo, typo::pick(word, 7, index, false).unwrap());
        assert_eq!(typo.common, typo::common_prefix(&typo.typed, word));
        let length = word.chars().count();
        let typed = typo.typed.chars().count();
        assert!(typed.abs_diff(length) <= 1);
    }
    // 错前缀组：错处总在前三个字母里
    for seed in 0..200 {
        let typo = typo::pick("separate", seed, 3, true).unwrap();
        assert!(typo.common < typo::FRONT_LETTERS, "{typo:?}");
    }
    // 换种子换一批错
    let picks: std::collections::HashSet<String> = (0..20)
        .map(|seed| typo::pick("separate", seed, 0, false).unwrap().typed)
        .collect();
    assert!(picks.len() > 5);
}

#[test]
fn keystroke_rules() {
    // 8 个字母的词敲 2 个字母后选中 = 3 键
    let mut ranks = vec![None, Some(0)];
    ranks.resize(8, Some(0));
    assert_eq!(Keys::completion(&ranks, 1, false), Keys::selected(2));
    assert_eq!(Keys::selected(2).actual, 3);
    // 只在前三第 2 位：首选组不选，前三组选
    let ranks = [None, Some(1), Some(1), Some(0), Some(0), Some(0)];
    assert_eq!(Keys::completion(&ranks, 3, false).article, 3);
    assert_eq!(Keys::completion(&ranks, 1, false).article, 5);
    // 敲到倒数第二个字母才出现：选它不省键，敲完；首选是它，实际也不多键
    let ranks = [None, None, None, Some(0), Some(0)];
    assert_eq!(
        Keys::completion(&ranks, 1, false),
        Keys {
            article: 5,
            actual: 5
        }
    );
    // 一直不出现、敲完首选又不是它：实际按键多一下回车
    assert_eq!(
        Keys::completion(&[None, None, None], 3, true),
        Keys {
            article: 3,
            actual: 4
        }
    );
    // 敲错没救回来：敲了 5 键、错在第 3 个字母，退 3 格、重敲 3 个字母
    assert_eq!(
        Keys::recovered(5, 2, 5, true),
        Keys {
            article: 11,
            actual: 12
        }
    );
}

#[test]
fn savings_and_timing() {
    let mut savings = Savings::default();
    savings.add(8, Keys::selected(2));
    savings.add(2, Keys::typed_out(2, true));
    assert_eq!(savings.article_rate(), Some(1.0 - 5.0 / 10.0));
    assert_eq!(savings.actual_rate(), Some(1.0 - 6.0 / 10.0));
    assert_eq!(Savings::default().actual_rate(), None);

    let mut timing = Timing::default();
    assert_eq!(timing.percentile(50.0), None);
    for ms in 1..=100 {
        timing.push(Duration::from_millis(ms));
    }
    assert_eq!(timing.percentile(50.0), Some(Duration::from_millis(50)));
    assert_eq!(timing.percentile(95.0), Some(Duration::from_millis(95)));
    assert_eq!(timing.max(), Some(Duration::from_millis(100)));
}

#[test]
fn case_tables_parse() {
    let path = Path::new("cases.tsv");
    let cases = PrefixCase::parse("# 注释\n\ntomm\ttomorrow\n", path).unwrap();
    assert_eq!(cases.len(), 1);
    assert_eq!(cases[0].expected, "tomorrow");
    assert!(PrefixCase::parse("tomm\n", path).is_err());

    let personal = PersonalCase::parse(
        "误选\t0\thelloWorld\thellow\t1\thel,hell\tbelow:hello\n",
        path,
    )
    .unwrap();
    assert_eq!(personal[0].prefixes, ["hel", "hell"]);
    assert_eq!(personal[0].rule, Rule::Below("hello".to_owned()));
    assert!(personal[0].touches("Help"));
    assert!(!personal[0].touches("world"));
    assert!(PersonalCase::parse("误选\t0\tx\tx\t1\tx\tnope\n", path).is_err());

    assert!(Rule::First.passes(Some(0), None));
    assert!(!Rule::First.passes(Some(1), None));
    let below = Rule::Below("hello".to_owned());
    assert!(below.passes(Some(2), Some(1)));
    assert!(below.passes(None, Some(1)));
    assert!(!below.passes(Some(0), Some(1)));
    assert!(!below.passes(Some(0), None));
}

/// 只装英文词表的小引擎。
fn engine(words: &str) -> (Engine, WordList) {
    let engine =
        Engine::new(Dictionary::parse("").unwrap()).with_english(WordList::parse(words).unwrap());
    (engine, WordList::parse(words).unwrap())
}

/// 写一个临时文件，返回路径。
fn temp_file(name: &str, text: &str) -> PathBuf {
    let path = std::env::temp_dir().join(format!(
        "glimmer-eval-english-{}-{name}",
        std::process::id()
    ));
    std::fs::write(&path, text).unwrap();
    path
}

/// 只跑给定几组的参数。
fn options(paths: Vec<PathBuf>, groups: Vec<Group>) -> Options {
    Options {
        paths,
        words: None,
        seed: 1,
        groups,
        prefix_cases: PathBuf::new(),
        personal_cases: PathBuf::new(),
        show_misses: 5,
    }
}

/// 词频是 Zipf × 1000（与随包词表同一单位，引擎按它换算概率）。
const WORDS: &str = "hello\thello\t5600\nhelp\thelp\t5400\nheld\theld\t5000\nworld\tworld\t5300\nword\tword\t5200\n";

#[test]
fn simulates_typing_on_a_tiny_engine() {
    let (mut engine, list) = engine(WORDS);
    let text = temp_file("text.txt", "hello world, helo 42\n");
    let report = run(
        &mut engine,
        Some(&list),
        &options(vec![text.clone()], vec![Group::Complete, Group::Space]),
    )
    .unwrap();
    std::fs::remove_file(text).ok();
    assert_eq!((report.words, report.gaps), (3, 1));
    // hello / world 敲 1 个字母就是首选：各 2 键；helo 不在词表，敲完 4 键，首选是 hello 要回车，实际 5 键
    assert_eq!(report.top1.letters, 14);
    assert_eq!(report.top1.article, 8);
    assert_eq!(report.top1.actual, 9);
    assert_eq!(report.hits[0], [3, 2, 2]);
    // 按空格：词表里的词不被改，helo 被换成 hello
    assert_eq!(
        (report.space_known.total, report.space_known.changed()),
        (2, 0)
    );
    assert_eq!(
        report.space_unknown,
        SpaceTally {
            total: 1,
            case_only: 0,
            replaced: 1
        }
    );
    assert_eq!(report.keystrokes.count(), 14);
    // 评测不学习
    assert_eq!(engine.learner().weight("hello"), 0);
}

#[test]
fn same_input_same_numbers() {
    let text = temp_file("twice.txt", "hello world help held word hello\n");
    let run_once = || {
        let (mut engine, list) = engine(WORDS);
        let report = run(
            &mut engine,
            Some(&list),
            &options(vec![text.clone()], vec![Group::Typo, Group::Prefix]),
        )
        .unwrap();
        (
            report.typo.actual,
            report.typo_kinds,
            report.prefix.actual,
            report.prefix_seen.clone(),
            report.typo_misses.clone(),
        )
    };
    let first = run_once();
    assert_eq!(first, run_once());
    std::fs::remove_file(text).ok();
}

#[test]
fn personal_case_builds_state_through_the_engine() {
    let (mut engine, list) = engine("open\topen\t5000\nopenfga\tOpenFGA\t0\n");
    let personal = temp_file(
        "personal.tsv",
        "长期常用\t3\topenfabrica\topenf\t3\topenf,openfa\tfirst\n",
    );
    let mut options = options(vec![], vec![Group::Personal]);
    options.personal_cases = personal.clone();
    let report = run(&mut engine, Some(&list), &options).unwrap();
    std::fs::remove_file(personal).ok();
    let outcome = &report.personal[0];
    // 第一次候选里没有，整词原样上屏学进个人词表；后两次从候选里选中
    assert_eq!(outcome.selected, 2);
    // 词表只有两个词，背景记录造不满 3 条（open 以 openf 开头的词要避开，OpenFGA 也是）
    assert!(outcome.background <= 3);
    assert!(outcome.passed, "{outcome:?}");
    // 跑完换回空的学习器
    assert_eq!(engine.learner().weight("openfabrica"), 0);
}

#[test]
fn no_word_list_is_an_error() {
    let (mut engine, _) = engine(WORDS);
    let result = run(&mut engine, None, &options(vec![], vec![Group::Complete]));
    assert!(matches!(result, Err(EnglishEvalError::NoWordList)));
}

#[test]
fn space_changes_split_case_from_replacement() {
    assert_eq!(Change::of("or", "or"), Change::Same);
    assert_eq!(Change::of("or", "OR"), Change::CaseOnly);
    assert_eq!(Change::of("Ubuntu", "ubuntu"), Change::CaseOnly);
    assert_eq!(Change::of("helo", "hello"), Change::Replaced);
    // 小写改成大写才进清单；大写改小写、换词都不算
    assert!(Change::upcased("or", "OR"));
    assert!(Change::upcased("may", "May"));
    assert!(!Change::upcased("Ubuntu", "ubuntu"));
    assert!(!Change::upcased("helo", "Hello"));
    assert!(!Change::upcased("or", "or"));

    let mut tally = SpaceTally::default();
    for change in [
        Change::Same,
        Change::CaseOnly,
        Change::Replaced,
        Change::CaseOnly,
    ] {
        tally.add(change);
    }
    assert_eq!(
        tally,
        SpaceTally {
            total: 4,
            case_only: 2,
            replaced: 1
        }
    );
    assert_eq!(tally.changed(), 3);
}

#[test]
fn space_pass_lists_upcased_words() {
    let words = "OR\tor\t6540\nOK\tok\t5000\nhello\thello\t5600\nhelp\thelp\t5400\n";
    let (mut engine, list) = engine(words);
    let text = temp_file("upcased.txt", "or ok or hello helo Or\n");
    let mut options = options(vec![text.clone()], vec![Group::Space]);
    options.show_misses = 1;
    let report = run(&mut engine, Some(&list), &options).unwrap();
    std::fs::remove_file(text).ok();
    // or / ok / or / Or 都只改了大小写（Or 首字母大写，其余照词表写法，上屏 OR）；helo 换成了 hello
    assert_eq!(
        report.space_known,
        SpaceTally {
            total: 5,
            case_only: 4,
            replaced: 0
        }
    );
    assert_eq!(
        report.space_unknown,
        SpaceTally {
            total: 1,
            case_only: 0,
            replaced: 1
        }
    );
    // 清单只收全小写敲的：or → OR 两次、ok → OK 一次；Or 敲的不是全小写，不进
    let upcased: Vec<(&str, &str, usize)> = report
        .space_upcased
        .iter()
        .map(|((typed, committed), count)| (typed.as_str(), committed.as_str(), *count))
        .collect();
    assert_eq!(upcased, [("ok", "OK", 1), ("or", "OR", 2)]);
    // --misses 1：只列次数最多的一条
    let text = report.to_string();
    assert!(
        text.contains("小写词被改成大写 3 次（2 种）：or → OR 2\n"),
        "{text}"
    );
    assert!(
        text.contains("词表里有 4 / 5（80.0%）只改大小写 4 换成别的词 0"),
        "{text}"
    );
}
