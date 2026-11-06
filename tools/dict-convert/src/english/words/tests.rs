//! 英文词表的行为测试：同码多词与先后、展示写法替换、撇号词编码、词频底值与多写法词频。

use std::path::PathBuf;

use super::{CASE_VARIANT_DISCOUNT, WordsOptions, convert};

/// 测试用的临时目录，每个用例一个，结束时删掉。
struct Scratch(PathBuf);

impl Scratch {
    /// 建一个以 `tag` 区分的空目录。
    fn new(tag: &str) -> Self {
        let dir = std::env::temp_dir().join(format!(
            "glimmer-english-words-{}-{tag}",
            std::process::id()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        Self(dir)
    }

    /// 写一个文件，返回路径。
    fn file(&self, name: &str, body: &str) -> PathBuf {
        let path = self.0.join(name);
        std::fs::write(&path, body).unwrap();
        path
    }

    /// 用给定的源表跑一次，读回 `(词, 编码, 词频)`。
    fn run(
        &self,
        inputs: Vec<PathBuf>,
        display_forms: Vec<PathBuf>,
        frequency: Option<PathBuf>,
    ) -> Vec<(String, String, u32)> {
        self.run_with_case(inputs, display_forms, frequency, None)
    }

    /// 同 [`Self::run`]，另给语料大小写次数表。
    fn run_with_case(
        &self,
        inputs: Vec<PathBuf>,
        display_forms: Vec<PathBuf>,
        frequency: Option<PathBuf>,
        case_counts: Option<PathBuf>,
    ) -> Vec<(String, String, u32)> {
        let output = self.0.join("english.tsv");
        convert(&WordsOptions {
            inputs,
            display_forms,
            frequency,
            frequency_floor: 3000,
            case_counts,
            case_min_total: 20,
            output: output.clone(),
        })
        .unwrap();
        std::fs::read_to_string(output)
            .unwrap()
            .lines()
            .filter(|l| !l.starts_with('#'))
            .map(|l| {
                let f: Vec<&str> = l.split('\t').collect();
                (f[0].to_owned(), f[1].to_owned(), f[2].parse().unwrap())
            })
            .collect()
    }
}

impl Drop for Scratch {
    /// 删掉临时目录。
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

/// 只看词这一列。
fn words(rows: &[(String, String, u32)]) -> Vec<&str> {
    rows.iter().map(|r| r.0.as_str()).collect()
}

/// 某个词的词频。
fn count_of(rows: &[(String, String, u32)], word: &str) -> u32 {
    rows.iter().find(|r| r.0 == word).unwrap().2
}

const HEADER: &str = "word\tkey\tfrequency_level\tspelling_region\ttech_domain\tsource\n";

#[test]
fn keeps_every_spelling_of_a_code_in_precedence_order() {
    let s = Scratch::new("order");
    // 总表按词典序，大写在前：旧规则先到先得会留下 OR / May
    let all = s.file(
        "all.tsv",
        &format!(
            "{HEADER}OR\tor\t35\nor\tor\t35\nMay\tmay\t35\nmay\tmay\t35\nI\ti\t35\ni\ti\t40\n\
             word\tword\t35\nAPI\tapi\t50\napi\tapi\t\t软件\tCSpell\nwe'll\twe'll\t40\nwell\twell\t35\nor\tor\t35\n"
        ),
    );
    let rows = s.run(vec![all], Vec::new(), None);
    // 同码：全小写在前（or / may），等级小的在前（I 35 > i 40，API 50 > api 无等级），不带撇号的在前（well > we'll）；
    // 重复的 or 只留一条；正文里的词 `word` 不是表头，留着，也不打断后面的等级列
    assert_eq!(
        words(&rows),
        [
            "or", "OR", "may", "May", "I", "i", "word", "API", "api", "well", "we'll"
        ]
    );
}

#[test]
fn apostrophe_words_are_kept_and_coded_without_apostrophes() {
    let s = Scratch::new("apostrophe");
    let all = s.file(
        "all.tsv",
        &format!(
            "{HEADER}don't\tdon't\t35\nI'm\ti'm\t35\nrock'n'roll\trock'n'roll\t60\n\
             goin'\tgoin'\t60\n'tis\t'tis\t60\ncafé\tcafé\t40\nx''y\tx''y\t60\nC++\tc++\t\n"
        ),
    );
    let rows = s.run(vec![all], Vec::new(), None);
    let pairs: Vec<(&str, &str)> = rows.iter().map(|r| (r.0.as_str(), r.1.as_str())).collect();
    // 首尾撇号、连着两个撇号、重音字母、符号都不收
    assert_eq!(
        pairs,
        [
            ("don't", "dont"),
            ("I'm", "im"),
            ("rock'n'roll", "rocknroll")
        ]
    );
}

#[test]
fn display_forms_replace_case_variants_and_become_the_default() {
    let s = Scratch::new("display");
    let all = s.file(
        "all.tsv",
        &format!(
            "{HEADER}typescript\ttypescript\t35\nTypeScript\ttypescript\t\nTypescript\ttypescript\t\n\
             vscode\tvscode\t\nwindows\twindows\t35\n"
        ),
    );
    let display = s.file(
        "display.tsv",
        "# 注释\nword\tkey\nTypeScript\ttypescript\nVSCode\tvscode\nYouTube\tyoutube\n",
    );
    let rows = s.run(vec![all], vec![display], None);
    // 小写写法不留（替换不是追加），没有的编码新加，别的词不动
    assert_eq!(words(&rows), ["TypeScript", "VSCode", "windows", "YouTube"]);
}

#[test]
fn display_forms_keep_other_words_of_the_same_code() {
    let s = Scratch::new("display-other");
    let all = s.file("all.tsv", &format!("{HEADER}ios\tios\t\nIo's\tio's\t50\n"));
    let display = s.file("display.tsv", "word\tkey\niOS\tios\n");
    let rows = s.run(vec![all], vec![display], None);
    // Io's 是另一个词（撇号词），不是 iOS 的大小写变体，留着、排在后面
    assert_eq!(words(&rows), ["iOS", "Io's"]);
}

#[test]
fn zero_frequency_words_get_the_floor() {
    let s = Scratch::new("floor");
    let all = s.file(
        "all.tsv",
        &format!("{HEADER}the\tthe\t35\nkubectl\tkubectl\t\nzzz\tzzz\t\n"),
    );
    let frequency = s.file("freq.tsv", "# 编码\t词频\nthe\t7730\nkubectl\t0\n");
    let rows = s.run(vec![all], Vec::new(), Some(frequency));
    assert_eq!(count_of(&rows, "the"), 7730);
    // 频率表里为 0 与没有的都给底值
    assert_eq!(count_of(&rows, "kubectl"), 3000);
    assert_eq!(count_of(&rows, "zzz"), 3000);
}

#[test]
fn the_default_spelling_never_has_a_lower_frequency_than_its_variants() {
    let s = Scratch::new("variants");
    let all = s.file(
        "all.tsv",
        &format!(
            "{HEADER}OR\tor\t35\nor\tor\t35\nits\tits\t35\nit's\tit's\t40\n\
             well\twell\t35\nwe'll\twe'll\t40\ndont\tdont\t60\n"
        ),
    );
    // wordfreq 按小写查，OR 与 or 同分；it's 比 its 常用
    let frequency = s.file(
        "freq.tsv",
        "or\t6500\nits\t5900\nit's\t6300\nwell\t6000\nwe'll\t5100\n",
    );
    let rows = s.run(vec![all], Vec::new(), Some(frequency));
    assert_eq!(count_of(&rows, "or"), 6500);
    // 只差大小写：缺省写法减一个 Zipf
    assert_eq!(count_of(&rows, "OR"), 6500 - CASE_VARIANT_DISCOUNT);
    // 别的写法：自己的词频，但不超过缺省写法
    assert_eq!(count_of(&rows, "it's"), 5900);
    assert_eq!(count_of(&rows, "we'll"), 5100);
    assert_eq!(
        words(&rows),
        ["or", "OR", "its", "it's", "well", "we'll", "dont"]
    );
}

#[test]
fn apostrophe_words_unknown_to_wordfreq_are_dropped_when_frequencies_are_given() {
    let s = Scratch::new("unknown-apostrophe");
    let all = s.file(
        "all.tsv",
        &format!(
            "{HEADER}aardvark's\taardvark's\t35\ntoday's\ttoday's\t35\nAlba's\talba's\t50\n\
             kubectl\tkubectl\t\n"
        ),
    );
    let frequency = s.file("freq.tsv", "today's\t4700\nalba's\t0\n");
    let with = s.run(vec![all.clone()], Vec::new(), Some(frequency));
    // 词频表里没有、为 0 的撇号词不收；不带撇号的照旧给底值
    assert_eq!(words(&with), ["today's", "kubectl"]);
    assert_eq!(count_of(&with, "today's"), 4700);
    assert_eq!(count_of(&with, "kubectl"), 3000);
    // 第一遍没有词频表：全收，好让脚本给它们算词频
    let without = s.run(vec![all], Vec::new(), None);
    assert_eq!(
        words(&without),
        ["aardvark's", "today's", "Alba's", "kubectl"]
    );
}

#[test]
fn corpus_case_counts_pick_the_default_spelling() {
    let s = Scratch::new("case-order");
    // 原规则：google 全小写在前、OR 与 or 同等级小写在前、ID 与 id 同等级小写在前
    let all = s.file(
        "all.tsv",
        &format!(
            "{HEADER}Google\tgoogle\t50\ngoogle\tgoogle\t50\nOR\tor\t35\nor\tor\t35\n\
             ID\tid\t35\nid\tid\t35\nI'd\ti'd\t40\npm\tpm\t50\nPM\tpm\t50\n"
        ),
    );
    let frequency = s.file(
        "freq.tsv",
        "google\t5000\nor\t6500\nid\t4600\ni'd\t4600\npm\t4000\n",
    );
    let case = s.file(
        "case.tsv",
        "# 写法\t次数\nGoogle\t100\ngoogle\t5\nor\t5000\nOR\t40\nID\t500\nid\t10\nPM\t3\npm\t2\n",
    );
    let rows = s.run_with_case(vec![all], Vec::new(), Some(frequency), Some(case));
    // Google、ID 按次数到前面；or 本来就在前；I'd 是撇号词，仍在不带撇号的后面；pm 合计 5 次不到 20，沿用原规则
    assert_eq!(
        words(&rows),
        [
            "Google", "google", "or", "OR", "ID", "id", "I'd", "pm", "PM"
        ]
    );
    // 其它写法 = 第一个 + 1000·log10(次数比)，夹在 [−3000, −300]
    assert_eq!(count_of(&rows, "Google"), 5000);
    assert_eq!(count_of(&rows, "google"), 5000 - 1301);
    assert_eq!(count_of(&rows, "OR"), 6500 - 2097);
    assert_eq!(count_of(&rows, "id"), 4600 - 1699);
    // 次数不够的组：照旧减 1000
    assert_eq!(count_of(&rows, "PM"), 4000 - CASE_VARIANT_DISCOUNT);
}

#[test]
fn case_variant_frequencies_are_clamped_and_unseen_spellings_are_kept() {
    let s = Scratch::new("case-clamp");
    let all = s.file(
        "all.tsv",
        &format!(
            "{HEADER}march\tmarch\t35\nMarch\tmarch\t35\nMARCH\tmarch\t60\n\
             brown\tbrown\t35\nBrown\tbrown\t40\nzed\tzed\t60\nZed\tzed\t60\n"
        ),
    );
    let frequency = s.file("freq.tsv", "march\t5000\nbrown\t5000\nzed\t200\n");
    let case = s.file(
        "case.tsv",
        "March\t1000\nmarch\t900\nbrown\t300\nBrown\t100\nzed\t30\nZed\t1\n",
    );
    let rows = s.run_with_case(vec![all], Vec::new(), Some(frequency), Some(case));
    // 语料里没出现过的 MARCH 照样留着，排最后
    assert_eq!(
        words(&rows),
        ["March", "march", "MARCH", "brown", "Brown", "zed", "Zed"]
    );
    // 次数相近：至少差 300；没出现过：差 3000
    assert_eq!(count_of(&rows, "march"), 5000 - 300);
    assert_eq!(count_of(&rows, "MARCH"), 5000 - 3000);
    assert_eq!(count_of(&rows, "Brown"), 5000 - 477);
    // 第一个的词频很小时下界不低于 1
    assert_eq!(count_of(&rows, "Zed"), 1);
}

#[test]
fn display_forms_still_win_over_corpus_case() {
    let s = Scratch::new("case-display");
    let all = s.file("all.tsv", &format!("{HEADER}I\ti\t35\ni\ti\t40\n"));
    let display = s.file("display.tsv", "word\tkey\nI\ti\n");
    let case = s.file("case.tsv", "i\t5000\nI\t10\n");
    let rows = s.run_with_case(vec![all], vec![display], None, Some(case));
    assert_eq!(words(&rows), ["I"]);
}
