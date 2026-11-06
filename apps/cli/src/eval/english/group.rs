//! 英文评测里可以单独挑出来跑的几组。

/// `--eval-english-groups` 的取值。
#[derive(Debug, Clone, Copy, PartialEq, Eq, clap::ValueEnum)]
pub enum Group {
    /// 补全：首选 / 前三省键率、敲 1–3 个字母时的命中率、每键耗时。
    Complete,

    /// 错字组：每词注一处错、整个敲完再看候选。
    Typo,

    /// 错前缀组：前三个字母里注一处错、接着敲，目标词进前三就选。
    Prefix,

    /// 错前缀固定用例表（`tools/eval/english/prefix-typos.tsv`）。
    Cases,

    /// 正常词被改错率：每个词敲完按空格。
    Space,

    /// 个人词回归用例（`tools/eval/english/personal.tsv`）。
    Personal,

    /// 下一词预测：照补全组逐词敲，开敲之前先看预测，第一个正好是目标词就按 Tab 接受。
    Predict,
}

impl Group {
    /// 全部分组，报告按这个顺序排；`--eval-english-groups` 不给时全跑。
    pub const ALL: [Self; 7] = [
        Self::Complete,
        Self::Typo,
        Self::Prefix,
        Self::Cases,
        Self::Space,
        Self::Personal,
        Self::Predict,
    ];
}
