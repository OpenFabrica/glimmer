//! 个人词回归用例表的一行。

use std::path::Path;

use super::Rule;
use crate::eval::english::EnglishEvalError;

/// 一条个人词回归用例：先造 `background` 条无关的学习记录，再在敲 `select_input` 时选 `word` 共 `times` 次，
/// 最后在 `prefixes` 的每个前缀上看排序合不合 `rule`。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PersonalCase {
    /// 报告里的名字。
    pub name: String,

    /// 先造多少条无关的学习记录（每条是一次英文候选上屏）。
    pub background: usize,

    /// 个人词。
    pub word: String,

    /// 选它时敲的串：这时候选里有它就选；没有（新词第一次）就把整个词敲完回车原样上屏，它这样进个人词表。
    pub select_input: String,

    /// 选几次。
    pub times: usize,

    /// 量排序用的前缀，每个都要合规则。
    pub prefixes: Vec<String>,

    pub rule: Rule,
}

impl PersonalCase {
    /// 解析 `名字\t背景记录数\t个人词\t选词时敲的\t选过次数\t查的前缀（逗号分隔）\t要求` 七列表；空行与 `#` 开头的行跳过。
    pub fn parse(text: &str, path: &Path) -> Result<Vec<Self>, EnglishEvalError> {
        let mut cases = Vec::new();
        for (index, line) in text.lines().enumerate() {
            let line = line.trim_end();
            if line.trim().is_empty() || line.starts_with('#') {
                continue;
            }
            let error = |reason: &str| EnglishEvalError::Case {
                path: path.to_owned(),
                line: index + 1,
                reason: reason.to_owned(),
            };
            let fields: Vec<&str> = line.split('\t').collect();
            let [name, background, word, select_input, times, prefixes, rule] = fields[..] else {
                return Err(error("expected 7 tab-separated columns"));
            };
            cases.push(Self {
                name: name.to_owned(),
                background: background
                    .parse()
                    .map_err(|_| error("background count is not a number"))?,
                word: word.to_owned(),
                select_input: select_input.to_owned(),
                times: times
                    .parse()
                    .map_err(|_| error("selection count is not a number"))?,
                prefixes: prefixes
                    .split(',')
                    .filter(|p| !p.is_empty())
                    .map(str::to_owned)
                    .collect(),
                rule: Rule::parse(rule)
                    .ok_or_else(|| error("rule must be `first` or `below:<word>`"))?,
            });
        }
        Ok(cases)
    }

    /// 造背景记录时要避开的词：以要查的前缀开头的、个人词本身、要比的那个词——碰了它们，量的就不是「无关记录」了。
    pub fn touches(&self, text: &str) -> bool {
        let lower = text.to_ascii_lowercase();
        lower == self.word.to_ascii_lowercase()
            || self
                .rule
                .rival()
                .is_some_and(|rival| lower == rival.to_ascii_lowercase())
            || self
                .prefixes
                .iter()
                .any(|prefix| lower.starts_with(&prefix.to_ascii_lowercase()))
    }
}
