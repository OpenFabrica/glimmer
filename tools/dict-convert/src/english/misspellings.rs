//! 误拼表：`assets/lexicon/05_english/06_misspellings.tsv`（typos，`misspelling\tsuggestion\tsource`，有表头）
//! → 运行时的 `english-misspellings.tsv`（`glimmer_dictionary::Misspellings` 认的格式：无表头，`误拼\t正确词`，一对多写多行）。
//!
//! 滤掉三种行，按下面的先后判断，一行只算进第一个命中的那一类：
//!
//! 1. 误拼与正确词忽略大小写后相同（`ok` → `OK`）：不是误拼，是大小写，词表自己就出得来。
//! 2. 正确词不在词表里（逐字节比；词表收词规则见 `words`，`château` 这类不收的词、被展示写法替换掉的小写写法都在这里滤掉）：
//!    引擎给不出这个候选。
//! 3. 误拼本身就是词表里一个正常词：这串字母原样就是某个不带撇号的词（`teh` 若是词表里的词），再「纠正」会把正常词改掉；
//!    或者误拼与正确词同码（`dont` → `don't`），词表同码就出得来，留着是重复。
//!    只与**撇号词**同码的不算（`companys` → `companies`，`company's` 也编码成 `companys`）：敲的字母不是哪个词的原样，
//!    撇号词与纠正词都可能是想要的，两个都给。源表 99,705 行里这种有 160 行（2026-11-06），全是所有格撞码，留着。
//!
//! 同一对重复出现只写一次；输出按源表的先后。

use std::collections::HashSet;
use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::PathBuf;

use glimmer_dictionary::{WordList, english_code};

use crate::error::ConvertError;

/// 生成误拼表的参数。
pub struct MisspellingOptions {
    /// 源表（`06_misspellings.tsv`）。
    pub input: PathBuf,

    /// 英文词表 `english.tsv`。
    pub words: PathBuf,

    /// 输出文件。
    pub output: PathBuf,
}

/// 各类过滤掉的行数，测试与日志用。
#[derive(Debug, Default, PartialEq, Eq)]
pub struct MisspellingReport {
    /// 源表正文行数。
    pub source: usize,

    /// 误拼与正确词只差大小写。
    pub same_ignoring_case: usize,

    /// 正确词不在词表里。
    pub unknown_correction: usize,

    /// 误拼本身是词表里不带撇号的词，或与正确词同码。
    pub typo_is_word: usize,

    /// 重复的对。
    pub duplicate: usize,

    /// 写出的行数。
    pub kept: usize,
}

/// 读源表与词表，按模块文档过滤后写出。
pub fn convert(options: &MisspellingOptions) -> Result<MisspellingReport, ConvertError> {
    let list = WordList::from_path(&options.words)?;
    let words: HashSet<&str> = list.entries().map(|(_, word, _)| word).collect();
    // 有不带撇号的词的编码：敲这串字母原样就是一个词
    let plain_codes: HashSet<&str> = list
        .entries()
        .filter(|(_, word, _)| !word.contains('\''))
        .map(|(code, _, _)| code)
        .collect();
    let mut report = MisspellingReport::default();
    let mut seen: HashSet<(String, String)> = HashSet::new();
    let mut file = BufWriter::new(File::create(&options.output)?);
    writeln!(
        file,
        "# 由 glimmer-dict-convert english-misspellings 从 06_misspellings.tsv（typos，MIT）生成。误拼\\t正确词"
    )?;
    for raw in std::fs::read_to_string(&options.input)?.lines() {
        let line = raw.trim_end();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let mut fields = line.split('\t').map(str::trim);
        let (Some(typo), Some(correction)) = (fields.next(), fields.next()) else {
            continue;
        };
        if typo == "misspelling" || typo.is_empty() || correction.is_empty() {
            continue;
        }
        report.source += 1;
        let typo = typo.to_ascii_lowercase();
        if typo.eq_ignore_ascii_case(correction) {
            report.same_ignoring_case += 1;
        } else if !words.contains(correction) {
            report.unknown_correction += 1;
        } else if plain_codes.contains(english_code(&typo).as_str())
            || english_code(&typo) == english_code(correction)
        {
            report.typo_is_word += 1;
        } else if !seen.insert((typo.clone(), correction.to_owned())) {
            report.duplicate += 1;
        } else {
            writeln!(file, "{typo}\t{correction}")?;
            report.kept += 1;
        }
    }
    file.flush()?;
    tracing::info!(path = %options.output.display(), ?report, "误拼表已写出");
    Ok(report)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 在临时目录里跑一次，返回报告与写出的正文行。
    fn run(tag: &str, source: &str) -> (MisspellingReport, Vec<String>) {
        let dir = std::env::temp_dir().join(format!(
            "glimmer-english-misspellings-{}-{tag}",
            std::process::id()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(
            dir.join("english.tsv"),
            "receive\treceive\t5000\nOK\tok\t5000\nthe\tthe\t7000\ndon't\tdont\t6000\nteh\tteh\t1000\n\
             companies\tcompanies\t5000\ncompany's\tcompanys\t4000\n",
        )
        .unwrap();
        std::fs::write(dir.join("source.tsv"), source).unwrap();
        let options = MisspellingOptions {
            input: dir.join("source.tsv"),
            words: dir.join("english.tsv"),
            output: dir.join("out.tsv"),
        };
        let report = convert(&options).unwrap();
        let rows = std::fs::read_to_string(&options.output)
            .unwrap()
            .lines()
            .filter(|l| !l.starts_with('#'))
            .map(str::to_owned)
            .collect();
        std::fs::remove_dir_all(&dir).unwrap();
        (report, rows)
    }

    #[test]
    fn keeps_real_misspellings_once() {
        let (report, rows) = run(
            "keep",
            "misspelling\tsuggestion\tsource\nrecieve\treceive\ttypos\nRecieve\treceive\ttypos\n",
        );
        assert_eq!(rows, ["recieve\treceive"]);
        assert_eq!((report.source, report.duplicate, report.kept), (2, 1, 1));
    }

    #[test]
    fn drops_rows_that_only_differ_in_case() {
        let (report, rows) = run("case", "ok\tOK\ttypos\n");
        assert!(rows.is_empty());
        assert_eq!(report.same_ignoring_case, 1);
    }

    #[test]
    fn drops_corrections_missing_from_the_word_list() {
        let (report, rows) = run(
            "unknown",
            "chateo\tchâteau\ttypos\nrecieve\tReceive\ttypos\n",
        );
        assert!(rows.is_empty());
        assert_eq!(report.unknown_correction, 2);
    }

    #[test]
    fn keeps_typos_that_only_collide_with_apostrophe_words() {
        // companys 也是 company's 的编码，但不是哪个词的原样：纠正词照留
        let (report, rows) = run("possessive", "companys\tcompanies\ttypos\n");
        assert_eq!(rows, ["companys\tcompanies"]);
        assert_eq!(report.typo_is_word, 0);
    }

    #[test]
    fn drops_typos_that_are_themselves_words() {
        // teh 在词表里是个词；dont 是 don't 的编码
        let (report, rows) = run("word", "teh\tthe\ttypos\ndont\tdon't\ttypos\n");
        assert!(rows.is_empty());
        assert_eq!(report.typo_is_word, 2);
    }
}
