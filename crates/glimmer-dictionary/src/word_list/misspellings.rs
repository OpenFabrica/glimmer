//! 英文误拼对照表：常见误拼 → 一个或多个正确写法。

use std::path::Path;

use crate::error::DictionaryError;

/// 误拼 → 正确词的查表（`recieve` → receive，`aache` → ache / cache）。
///
/// 运行时文件格式 TSV，UTF-8，**没有表头**：
///
/// ```text
/// 误拼\t正确词
/// recieve\treceive
/// aache\tache
/// aache\tcache
/// ```
///
/// - 第 1 列误拼：按 ASCII 小写存（读入时转小写），即敲的检索编码。
/// - 第 2 列正确词：原样写法（`ADD`、`Aborigine` 照存）。
/// - 第 3 列起忽略（源表 `06_misspellings.tsv` 的 `source` 列可以留着）。
/// - `#` 开头为注释、空行忽略；行序不限，加载时按误拼排序。
/// - 一个误拼对多个正确词时写多行，同一误拼下按文件里的先后返回；同一对重复出现只留一次。
#[derive(Debug, Default)]
pub struct Misspellings {
    /// (误拼, 正确词)，按误拼字节序排好，同一误拼保持文件先后。
    rows: Vec<(String, String)>,
}

impl Misspellings {
    /// 解析上面格式的 TSV；缺列或某列为空报 [`DictionaryError::Line`]。
    pub fn parse(source: &str) -> Result<Self, DictionaryError> {
        let mut rows: Vec<(String, String)> = Vec::new();
        for (index, raw) in source.lines().enumerate() {
            let line = raw.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            let mut fields = line.split('\t').map(str::trim);
            let (Some(typo), Some(word)) = (fields.next(), fields.next()) else {
                return Err(DictionaryError::Line {
                    line: index + 1,
                    reason: "expected misspelling and correction",
                });
            };
            if typo.is_empty() || word.is_empty() {
                return Err(DictionaryError::Line {
                    line: index + 1,
                    reason: "empty misspelling or correction",
                });
            }
            rows.push((typo.to_ascii_lowercase(), word.to_owned()));
        }
        rows.sort_by(|a, b| a.0.cmp(&b.0));
        // 同一误拼的行排在一起，组很小（一般 1–3 条），组内线性比即可
        let mut kept: Vec<(String, String)> = Vec::with_capacity(rows.len());
        let mut group_start = 0;
        for row in rows {
            if kept.last().is_none_or(|last| last.0 != row.0) {
                group_start = kept.len();
            } else if kept[group_start..].iter().any(|r| r.1 == row.1) {
                continue;
            }
            kept.push(row);
        }
        tracing::debug!(pairs = kept.len(), "英文误拼表加载完成");
        Ok(Self { rows: kept })
    }

    /// 读文件再 [`Misspellings::parse`]。
    pub fn from_path(path: impl AsRef<Path>) -> Result<Self, DictionaryError> {
        Self::parse(&std::fs::read_to_string(path)?)
    }

    /// 误拼正好是 `typo`（已小写）的全部正确词，按文件先后。
    pub fn corrections(&self, typo: &str) -> impl Iterator<Item = &str> {
        let start = self.rows.partition_point(|r| r.0.as_str() < typo);
        self.rows[start..]
            .iter()
            .take_while(move |r| r.0 == typo)
            .map(|r| r.1.as_str())
    }

    /// 以 `prefix`（已小写）开头的误拼（含正好相等的）及其正确词：(误拼, 正确词)，按误拼字节序。两次二分，不分配。
    pub fn with_prefix(&self, prefix: &str) -> &[(String, String)] {
        let start = self.rows.partition_point(|r| r.0.as_str() < prefix);
        let len = self.rows[start..].partition_point(|r| r.0.starts_with(prefix));
        &self.rows[start..start + len]
    }

    /// (误拼, 正确词) 对数。
    pub fn len(&self) -> usize {
        self.rows.len()
    }

    /// 是否一对都没有。
    pub fn is_empty(&self) -> bool {
        self.rows.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn table() -> Misspellings {
        Misspellings::parse(
            "# 注释\nrecieve\treceive\naache\tache\ttypos\naache\tcache\nRecieved\treceived\n\
             aache\tache\nreciept\treceipt\n",
        )
        .unwrap()
    }

    #[test]
    fn loads_and_hits_exact_misspellings() {
        let t = table();
        assert_eq!(t.len(), 5);
        assert_eq!(t.corrections("recieve").collect::<Vec<_>>(), ["receive"]);
        // 误拼列转小写
        assert_eq!(t.corrections("recieved").collect::<Vec<_>>(), ["received"]);
        assert_eq!(t.corrections("recie").count(), 0);
        assert_eq!(t.corrections("zzz").count(), 0);
    }

    #[test]
    fn one_misspelling_maps_to_several_words_once_each() {
        assert_eq!(
            table().corrections("aache").collect::<Vec<_>>(),
            ["ache", "cache"]
        );
    }

    #[test]
    fn prefix_lookup_covers_unfinished_misspellings() {
        let t = table();
        let hits: Vec<(&str, &str)> = t
            .with_prefix("recie")
            .iter()
            .map(|(a, b)| (a.as_str(), b.as_str()))
            .collect();
        assert_eq!(
            hits,
            [
                ("reciept", "receipt"),
                ("recieve", "receive"),
                ("recieved", "received")
            ]
        );
        assert_eq!(t.with_prefix("recieve").len(), 2);
        assert!(t.with_prefix("zz").is_empty());
        assert_eq!(t.with_prefix("").len(), t.len());
    }

    #[test]
    fn rejects_lines_without_a_correction() {
        assert!(Misspellings::parse("recieve\n").is_err());
        assert!(Misspellings::parse("recieve\t\n").is_err());
        assert!(Misspellings::parse("").unwrap().is_empty());
    }
}
