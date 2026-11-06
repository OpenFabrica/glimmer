//! 敲错变体生成、代价档、先过词表再限额的测试。

use super::*;
use glimmer_dictionary::WordList;

/// `code` 的一处错误变体里 `text` 的档；没有为 `None`。
fn tier_of(code: &str, text: &str) -> Option<CostTier> {
    one_edit_variants(code)
        .into_iter()
        .find(|v| v.text == text)
        .map(|v| v.tier)
}

#[test]
fn each_kind_of_error_is_undone() {
    // 替换（不相邻）：tomm 的 m 本来是 o，当前缀能补出 tomorrow
    assert_eq!(tier_of("tomm", "tomo"), Some(CostTier::Edit));
    // 相邻对调
    assert_eq!(tier_of("teh", "the"), Some(CostTier::Near));
    // 漏敲双写字母的一半
    assert_eq!(tier_of("helo", "hello"), Some(CostTier::Near));
    // 多敲（与左右相同）
    assert_eq!(tier_of("helllo", "hello"), Some(CostTier::Near));
    // 邻键替换：w 在 e 旁边
    assert_eq!(tier_of("hwllo", "hello"), Some(CostTier::Near));
}

#[test]
fn tiers_separate_physical_slips_from_other_edits() {
    // z 不在 e 旁边
    assert_eq!(tier_of("hzllo", "hello"), Some(CostTier::Edit));
    // 漏掉的 e 与左右都不同
    assert_eq!(tier_of("hllo", "hello"), Some(CostTier::Edit));
    // 多出的 x 与左右的 l 不同也不相邻
    assert_eq!(tier_of("helxlo", "hello"), Some(CostTier::Edit));
    // 多出的 w 是左边 e 的邻键：一指按到两个键
    assert_eq!(tier_of("hewllo", "hello"), Some(CostTier::Near));
    // 同一个串几种错误都能得到时留最便宜的：helo 在第 2 位或第 3 位补 l 都是 hello
    let variants = one_edit_variants("helo");
    assert_eq!(variants.iter().filter(|v| v.text == "hello").count(), 1);
}

#[test]
fn variants_never_repeat_or_extend_the_typed_text() {
    for code in ["tomm", "hell", "abb", "aaaa", "x"] {
        let variants = one_edit_variants(code);
        let mut texts: Vec<&str> = variants.iter().map(|v| v.text.as_str()).collect();
        assert!(texts.iter().all(|t| !t.starts_with(code)), "{code}");
        assert!(texts.iter().all(|t| !t.is_empty()), "{code}");
        let before = texts.len();
        texts.sort_unstable();
        texts.dedup();
        assert_eq!(texts.len(), before, "{code}");
    }
    // 删掉末尾多敲的字母得到原串的真前缀：它可能正是想要的整词，留着
    assert_eq!(tier_of("goodd", "good"), Some(CostTier::Near));
    assert!(one_edit_variants("Hello").is_empty());
    assert!(one_edit_variants("he1lo").is_empty());
    assert!(one_edit_variants("").is_empty());
}

#[test]
fn two_edit_variants_exclude_one_edit_ones() {
    let one: Vec<String> = one_edit_variants("abcd")
        .into_iter()
        .map(|v| v.text)
        .collect();
    let two = two_edit_variants("abcd");
    assert!(two.iter().all(|v| v.tier == CostTier::TwoEdits));
    assert!(
        two.iter()
            .all(|v| !one.contains(&v.text) && !v.text.starts_with("abcd"))
    );
    // 换一个字母再对调一次
    assert!(two.iter().any(|v| v.text == "baxd"));
}

#[test]
fn correction_policy() {
    assert!(!should_correct("teh"));
    assert!(should_correct("tehx"));
    assert!(!should_correct("hel1"));
    assert!(!should_correct("x_yz"));
    assert!(!should_correct("Hello"));
    assert!(should_try_two_edits("recieveing", 0));
    assert!(!should_try_two_edits("recieveing", 1));
    assert!(!should_try_two_edits("recieve", 0));
    // 两处错误最多试到 12 个字母
    assert!(should_try_two_edits("recieveingly", 0));
    assert!(!should_try_two_edits("recieveinglyx", 0));
}

