//! 英文 n-gram 统计的小样例：临时目录里放词表与语料，跑完读回三张表，再用 `NgramModel` 加载。

use std::path::{Path, PathBuf};

use glimmer_core::sentence::{Context, LanguageModel};
use glimmer_lm::NgramModel;

use super::{NgramOptions, convert};

/// 词表：`green` 不在里面；`I` 大写、`we'll` 与 `well` 同码。
const WORDS: &str = "I\ti\t7000\nlike\tlike\t6000\ntea\ttea\t5000\nis\tis\t7000\n\
good\tgood\t6000\nwe'll\twell\t5000\nwell\twell\t6000\ngo\tgo\t6000\n";

/// 跑一次统计，返回输出目录。
fn run(tag: &str, corpus: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "glimmer-english-ngram-{}-{tag}",
        std::process::id()
    ));
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("english.tsv"), WORDS).unwrap();
    std::fs::write(dir.join("corpus.txt"), corpus).unwrap();
    convert(
        &NgramOptions {
            corpus: vec![dir.join("corpus.txt")],
            words: dir.join("english.tsv"),
            prefix: "lm-en".to_owned(),
            min_count: 1,
            max_bigrams: 1000,
            min_trigram_count: 1,
            max_trigrams: 1000,
            max_trigram_entries: 10_000,
        },
        &dir,
    )
    .unwrap();
    dir
}

/// 读回一张表的正文行（去掉 `#` 开头的表头）。
fn rows(dir: &Path, table: &str) -> Vec<String> {
    std::fs::read_to_string(dir.join(format!("lm-en-{table}.tsv")))
        .unwrap()
        .lines()
        .filter(|l| !l.starts_with('#'))
        .map(str::to_owned)
        .collect()
}

/// 二元表里只看「前词 后词」两列。
fn bigrams(dir: &Path) -> Vec<String> {
    rows(dir, "bigram")
        .iter()
        .map(|r| r.rsplit_once('\t').unwrap().0.replace('\t', " "))
        .collect()
}

#[test]
fn unknown_words_break_instead_of_joining_neighbors() {
    let dir = run("unknown", "I like green tea\n");
    let bigrams = bigrams(&dir);
    std::fs::remove_dir_all(&dir).unwrap();
    // green 不在词表：like 与 tea 不能接起来，tea 也不是句首
    assert!(bigrams.contains(&"<s> i".to_owned()));
    assert!(bigrams.contains(&"i like".to_owned()));
    assert!(!bigrams.contains(&"like tea".to_owned()));
    assert!(!bigrams.contains(&"<s> tea".to_owned()));
}

#[test]
fn gaps_break_without_starting_a_sentence() {
    let dir = run("gap", "I like 3 tea\n");
    let bigrams = bigrams(&dir);
    let trigrams = rows(&dir, "trigram");
    std::fs::remove_dir_all(&dir).unwrap();
    assert!(!bigrams.contains(&"like tea".to_owned()));
    assert!(!bigrams.contains(&"<s> tea".to_owned()));
    assert_eq!(trigrams, ["<s>\ti\tlike\t1"]);
}

#[test]
fn sentence_ends_start_a_new_sentence() {
    let dir = run("end", "I like tea. Tea is good!\nwell\n");
    let unigram = rows(&dir, "unigram");
    let bigrams = bigrams(&dir);
    let trigrams = rows(&dir, "trigram");
    std::fs::remove_dir_all(&dir).unwrap();
    // 三句：两个句末标点 + 换行
    assert!(unigram.contains(&"<s>\t3".to_owned()));
    assert!(bigrams.contains(&"<s> tea".to_owned()));
    assert!(bigrams.contains(&"<s> well".to_owned()));
    assert!(!bigrams.contains(&"tea tea".to_owned()));
    assert!(trigrams.contains(&"<s>\ttea\tis\t1".to_owned()));
}

#[test]
fn apostrophe_words_are_their_own_tokens() {
    let dir = run("apostrophe", "We’ll go well\nwell, we'll go\n");
    let unigram = rows(&dir, "unigram");
    let bigrams = bigrams(&dir);
    std::fs::remove_dir_all(&dir).unwrap();
    assert!(unigram.contains(&"we'll\t2".to_owned()));
    assert!(unigram.contains(&"well\t2".to_owned()));
    assert!(bigrams.contains(&"we'll go".to_owned()));
    assert!(bigrams.contains(&"go well".to_owned()));
    assert!(bigrams.contains(&"well we'll".to_owned()));
}

#[test]
fn the_tables_load_back_into_ngram_model() {
    let dir = run("load", &"I like tea. I like tea. I go\n".repeat(5));
    let model = NgramModel::from_paths(
        &dir.join("lm-en-unigram.tsv"),
        &dir.join("lm-en-bigram.tsv"),
        Some(&dir.join("lm-en-trigram.tsv")),
    )
    .unwrap();
    std::fs::remove_dir_all(&dir).unwrap();
    assert!(model.trigram_count() > 0);
    let after_i = Context {
        previous: Some("i"),
        earlier: None,
    };
    let like = model.log_prob(after_i, "like").unwrap();
    let go = model.log_prob(after_i, "go").unwrap();
    assert!(like > go, "like {like} go {go}");
    // 不在词表里的词模型里也没有
    assert!(model.log_prob(after_i, "green").is_none());
}

/// 真数据自检：加载 `data/generated/lm-en.qj`（先跑 `english-ngram` 与 `pack lm`），打印几组 log 概率与加载耗时。
/// `cargo test -p glimmer-dict-convert --release real_model -- --ignored --nocapture`
#[test]
#[ignore = "要先生成 data/generated/lm-en.qj"]
fn real_model_prefers_the_obvious_continuations() {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/generated/lm-en.qj");
    let started = std::time::Instant::now();
    let model = NgramModel::from_path(&path).unwrap();
    println!(
        "加载 {} 用时 {:?}：{} 词、{} 二元、{} 三元",
        path.display(),
        started.elapsed(),
        model.word_count(),
        model.bigram_count(),
        model.trigram_count()
    );
    // 打印一组候选在同一上文下的 log 概率，返回它们（模型里没有的词是 None）
    let show = |label: &str, context: Context<'_>, words: &[&str]| -> Vec<Option<f64>> {
        let scores: Vec<Option<f64>> = words.iter().map(|w| model.log_prob(context, w)).collect();
        let shown: Vec<String> = words
            .iter()
            .zip(&scores)
            .map(|(w, s)| format!("{w}={}", s.map_or("无".to_owned(), |s| format!("{s:.3}"))))
            .collect();
        println!("{label}：{}", shown.join("  "));
        scores
    };
    let thank = show(
        "上文 thank",
        Context {
            previous: Some("thank"),
            earlier: None,
        },
        &["you", "your", "yon"],
    );
    let know = show(
        "上文 i don't",
        Context {
            previous: Some("don't"),
            earlier: Some("i"),
        },
        &["know", "knot"],
    );
    let start = show("句首", Context::START, &["the", "teh"]);
    show(
        "上文 i think",
        Context {
            previous: Some("think"),
            earlier: Some("i"),
        },
        &["we'll", "well"],
    );
    // 模型里没有的词按负无穷比：它一定最低
    let low = |s: Option<f64>| s.unwrap_or(f64::NEG_INFINITY);
    assert!(low(thank[0]) > low(thank[1]) && low(thank[1]) > low(thank[2]));
    assert!(low(know[0]) > low(know[1]));
    assert!(low(start[0]) > low(start[1]));
}
