//! 英文候选的排序：精确词规则、上文、个人词封顶、截断线以外的召回、错前缀、最坏情况耗时。

use std::time::{Duration, Instant};

use glimmer_dictionary::{Misspellings, WordList};

use super::*;
use crate::english::test_model::TableModel;
use crate::english::{NgramHistory, SuccessorCache};
use crate::sentence::Context;

/// 只有随包词表的候选。
fn plain(list: &WordList, typed: &str) -> Vec<String> {
    suggest(
        &Sources::lists(Some(list), None, None),
        typed,
        &ScoreParams::DEFAULT,
        18,
    )
}

/// 带模型、上文与后继缓存的候选。
fn with_model(
    list: &WordList,
    model: &TableModel,
    history: NgramHistory<'_>,
    typed: &str,
) -> Vec<String> {
    let mut cache = SuccessorCache::default();
    cache.prepare(model, history, &english_code(typed));
    let sources = Sources {
        main: Some(list),
        personal: None,
        domain: None,
        misspellings: None,
        model: Some(model),
        history,
        successors: Some(&cache),
        dead_ends: None,
    };
    suggest(&sources, typed, &ScoreParams::DEFAULT, 18)
}

#[test]
fn the_typed_spelling_ranks_first_even_when_another_spelling_shares_the_code() {
    // OR 比 or 常见、we'll 比 well 常见也不行：敲的原样写法第一
    let list = WordList::parse(
        "OR\tor\t6000\nor\tor\t5000\norange\torange\t4500\nwe'll\twell\t5600\nwell\twell\t5500\n\
         wellness\twellness\t3500\nI\ti\t7000\nif\tif\t6500\ndon't\tdont\t6000\ndone\tdone\t5500\n\
         may\tmay\t5800\nMay\tmay\t5900\nUbuntu\tubuntu\t3800\n",
    )
    .unwrap();
    let or = plain(&list, "or");
    assert_eq!(or[0], "or");
    assert!(or.contains(&"OR".to_owned()));
    let well = plain(&list, "well");
    assert_eq!(well[..2], ["well", "we'll"]);
    assert_eq!(plain(&list, "may")[0], "may");
    assert_eq!(plain(&list, "OR")[0], "OR");
    // 原样写法不在词表里，同码的词排第一
    assert_eq!(plain(&list, "i")[0], "I");
    assert_eq!(plain(&list, "dont")[0], "don't");
    assert_eq!(plain(&list, "ubuntu"), ["Ubuntu"]);
    // 敲了撇号也按编码查
    assert_eq!(plain(&list, "don't")[0], "don't");
    // 敲的含大写：沿用大小写调整
    assert_eq!(plain(&list, "Or")[0], "Or");
    // 同码几个写法里先给只差大小写的：walter's 是 Walter's，不是更常见的 Walters
    let names = WordList::parse("Walters\twalters\t3900\nWalter's\twalters\t3200\n").unwrap();
    assert_eq!(plain(&names, "walter's")[0], "Walter's");
    assert_eq!(plain(&names, "walters")[0], "Walters");
}

#[test]
fn context_changes_the_first_completion() {
    let list = WordList::parse("then\tthen\t5800\nthere\tthere\t5900\nthey\tthey\t6000\n").unwrap();
    let model = TableModel::new(&[("then", 0.003), ("there", 0.004), ("they", 0.005)])
        .bigram("and", "then", 0.08, 400)
        .bigram("over", "there", 0.2, 300);
    assert_eq!(
        with_model(&list, &model, NgramHistory::Unknown, "th")[0],
        "they"
    );
    assert_eq!(
        with_model(&list, &model, NgramHistory::Previous("and"), "th")[0],
        "then"
    );
    assert_eq!(
        with_model(
            &list,
            &model,
            NgramHistory::Context(Context::after("over")),
            "th"
        )[0],
        "there"
    );
}