/// 测试用词表。
fn list() -> WordList {
    WordList::parse(
        "the\tthe\t7700\nthen\tthen\t6000\nhello\thello\t5000\nhelp\thelp\t5500\n\
         tomorrow\ttomorrow\t5200\ntom\ttom\t4000\ntomato\ttomato\t4100\nzzzz\tzzzz\t10\n\
         good\tgood\t6200\nreceive\treceive\t4800\nhollow\thollow\t200\n",
    )
    .unwrap()
}

#[test]
fn valid_variants_keep_only_word_prefixes() {
    let words = list();
    let found = valid_variants("tomm", &[&words], 64);
    let texts: Vec<&str> = found.iter().map(|v| v.text.as_str()).collect();
    assert!(texts.contains(&"tomo"));
    assert!(texts.contains(&"tom"));
    // 不是任何词前缀的扔掉
    assert!(!texts.contains(&"tomx"));
    // 词频和：tom 下有 tom / tomato / tomorrow
    let tom = found.iter().find(|v| v.text == "tom").unwrap();
    assert_eq!(tom.mass, 4000 + 4100 + 5200);
    // 少于 4 个字母、带数字不纠
    assert!(valid_variants("teh", &[&words], 64).is_empty());
    assert!(valid_variants("hel1o", &[&words], 64).is_empty());
    assert!(valid_variants("hwllo", &[&words], 0).is_empty());
}

#[test]
fn small_limits_prefer_cheap_tiers() {
    let words = list();
    // hwllo：hello 是邻键（Near）；hollo（w → o 不相邻，hollow 的前缀）是 Edit 档
    let all = valid_variants("hwllo", &[&words], 64);
    assert!(all.len() > 1);
    let one = valid_variants("hwllo", &[&words], 1);
    assert_eq!(one.len(), 1);
    assert_eq!(one[0].text, "hello");
    assert_eq!(one[0].tier, CostTier::Near);
    // 档内按词频和降序，整体按档升序
    assert!(
        all.windows(2)
            .all(|w| (w[0].tier, std::cmp::Reverse(w[0].mass))
                <= (w[1].tier, std::cmp::Reverse(w[1].mass)))
    );
}

#[test]
fn the_right_variant_survives_even_when_generated_last() {
    // 正确变体 abcz 只差在最后一位，生成顺序里排在最后；诱饵改的是第一位（最先生成）、词频高得多，
    // 一共 21 个，限额却只有 1。诱饵的首字母都不是 a 的邻键，是 Edit 档
    let mut tsv = String::new();
    for c in (b'b'..=b'z').filter(|&c| !b"qwsz".contains(&c)) {
        tsv.push_str(&format!("{0}bcs\t{0}bcs\t9000\n", c as char));
    }
    tsv.push_str("abcz\tabcz\t1\n");
    let words = WordList::parse(&tsv).unwrap();
    // 21 个诱饵 + abcz + abc（删掉末尾的 s，Edit 档：s 与 c 不同也不相邻）
    assert_eq!(valid_variants("abcs", &[&words], 64).len(), 23);
    // s 与 z 相邻，abcz 是 Near
    let found = valid_variants("abcs", &[&words], 1);
    assert_eq!(found.len(), 1);
    assert_eq!(found[0].text, "abcz");
    assert_eq!(found[0].tier, CostTier::Near);
}

#[test]
fn pruned_generation_equals_filtering_everything() {
    let words = list();
    let lists = [&words];
    for code in [
        "tomm", "hwllo", "helllo", "thwn", "goodd", "recieve", "zzzx", "qqqq",
    ] {
        let mut brute: Vec<(String, CostTier)> = one_edit_variants(code)
            .into_iter()
            .filter(|v| !words.prefix_range(&v.text).is_empty())
            .map(|v| (v.text, v.tier))
            .collect();
        brute.sort();
        let mut pruned: Vec<(String, CostTier)> = valid_variants(code, &lists, usize::MAX)
            .into_iter()
            .map(|v| (v.text, v.tier))
            .collect();
        pruned.sort();
        assert_eq!(pruned, brute, "{code}");
    }
}

