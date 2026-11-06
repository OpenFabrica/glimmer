//! `english-ngram` 子命令的参数，阈值的含义与中文 `bigram` 同名参数一样。

use std::path::PathBuf;

/// 从英文语料统计 n-gram 的参数。
pub struct NgramOptions {
    /// 语料文件（UTF-8，一行一句或一条消息）。
    pub corpus: Vec<PathBuf>,

    /// 英文词表 `english.tsv`：词转小写后就是语言模型的 token，不在里面的词当断开。
    pub words: PathBuf,

    /// 输出文件名的前缀：写 `<前缀>-unigram.tsv` / `-bigram.tsv` / `-trigram.tsv`。
    pub prefix: String,

    /// 二元计数下限。
    pub min_count: u32,

    /// 最多输出多少条二元（按计数取前 N）。
    pub max_bigrams: usize,

    /// 三元计数下限。
    pub min_trigram_count: u32,

    /// 最多输出多少条三元（按计数取前 N）。
    pub max_trigrams: usize,

    /// 统计时三元表在内存里最多留多少条，到了就整批剪枝。
    pub max_trigram_entries: usize,
}
