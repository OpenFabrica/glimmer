//! 英文词表：精确查询、前缀区间与补全、短前缀预计算，以及领域词库中的英文名称；另有误拼对照表。

mod entry;
mod lookup;
mod misspellings;
mod short_top;

use std::collections::HashMap;
use std::path::Path;

use crate::error::DictionaryError;
pub use entry::WordEntry;
pub use misspellings::Misspellings;
pub use short_top::SHORT_PREFIX_TOP;
use short_top::ShortTop;

/// 英文词表：按小写编码查原样大小写的词，也能按前缀取区间、补全。
/// 文件格式 TSV：`词\t编码[\t词频]`（编码缺省为 [`english_code`]，即小写去撇号；词频缺省 0）。
///
/// 一个编码可以对多个词（`well` / `we'll` 同码 `well`）：同码的词按文件里出现的先后排，
/// 同一编码下同一个词只留第一次出现的那条。
#[derive(Debug, Default)]
pub struct WordList {
    /// 按编码字节序排好（同码按文件先后），前缀查询两头二分定位。
    entries: Vec<WordEntry>,

    /// 词频前缀和：`mass[i]` 是 `entries[..i]` 的词频总和，长度 `entries.len() + 1`（空表为空）。
    mass: Vec<u64>,

    /// 1–2 个字母前缀的词频前 [`SHORT_PREFIX_TOP`]。
    short: ShortTop,
}

impl WordList {
    /// 解析 TSV 词表，`#` 开头为注释、空行忽略。
    pub fn parse(source: &str) -> Result<Self, DictionaryError> {
        let mut entries: Vec<WordEntry> = Vec::new();
        for (index, raw) in source.lines().enumerate() {
            let line = raw.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            let mut fields = line.split('\t');
            let word = fields
                .next()
                .filter(|s| !s.is_empty())
                .ok_or(DictionaryError::Line {
                    line: index + 1,
                    reason: "missing word",
                })?;
            let code = fields
                .next()
                .filter(|s| !s.is_empty())
                .map(str::to_ascii_lowercase)
                .unwrap_or_else(|| english_code(word));
            let frequency = fields
                .next()
                .map(|s| s.trim().parse::<u32>())
                .transpose()
                .map_err(|_| DictionaryError::Line {
                    line: index + 1,
                    reason: "frequency is not a non-negative integer",
                })?
                .unwrap_or(0);
            entries.push(WordEntry {
                code,
                word: word.to_owned(),
                frequency,
            });
        }
        let list = Self::assemble(entries);
        tracing::debug!(words = list.entries.len(), "英文词表加载完成");
        Ok(list)
    }

    /// 从附加词库提取显式别名：`@` 后为小写 ASCII 字母数字码，
    /// 例如 `C++` → `@cpp`、`Git分支` → `@gitfenzhi`。输出保持源词形；
    /// 未标记的普通拼音词与旧式 AI 混合词不进入英文表。
    /// 同一编码保留最先加载的词库（这里仍是一码一词），关闭或移除词库后重新构建即可撤销补全。
    pub fn from_dictionaries(dictionaries: &[crate::Dictionary]) -> Self {
        let mut first: HashMap<&str, &str> = HashMap::new();
        let mut entries: Vec<WordEntry> = Vec::new();
        for entry in dictionaries.iter().flat_map(crate::Dictionary::entries) {
            let Some(code) = entry.pinyin.strip_prefix('@') else {
                continue;
            };
            if !valid_alias_code(code)
                || entry.text.is_empty()
                || entry.text.chars().any(char::is_control)
            {
                continue;
            }
            if let Some(&text) = first.get(code) {
                if text != entry.text {
                    // 导入词库可能没有经过生成器；保留既有加载顺序，同时明确报告冲突。
                    // 不记录词形或输入码，避免把项目内部名称写入运行日志。
                    tracing::warn!("领域词库存在冲突的别名编码，保留先加载的词条");
                }
                continue;
            }
            first.insert(code, entry.text);
            entries.push(WordEntry {
                code: code.to_owned(),
                word: entry.text.to_owned(),
                frequency: entry.frequency,
            });
        }
        Self::assemble(entries)
    }

    /// 读文件再 [`WordList::parse`]。
    pub fn from_path(path: impl AsRef<Path>) -> Result<Self, DictionaryError> {
        Self::parse(&std::fs::read_to_string(path)?)
    }

    /// 按编码稳定排序（同码保持文件先后）、同码同词去重，再算前缀和与短前缀表。
    fn assemble(mut entries: Vec<WordEntry>) -> Self {
        entries.sort_by(|a, b| a.code.cmp(&b.code));
        // 同码的词排在一起，组都很小（一般 1 条，撇号撞码 2–3 条），组内线性比就够，不用整表建哈希
        let mut kept: Vec<WordEntry> = Vec::with_capacity(entries.len());
        let mut group_start = 0;
        for entry in entries {
            if kept.last().is_none_or(|last| last.code != entry.code) {
                group_start = kept.len();
            } else if kept[group_start..].iter().any(|e| e.word == entry.word) {
                continue;
            }
            kept.push(entry);
        }
        let mut mass = Vec::new();
        if !kept.is_empty() {
            mass.reserve(kept.len() + 1);
            let mut total = 0u64;
            mass.push(0);
            for entry in &kept {
                total += u64::from(entry.frequency);
                mass.push(total);
            }
        }
        let short = ShortTop::build(&kept);
        Self {
            entries: kept,
            mass,
            short,
        }
    }
}

#[cfg(test)]
mod tests;

/// 领域别名须以字母开头，只含小写 ASCII 字母数字，长度不超过 128 字节。
/// 不接受标点或空白，以免与选词、命令和普通拼音音节键混淆。
pub fn valid_alias_code(code: &str) -> bool {
    !code.is_empty()
        && code.len() <= 128
        && code.as_bytes()[0].is_ascii_lowercase()
        && code
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit())
}

/// 英文词 → 检索编码：ASCII 转小写、去掉撇号（直撇 `'` 与弯撇 `’`），`don't` → `dont`、`I'm` → `im`。
/// 词表、语料工具、引擎都只用这一个函数算编码，免得两边算法不一致查不到。
pub fn english_code(word: &str) -> String {
    word.chars()
        .filter(|&c| c != '\'' && c != '\u{2019}')
        .map(|c| c.to_ascii_lowercase())
        .collect()
}