#[test]
fn pruned_two_edits_equal_filtering_everything() {
    let words = WordList::parse(
        "receiving\treceiving\t3000\nreceive\treceive\t4800\nbelieving\tbelieving\t3500\n\
         definitely\tdefinitely\t4000\nseparate\tseparate\t4200\nabracadabra\tabracadabra\t100\n",
    )
    .unwrap();
    // abradabra：同一个位置连漏两个字母（ca），两步插入落在同一处，最容易被剪枝误伤
    for code in [
        "recieving",
        "defintely",
        "seperete",
        "beleiving",
        "abradabra",
    ] {
        let mut brute: Vec<String> = two_edit_variants(code)
            .into_iter()
            .filter(|v| !words.prefix_range(&v.text).is_empty())
            .map(|v| v.text)
            .collect();
        brute.sort();
        let mut pruned: Vec<String> = valid_two_edit_variants(code, &[&words], usize::MAX)
            .into_iter()
            .map(|v| v.text)
            .collect();
        pruned.sort();
        assert_eq!(pruned, brute, "{code}");
        assert!(!pruned.is_empty(), "{code}");
    }
    // 两个错都在第一步之后很远（第二步的区间收得最窄），以及改到第一个字母的
    for code in ["definitlye", "xeceiving", "separaet", "believign"] {
        let brute: std::collections::BTreeSet<String> = two_edit_variants(code)
            .into_iter()
            .filter(|v| !words.prefix_range(&v.text).is_empty())
            .map(|v| v.text)
            .collect();
        let pruned: std::collections::BTreeSet<String> =
            valid_two_edit_variants(code, &[&words], usize::MAX)
                .into_iter()
                .map(|v| v.text)
                .collect();
        assert_eq!(pruned, brute, "{code}");
    }
    // 伪随机改坏两三处的串，与全生成逐个对拍
    let bases = ["receiving", "believing", "definitely", "separate"];
    let mut seed: u32 = 11;
    let mut next = |bound: usize| {
        seed = seed.wrapping_mul(1_103_515_245).wrapping_add(12_345);
        (seed >> 16) as usize % bound
    };
    for round in 0..12 {
        let mut code: Vec<u8> = bases[round % bases.len()].bytes().collect();
        for _ in 0..2 + round % 2 {
            let at = next(code.len() - 1);
            match next(4) {
                0 => code[at] = b"aeilnrstv"[next(9)],
                1 => code.insert(at, b"aeilnrstv"[next(9)]),
                2 => code.swap(at, at + 1),
                _ => {
                    code.remove(at);
                }
            }
        }
        let code = String::from_utf8(code).unwrap();
        if code.len() < MIN_TWO_EDIT_LETTERS {
            continue;
        }
        let brute: std::collections::BTreeSet<String> = two_edit_variants(&code)
            .into_iter()
            .filter(|v| !words.prefix_range(&v.text).is_empty())
            .map(|v| v.text)
            .collect();
        let pruned: std::collections::BTreeSet<String> =
            valid_two_edit_variants(&code, &[&words], usize::MAX)
                .into_iter()
                .map(|v| v.text)
                .collect();
        assert_eq!(pruned, brute, "{code}");
    }
    let found = valid_two_edit_variants("abradabra", &[&words], 64);
    assert!(found.iter().any(|v| v.text == "abracadabra"));
    // 不到 8 个字母不试两处
    assert!(valid_two_edit_variants("recieve", &[&words], 64).is_empty());
}

