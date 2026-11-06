//! 候选表里的一行。

/// 一个候选：上屏文字加可选的译文注解（IBus 里拼在文字后面、画灰）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CandidateRow {
    /// 序号：数字键能选到的前 9 个是 `1`–`9`，其余为空；英文下一词预测第一行是 `⇥`（只有 Tab 能接受）、其余为空。
    pub label: String,

    /// 候选文字。
    pub text: String,

    /// 学习语言的译文；没有为 `None`。
    pub annotation: Option<String>,
}
