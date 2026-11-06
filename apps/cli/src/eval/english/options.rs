//! 英文评测的参数（来自命令行）。

use std::path::PathBuf;

use super::Group;

/// 一次英文评测要读什么、跑哪几组。
#[derive(Debug, Clone)]
pub struct Options {
    /// 英文文本文件，按顺序接起来评。
    pub paths: Vec<PathBuf>,

    /// 最多评前这么多个词；`None` 为全部。
    pub words: Option<usize>,

    /// 注错的种子。
    pub seed: u64,

    /// 要跑的组；空为全部。
    pub groups: Vec<Group>,

    /// 错前缀固定用例表。
    pub prefix_cases: PathBuf,

    /// 个人词回归用例表。
    pub personal_cases: PathBuf,

    /// 每类最多举几个没做对的例子。
    pub show_misses: usize,
}

impl Options {
    /// 这一组要不要跑。
    pub fn wants(&self, group: Group) -> bool {
        self.groups.is_empty() || self.groups.contains(&group)
    }

    /// 第 `index` 个词（从 0 数）还在评测范围里吗。
    pub fn within(&self, index: usize) -> bool {
        self.words.is_none_or(|limit| index < limit)
    }
}
