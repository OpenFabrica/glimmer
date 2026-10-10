//! AI 领域词表的严格校验、去重与打包；人工权重与正文出现次数分列记录。

#[cfg(test)]
mod tests;

use std::collections::{BTreeMap, HashSet};
use std::fmt::Write as FmtWrite;
use std::path::{Path, PathBuf};

mod corpus;
mod inventory;
mod validation;

use glimmer_dictionary::Dictionary;
use glimmer_format::Metadata;
use validation::validate;

use crate::error::ConvertError;

/// 校验所有源文件后生成 AI 领域词库与审计表。任何格式错误均带源文件行号返回；
/// 主词库与其他领域词库的重叠项仍留在源文件中，但不重复写入生成词库。
pub fn convert(
    source: &Path,
    exclude: &[PathBuf],
    corpus: &[PathBuf],
    out: &Path,
) -> Result<(), ConvertError> {
    let manifest: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(source.join("sources.json"))?)?;
    let mut existing = HashSet::new();
    for path in exclude {
        let dict = Dictionary::from_path(path)?;
        existing.extend(
            dict.entries()
                .map(|e| (e.text.to_owned(), e.pinyin.to_owned())),
        );
    }
    let text = corpus::load(corpus)?;
    let mut rows = BTreeMap::new();
    let mut paths = vec![source.join("terms.tsv"), source.join("names.tsv")];
    let domains = source.join("domains");
    if domains.exists() {
        for entry in std::fs::read_dir(&domains)? {
            let path = entry?.path();
            if path.is_file() && path.extension().is_some_and(|e| e == "tsv") {
                paths.push(path);
            }
        }
    }
    paths.sort();
    let mut aliases = BTreeMap::new();
    for path in paths {
        for (index, line) in std::fs::read_to_string(&path)?.lines().enumerate() {
            if line.trim().is_empty() || line.starts_with('#') {
                continue;
            }
            let fields: Vec<&str> = line.split('\t').collect();
            validate(&fields).map_err(|reason| ConvertError::Format {
                path: path.clone(),
                line: index + 1,
                reason,
            })?;
            if !manifest
                .get(fields[6])
                .and_then(|s| s.get("license"))
                .and_then(serde_json::Value::as_str)
                .is_some_and(|license| {
                    matches!(
                        license,
                        "GPL-3.0-or-later"
                            | "Apache-2.0"
                            | "CC-BY-SA-4.0"
                            | "MIT"
                            | "MIT AND Unicode-3.0"
                            | "CC-BY-4.0"
                            | "name-facts"
                    )
                })
            {
                return Err(ConvertError::Format {
                    path: path.clone(),
                    line: index + 1,
                    reason: "missing source or unsupported source license".into(),
                });
            }
            if validation::is_alias(&fields)
                && let Some((word, first_path, first_line)) = aliases.insert(
                    fields[1].to_owned(),
                    (fields[0].to_owned(), path.clone(), index + 1),
                )
                && word != fields[0]
            {
                return Err(ConvertError::Format {
                    path: path.clone(),
                    line: index + 1,
                    reason: format!(
                        "alias code '{}' conflicts with '{}' at {}:{}",
                        fields[1],
                        word,
                        first_path.display(),
                        first_line
                    ),
                });
            }
            let key = (fields[0].to_owned(), fields[1].to_owned());
            if rows
                .insert(
                    key,
                    fields.iter().map(|f| (*f).to_owned()).collect::<Vec<_>>(),
                )
                .is_some()
            {
                return Err(ConvertError::Format {
                    path: path.clone(),
                    line: index + 1,
                    reason: "duplicate word and code".into(),
                });
            }
        }
    }
    let mut dictionary = String::from("# AI 与软件开发：词\t编码\t权重\n");
    let mut audit =
        String::from("# 词\t编码\t人工权重\t正文出现次数\t最终权重\t状态\t分类\t来源\n");
    let mut included = 0;
    for ((word, code), fields) in &rows {
        let manual: u32 = fields[2].parse().expect("validated weight");
        let count = corpus::count(&text, word);
        // 专业语料仅辅助词级排序，不冒充通用语料一元计数，不修改全局语言模型。
        let weight = manual.max(count.min(200) as u32);
        let refs: Vec<&str> = fields.iter().map(String::as_str).collect();
        let key = if validation::is_alias(&refs) {
            format!("@{code}")
        } else {
            code.clone()
        };
        let status = if existing.contains(&(word.clone(), key.clone())) {
            "existing"
        } else {
            "included"
        };
        writeln!(
            audit,
            "{word}\t{code}\t{manual}\t{count}\t{weight}\t{status}\t{}\t{}",
            fields[3], fields[6]
        )
        .expect("String write");
        if status == "included" {
            // 专用前缀使 AI / GAN 等恰好也是拼音的缩写不进入中文词图，避免重复候选。
            writeln!(dictionary, "{word}\t{key}\t{weight}").expect("String write");
            included += 1;
        }
    }
    let dict = Dictionary::parse(&dictionary)?;
    if dict.is_empty() {
        return Err(ConvertError::Format {
            path: source.to_owned(),
            line: 0,
            reason: "empty AI dictionary after exclusions".into(),
        });
    }
    let version = rows
        .values()
        .map(|row| row[7].as_str())
        .max()
        .unwrap_or_default()
        .to_owned();
    let coverage = inventory::report(source, &rows, &existing)?;
    let dir = out.join("dicts");
    std::fs::create_dir_all(&dir)?;
    glimmer_core::storage::write_atomic_str(&dir.join("ai.tsv"), &dictionary)?;
    dict.write_qj(&dir.join("ai.qj"), &Metadata {
        name: "AI 与软件开发".into(),
        license: "GPL-3.0-or-later AND Apache-2.0 AND CC-BY-SA-4.0 AND MIT AND Unicode-3.0 AND CC-BY-4.0".into(),
        attribution: "Glimmer contributors；动手学深度学习作者及中文贡献者；Eugene Siow 与 AI Glossary in Mandarin 贡献者；THUOCL；Unicode；Evan You 与 Vue 文档贡献者；Rust 中文社区".into(),
        source: "https://github.com/OpenFabrica/glimmer/tree/main/assets/lexicon/ai".into(),
        version,
        ..Metadata::default()
    })?;
    glimmer_core::storage::write_atomic_str(&out.join("ai-audit.tsv"), &audit)?;
    glimmer_core::storage::write_atomic_str(&out.join("ai-coverage.tsv"), &coverage)?;
    tracing::info!(
        source_rows = rows.len(),
        included,
        excluded = rows.len() - included,
        "AI 领域词库已生成"
    );
    Ok(())
}