#[test]
fn personal_words_are_capped() {
    let main = WordList::parse("hello\thello\t5000\nhelp\thelp\t5700\nheld\theld\t4900\n").unwrap();
    // 误选过一次（次数 1）：个人份额是 0，压不过 hello
    let once = WordList::parse("helloWorld\thelloworld\t1\n").unwrap();
    let ranked = suggest(
        &Sources::lists(Some(&main), Some(&once), None),
        "hel",
        &ScoreParams::DEFAULT,
        18,
    );
    let at = |list: &[String], word: &str| list.iter().position(|w| w == word);
    assert!(
        at(&ranked, "hello") < at(&ranked, "helloWorld"),
        "{ranked:?}"
    );
    // 上千条别的记录之后再误选一次：λ 到顶，但它自己的份额仍是 0
    let mut tsv: String = (0..1200)
        .map(|i| format!("w{i:04}\tw{i:04}\t3\n"))
        .collect();
    tsv.push_str("helloWorld\thelloworld\t1\n");
    let busy = WordList::parse(&tsv).unwrap();
    let ranked = suggest(
        &Sources::lists(Some(&main), Some(&busy), None),
        "hel",
        &ScoreParams::DEFAULT,
        18,
    );
    assert!(
        at(&ranked, "hello") < at(&ranked, "helloWorld"),
        "{ranked:?}"
    );
    // 选过 20 次、前缀上没有强势对手：排第一
    let mut tsv: String = (0..1000)
        .map(|i| format!("w{i:04}\tw{i:04}\t1\n"))
        .collect();
    tsv.push_str("openfabrica\topenfabrica\t20\n");
    let regular = WordList::parse(&tsv).unwrap();
    let main = WordList::parse("OpenFGA\topenfga\t0\nopen\topen\t5500\n").unwrap();
    for prefix in ["openf", "openfa"] {
        let ranked = suggest(
            &Sources::lists(Some(&main), Some(&regular), None),
            prefix,
            &ScoreParams::DEFAULT,
            18,
        );
        assert_eq!(ranked[0], "openfabrica", "{prefix}: {ranked:?}");
    }
}

#[test]
fn a_frequent_follower_beyond_the_frequency_cutoff_is_recalled() {
    // 600 个 s 开头的词，词频都比 sushi 高：只按词频取前 512，sushi 进不来；它紧跟 eat 很常见
    let mut tsv: String = (0..600)
        .map(|i| format!("s{0:03}x\ts{0:03}x\t{1}\n", i, 6000 - i))
        .collect();
    tsv.push_str("sushi\tsushi\t3100\n");
    let list = WordList::parse(&tsv).unwrap();
    let mut unigram: Vec<(String, f64)> = (0..600).map(|i| (format!("s{i:03}x"), 1e-4)).collect();
    unigram.push(("sushi".to_owned(), 1e-6));
    let refs: Vec<(&str, f64)> = unigram.iter().map(|(w, p)| (w.as_str(), *p)).collect();
    let model = TableModel::new(&refs).bigram("eat", "sushi", 0.3, 50);
    assert!(!plain(&list, "s").contains(&"sushi".to_owned()));
    let ranked = with_model(&list, &model, NgramHistory::Previous("eat"), "s");
    assert_eq!(ranked[0], "sushi");
}

