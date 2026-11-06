//! 英文评测的错误。

use std::path::PathBuf;

/// 英文评测跑不起来的原因。
#[derive(Debug, thiserror::Error)]
pub enum EnglishEvalError {
    #[error("cannot read {path}: {source}")]
    Read {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    /// 用例表某一行格式不对。
    #[error("{path}:{line}: {reason}")]
    Case {
        path: PathBuf,
        line: usize,
        reason: String,
    },

    /// 没有英文词表：英文模式下没有候选，评了也全是 0。
    #[error("English word list not found; pass --english or generate data/generated/english.tsv")]
    NoWordList,
}