#[test]
fn several_lists_pool_their_mass() {
    let main = list();
    let user = WordList::parse("helloworld\thelloworld\t3\n").unwrap();
    let found = valid_variants("hwllo", &[&user, &main], 64);
    let hello = found.iter().find(|v| v.text == "hello").unwrap();
    assert_eq!(hello.mass, 5000 + 3);
    // 只在个人表里有的前缀也算有效
    let found = valid_variants("hwlloworld", &[&user, &main], 64);
    assert!(found.iter().any(|v| v.text == "helloworld"));
}

/// [`crate::english::DeadEnds`] 的依据：没有补全、一处与两处错误都没有变体的串，往后加哪个字母都还是什么都没有。
/// 用确定的伪随机把词表里的词改坏三四处造串，过了前提的逐个延长一个字母验证。
#[test]
fn no_variants_stay_none_when_typing_on() {
    let words = WordList::parse(
        "receiving\treceiving\t3000\nreceive\treceive\t4800\nbelieving\tbelieving\t3500\n\
         definitely\tdefinitely\t4000\nseparate\tseparate\t4200\nseparately\tseparately\t3900\n",
    )
    .unwrap();
    let lists = [&words];
    let nothing = |code: &str| {
        words.prefix_range(code).is_empty()
            && valid_variants(code, &lists, usize::MAX).is_empty()
            && valid_two_edit_variants(code, &lists, usize::MAX).is_empty()
    };
    let bases = ["receiving", "believing", "definitely", "separately"];
    let mut seed: u32 = 7;
    let mut next = |bound: usize| {
        seed = seed.wrapping_mul(1_103_515_245).wrapping_add(12_345);
        (seed >> 16) as usize % bound
    };
    let mut checked = 0;
    for round in 0..200 {
        let mut code: Vec<u8> = bases[round % bases.len()].bytes().collect();
        for _ in 0..3 + round % 2 {
            let at = next(code.len());
            let letter = b"aeinrstvy"[next(9)];
            match next(3) {
                0 => code[at] = letter,
                1 => code.insert(at, letter),
                _ => {
                    code.remove(at);
                }
            }
        }
        let code = String::from_utf8(code).unwrap();
        if code.len() < MIN_TWO_EDIT_LETTERS || !nothing(&code) {
            continue;
        }
        checked += 1;
        for c in 'a'..='z' {
            let longer = format!("{code}{c}");
            assert!(nothing(&longer), "{code} → {longer}");
        }
    }
    assert!(checked >= 20, "只验证了 {checked} 个串");
}

/// 「生成变体 + 过词表 + 限额到 64」的耗时（release 下跑：
/// `cargo test --release -p glimmer-core variant_timing -- --ignored --nocapture`），数据不在就跳过。
#[test]
#[ignore]
fn variant_timing() {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../data/generated/english.tsv"
    );
    if !std::path::Path::new(path).exists() {
        return;
    }
    let words = WordList::from_path(path).unwrap();
    for code in [
        "tehy",
        "recieved",
        "accomodation",
        "abcdefghijkl",
        "autogenratin",
        "configuatino",
        "authenitcator",
    ] {
        let rounds = 200;
        let start = std::time::Instant::now();
        let mut found = 0;
        for _ in 0..rounds {
            found = valid_variants(code, &[&words], 64).len();
        }
        let one = start.elapsed() / rounds;
        let start = std::time::Instant::now();
        let mut two = 0;
        for _ in 0..rounds {
            two = valid_two_edit_variants(code, &[&words], 64).len();
        }
        eprintln!(
            "{code}（{} 字母）：一处 {one:?} 留 {found}；两处 {:?} 留 {two}",
            code.len(),
            start.elapsed() / rounds
        );
    }
    let typos = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../assets/lexicon/05_english/06_misspellings.tsv"
    );
    let Ok(table) = glimmer_dictionary::Misspellings::from_path(typos) else {
        return;
    };
    for code in ["reci", "recie", "recieved", "acco"] {
        let rounds = 200;
        let start = std::time::Instant::now();
        let mut found = 0;
        for _ in 0..rounds {
            found = crate::english::misspelling_hits(&table, code, 64).len();
        }
        eprintln!("误拼表 {code}：{:?} 留 {found}", start.elapsed() / rounds);
    }
}