#[test]
fn a_trigram_follower_beyond_the_bigram_cutoff_is_recalled() {
    // like 后面有 300 个二元后继，都比 tea 次数多：只看二元按次数取前 256 它进不来；
    // 但「i like」之后 tea 很常见（三元），三元先占名额
    let mut tsv: String = (0..300)
        .map(|i| format!("t{0:03}x\tt{0:03}x\t{1}\n", i, 6000 - i))
        .collect();
    tsv.push_str("tea\ttea\t3000\n");
    let list = WordList::parse(&tsv).unwrap();
    let mut unigram: Vec<(String, f64)> = (0..300).map(|i| (format!("t{i:03}x"), 1e-4)).collect();
    unigram.push(("tea".to_owned(), 1e-6));
    let refs: Vec<(&str, f64)> = unigram.iter().map(|(w, p)| (w.as_str(), *p)).collect();
    let mut model = TableModel::new(&refs);
    for i in 0..300 {
        model = model.bigram("like", &format!("t{i:03}x"), 0.003, 1000 - i);
    }
    let model = model
        .bigram("like", "tea", 0.0001, 2)
        .trigram("i", "like", "tea", 0.4, 80);
    // 三元先占名额，tea 在候选里，而且按三元概率排第一
    let ranked = with_model(
        &list,
        &model,
        NgramHistory::Context(Context::after_two("i", "like")),
        "t",
    );
    assert_eq!(ranked[0], "tea");
    // 只知道前一个词时它被截在二元的 256 名以外（这里一共才 301 个词，正常前缀那一路照样收得到它，
    // 所以截断线单独在缓存的 `top` 上验）
    let mut cache = SuccessorCache::default();
    cache.prepare(&model, NgramHistory::Previous("like"), "t");
    assert!(cache.top("t").iter().all(|s| s.token() != "tea"));
    let mut cache = SuccessorCache::default();
    let history = NgramHistory::Context(Context::after_two("i", "like"));
    cache.prepare(&model, history, "t");
    assert_eq!(cache.top("t")[0].token(), "tea");
    assert_eq!(cache.top("t").len(), 256);
}

#[test]
fn unfinished_typos_reach_the_intended_word() {
    let list = WordList::parse(
        "tomorrow\ttomorrow\t5200\nTommy\ttommy\t3600\nTommie\ttommie\t3000\nTom\ttom\t4500\n\
         receive\treceive\t4800\nrecipe\trecipe\t4600\nrecite\trecite\t3100\n",
    )
    .unwrap();
    assert!(plain(&list, "tomm").contains(&"tomorrow".to_owned()));
    // recie 是误拼 recieve 的开头：靠误拼表
    let table = Misspellings::parse("recieve\treceive\nrecieved\treceived\n").unwrap();
    let sources = Sources {
        misspellings: Some(&table),
        ..Sources::lists(Some(&list), None, None)
    };
    let ranked = suggest(&sources, "recie", &ScoreParams::DEFAULT, 18);
    assert!(ranked[..3].contains(&"receive".to_owned()), "{ranked:?}");
}

#[test]
fn worst_case_inputs_stay_fast() {
    // 3 万个词、一个有上万条后继的前词：只敲 1 个字母、敲 12 个字母以上的错串
    let letters: Vec<char> = ('a'..='z').collect();
    let mut tsv = String::new();
    let mut unigram: Vec<(String, f64)> = Vec::new();
    for i in 0..30_000usize {
        let word: String = (0..8).map(|k| letters[(i / 26usize.pow(k)) % 26]).collect();
        tsv.push_str(&format!("{word}\t{word}\t{}\n", 3000 + i % 4000));
        unigram.push((word, 1e-5));
    }
    let list = WordList::parse(&tsv).unwrap();
    let refs: Vec<(&str, f64)> = unigram.iter().map(|(w, p)| (w.as_str(), *p)).collect();
    let mut model = TableModel::new(&refs);
    for (i, (word, _)) in unigram.iter().enumerate().take(12_000) {
        model = model.bigram("the", word, 1e-4, i as u32 % 500 + 1);
    }
    let history = NgramHistory::Previous("the");
    let mut longest = Duration::ZERO;
    for typed in ["a", "q", "z", "internationalizaton", "qwertyuiopasdfghjkl"] {
        let started = Instant::now();
        let ranked = with_model(&list, &model, history, typed);
        longest = longest.max(started.elapsed());
        assert!(ranked.len() <= 18);
    }
    // debug 构建下也该远小于这个数；release 的真实耗时看 `--eval-english` 的报告
    assert!(longest < Duration::from_millis(500), "{longest:?}");
}

