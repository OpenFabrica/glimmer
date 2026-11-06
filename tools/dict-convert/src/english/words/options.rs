//! `english` 子命令的参数。

use std::path::PathBuf;

/// 生成英文词表的参数。
pub struct WordsOptions {
    /// 源词表：每行 `词\t编码[\t…]`，可带表头（`word\tkey\tfrequency_level…`）。按给的先后读，先后只在别的规则分不出时起作用。
    pub inputs: Vec<PathBuf>,

    /// 展示写法表（`07_display_forms.tsv`）：替换同词的其它大小写写法并排到同码第一，见模块文档。
    pub display_forms: Vec<PathBuf>,

    /// 词频表（`小写词\t词频`，wordfreq Zipf × 1000）；不给就全按底值。
    pub frequency: Option<PathBuf>,

    /// 词频为 0（wordfreq 不认识）的词给的底值，缺省 3000 即 Zipf 3。
    pub frequency_floor: u32,

    /// 语料里原样写法的次数（`english-case` 的输出）：给了就按它定「只差大小写」的写法谁在前。
    pub case_counts: Option<PathBuf>,

    /// 一组「只差大小写」的写法语料次数合计不到这个数就不采信，沿用原规则。
    pub case_min_total: u64,

    /// 输出文件。
    pub output: PathBuf,
}
