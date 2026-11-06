//! `english-case` 子命令的参数。

use std::path::PathBuf;

/// 统计语料里原样写法次数的参数。
pub struct CaseOptions {
    /// 语料文件：只给大小写规范的书面文本（`data/corpus/en/train/fineweb.txt`），聊天与 IRC 大量全小写，定不了大小写。
    pub corpus: Vec<PathBuf>,

    /// 英文词表：只输出转小写后在词表里的写法，文件才不至于几十万行。
    pub words: PathBuf,

    /// 输出文件。
    pub output: PathBuf,
}
