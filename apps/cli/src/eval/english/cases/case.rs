//! 错前缀固定用例表的一行。

use std::path::Path;

use crate::eval::english::EnglishEvalError;

/// 一条固定用例：敲了 `typed`（带错的前缀或整词），期望候选里有 `expected`。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PrefixCase {
    /// 敲的串。
    pub typed: String,

    /// 期望出现的词（大小写照候选）。
    pub expected: String,
}

impl PrefixCase {
    /// 解析 `敲的\t期望词` 两列表；空行与 `#` 开头的行跳过。`path` 只用来报错。
    pub fn parse(text: &str, path: &Path) -> Result<Vec<Self>, EnglishEvalError> {
        let mut cases = Vec::new();
        for (index, line) in text.lines().enumerate() {
            let line = line.trim_end();
            if line.trim().is_empty() || line.starts_with('#') {
                continue;
            }
            let mut fields = line.split('\t');
            match (fields.next(), fields.next(), fields.next()) {
                (Some(typed), Some(expected), None)
                    if !typed.is_empty() && !expected.is_empty() =>
                {
                    cases.push(Self {
                        typed: typed.to_owned(),
                        expected: expected.to_owned(),
                    });
                }
                _ => {
                    return Err(EnglishEvalError::Case {
                        path: path.to_owned(),
                        line: index + 1,
                        reason: "expected two tab-separated columns: typed, expected".to_owned(),
                    });
                }
            }
        }
        Ok(cases)
    }
}
