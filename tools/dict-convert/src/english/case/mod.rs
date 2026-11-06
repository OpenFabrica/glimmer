//! 英文写法的大小写次数：规范书面语料里每个**原样写法**（区分大小写、撇号统一成 `'`）出现几次 → `english-case.tsv`（`写法\t次数`），
//! 给 `english --case-counts` 定同码「只差大小写」的写法谁是缺省（`Google` 与 `google`、`or` 与 `OR`）。
//!
//! 缺省命令：`glimmer-dict-convert english-case data/corpus/en/train/fineweb.txt`（只用 FineWeb：聊天、IRC 大量全小写，不能拿来定大小写）。
//!
//! 词与句子边界全用 `glimmer_core::english::tokenize`，只是把它的小写词对回原文写法（见 [`surface::line_words`]）；
//! 句首的词、全大写行与标题行里的词、多词专名里的大写词（`South Africa` 的 South）不计。一行就是 `tokenize` 的一次输入，与 `english-ngram` 一样。
//! 输出只留转小写后在词表里的写法，按次数降序、同次数按写法排。

mod options;
mod surface;

use std::collections::{HashMap, HashSet};
use std::fs::File;
use std::io::{BufRead, BufReader, BufWriter, Write};

use glimmer_dictionary::WordList;

use crate::error::ConvertError;

pub use options::CaseOptions;
use surface::line_words;

/// 扫语料数原样写法，写出 `写法\t次数`。
pub fn convert(options: &CaseOptions) -> Result<(), ConvertError> {
    let list = WordList::from_path(&options.words)?;
    let vocabulary: HashSet<String> = list
        .entries()
        .map(|(_, word, _)| word.to_ascii_lowercase())
        .collect();
    let mut counts: HashMap<String, u64> = HashMap::new();
    let (mut lines, mut counted, mut initial, mut heading, mut in_name, mut mismatched) =
        (0u64, 0u64, 0u64, 0u64, 0u64, 0u64);
    for path in &options.corpus {
        tracing::info!(path = %path.display(), "统计大小写");
        for line in BufReader::new(File::open(path)?).lines() {
            let line = line?;
            lines += 1;
            let words = line_words(&line);
            mismatched += u64::from(words.mismatched);
            initial += words.initial as u64;
            heading += words.heading as u64;
            in_name += words.in_name as u64;
            counted += words.counted.len() as u64;
            for raw in words.counted {
                *counts.entry(raw).or_insert(0) += 1;
            }
        }
    }
    let mut rows: Vec<(String, u64)> = counts
        .into_iter()
        .filter(|(raw, _)| vocabulary.contains(&raw.to_ascii_lowercase()))
        .collect();
    rows.sort_unstable_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
    let mut writer = BufWriter::new(File::create(&options.output)?);
    writeln!(
        writer,
        "# 由 glimmer-dict-convert english-case 从语料统计（句首、全大写行与标题行、多词专名里的大写词不计）。写法\\t次数"
    )?;
    for (raw, count) in &rows {
        writeln!(writer, "{raw}\t{count}")?;
    }
    writer.flush()?;
    tracing::info!(
        path = %options.output.display(),
        lines,
        counted,
        skipped_initial = initial,
        skipped_heading = heading,
        skipped_in_name = in_name,
        mismatched_lines = mismatched,
        spellings = rows.len(),
        "大小写次数已写出"
    );
    Ok(())
}

#[cfg(test)]
mod tests;