#[test]
fn a_dead_prefix_stays_dead_when_extended() {
    // 对拍：8 个字母以上（两处编辑试过）一个候选都没有的串，往后再接任何字母也没有
    let list = WordList::parse(
        "internationalization\tinternationalization\t4200\nqwerty\tqwerty\t3500\n\
         configuration\tconfiguration\t4600\nconfigure\tconfigure\t4500\n",
    )
    .unwrap();
    let mut checked = 0;
    for code in ["qwertyuiop", "configxxxyz", "internxtqqqq", "zzzzzzzzz"] {
        assert!(stays_empty(code));
        if !plain(&list, code).is_empty() {
            continue;
        }
        checked += 1;
        for c in 'a'..='z' {
            assert!(plain(&list, &format!("{code}{c}")).is_empty(), "{code}{c}");
        }
    }
    assert!(checked >= 2);
    // 不到 8 个字母时不能下这个结论：两处编辑还没试过
    assert!(!stays_empty("qwerty"));
    assert!(stays_empty("foo1"));
}

/// [`stays_empty`] 在两处错误的长度上限之后也成立：13 个字母的串本身有两处错误的变体（不试，所以没有候选），
/// 往后每加一个字母照样什么都没有，跳过与不跳过一样。
#[test]
fn empty_codes_past_the_two_edit_limit_stay_empty() {
    let list = WordList::parse("abcdefghijklmnop\tabcdefghijklmnop\t3000\n").unwrap();
    let sources = Sources::lists(Some(&list), None, None);
    // 12 个字母时两处错误能纠回来
    assert!(!suggest(&sources, "abxdeyghijkl", &ScoreParams::DEFAULT, 9).is_empty());
    // 13 个字母不试两处，一处纠不回，没有候选
    let code = "abxdeyghijklm";
    assert!(suggest(&sources, code, &ScoreParams::DEFAULT, 9).is_empty());
    assert!(stays_empty(code));
    for c in 'a'..='z' {
        let longer = format!("{code}{c}");
        assert!(
            suggest(&sources, &longer, &ScoreParams::DEFAULT, 9).is_empty(),
            "{longer}"
        );
    }
}

/// 敲的原样不是「只差大小写」那组的缺省写法（组里第一个）时，缺省写法紧跟着排第二；敲的就是缺省写法时不提前。
/// 词频照新词表的量级：常用的长词（idea、orange、maybe）分数比另一个大小写写法高。
#[test]
fn the_default_spelling_follows_a_typed_non_default_one() {
    let list = WordList::parse(
        "ID\tid\t4610\nid\tid\t4039\nI'd\tid\t4610\nidea\tidea\t5600\nideas\tideas\t5300\n\
         API\tapi\t3680\napi\tapi\t2590\napis\tapis\t3000\n\
         B\tb\t5350\nb\tb\t5050\nbe\tbe\t6900\nbut\tbut\t6700\n\
         or\tor\t6540\nOR\tor\t3540\norange\torange\t4500\n\
         may\tmay\t5980\nMay\tmay\t4913\nmaybe\tmaybe\t5600\n",
    )
    .unwrap();
    // 敲的不是缺省写法（两个字母以上）：缺省写法第二，其余（I'd、idea）照分数
    let id = plain(&list, "id");
    assert_eq!(id[..2], ["id", "ID"], "{id:?}");
    assert!(id.contains(&"I'd".to_owned()));
    assert_eq!(plain(&list, "api")[..2], ["api", "API"]);
    // 单字母不提前：B 不占第二位，常用词 be、but 照分数排在前面
    assert_eq!(plain(&list, "b")[..3], ["b", "be", "but"]);
    // 敲的就是缺省写法：另一个大小写写法不提前，常用的补全排在它前面
    let or = plain(&list, "or");
    assert_eq!(or[..2], ["or", "orange"], "{or:?}");
    let may = plain(&list, "may");
    assert_eq!(may[..2], ["may", "maybe"], "{may:?}");
    // 敲的含大写：没有原样写法的（Id）按大小写调整，id 调成的 Id 第一、只差大小写的 ID 第二
    assert_eq!(plain(&list, "Id")[..2], ["Id", "ID"]);
    // 敲的原样是缺省写法（ID）、或缺省写法调完与敲的一样（May、OR）：不挪，也不出重复
    for typed in ["ID", "May", "OR"] {
        let out = plain(&list, typed);
        assert_eq!(out[0], typed, "{out:?}");
        let mut unique = out.clone();
        unique.sort();
        unique.dedup();
        assert_eq!(unique.len(), out.len(), "{out:?}");
    }
}
