//! 一条前端指令（[`Output`]）换成 C 能直接读的形状：字符串预先转好 `CString`，下标换成 UTF-8 字节偏移。

mod candidate;
mod segment;

use std::ffi::CString;

use glimmer_linux::frontend::Output;
use glimmer_linux::frontend::view::{CandidateView, PreeditView};

pub use candidate::Candidate;
pub use segment::Segment;

use super::{
    GLIMMER_OUTPUT_AUXILIARY, GLIMMER_OUTPUT_CANDIDATES, GLIMMER_OUTPUT_COMMIT,
    GLIMMER_OUTPUT_MODE, GLIMMER_OUTPUT_PREEDIT,
};

/// 一条指令；各字段只对相应的 `kind` 有意义，其余保持缺省。
#[derive(Debug, Default)]
pub struct Entry {
    /// `GLIMMER_OUTPUT_*`。
    pub kind: i32,

    /// 上屏文字 / preedit 整行 / 辅助文字；收起时为 `None`。
    pub text: Option<CString>,

    /// preedit 光标，UTF-8 字节偏移（Fcitx5 `Text::setCursor` 的单位）。
    pub cursor: i32,

    /// preedit 按样式分的段。
    pub segments: Vec<Segment>,

    /// 当前页候选；收起时为空。
    pub candidates: Vec<Candidate>,

    /// 高亮的候选下标（页内）。
    pub highlight: i32,

    /// 竖排。
    pub vertical: bool,

    /// 有上一页。
    pub has_prev: bool,

    /// 有下一页。
    pub has_next: bool,

    /// 中英模式指令里：现在是英文模式。
    pub english: bool,
}

impl From<Output> for Entry {
    /// 按指令种类摊开；`RegisterProperties` 与 `Mode` 在 Fcitx5 里是同一件事（刷新状态区的中 / 英 / 英+），合成一种。
    /// 英文辅助开关不随指令带：C++ 刷新状态区时经 `glimmer_session_english_assist` 现取。
    fn from(output: Output) -> Self {
        match output {
            Output::Commit(text) => Self {
                kind: GLIMMER_OUTPUT_COMMIT,
                text: Some(c_string(&text)),
                ..Self::default()
            },
            Output::Preedit(view) => preedit(view),
            Output::Candidates(view) => candidates(view),
            Output::Auxiliary(text) => Self {
                kind: GLIMMER_OUTPUT_AUXILIARY,
                text: text.as_deref().map(c_string),
                ..Self::default()
            },
            Output::RegisterProperties { english, .. } | Output::Mode { english, .. } => Self {
                kind: GLIMMER_OUTPUT_MODE,
                english,
                ..Self::default()
            },
        }
    }
}

/// preedit：按淡色区间切段，字符下标的光标换成字节偏移。
fn preedit(view: Option<PreeditView>) -> Entry {
    let Some(view) = view else {
        return Entry {
            kind: GLIMMER_OUTPUT_PREEDIT,
            ..Entry::default()
        };
    };
    let chars: Vec<char> = view.text.chars().collect();
    let mut segments: Vec<Segment> = Vec::new();
    let mut current = String::new();
    let mut current_dimmed = false;
    for (index, c) in chars.iter().enumerate() {
        let dimmed = view.dimmed.iter().any(|range| range.contains(&index));
        if dimmed != current_dimmed && !current.is_empty() {
            segments.push(Segment {
                text: c_string(&std::mem::take(&mut current)),
                dimmed: current_dimmed,
            });
        }
        current_dimmed = dimmed;
        current.push(*c);
    }
    if !current.is_empty() {
        segments.push(Segment {
            text: c_string(&current),
            dimmed: current_dimmed,
        });
    }
    let cursor: usize = chars.iter().take(view.cursor).map(|c| c.len_utf8()).sum();
    Entry {
        kind: GLIMMER_OUTPUT_PREEDIT,
        text: Some(c_string(&view.text)),
        cursor: i32::try_from(cursor).unwrap_or(i32::MAX),
        segments,
        ..Entry::default()
    }
}

/// 候选页：序号（前端已按数字键 / 英文预测算好）与翻页可用性。
fn candidates(view: Option<CandidateView>) -> Entry {
    let Some(view) = view else {
        return Entry {
            kind: GLIMMER_OUTPUT_CANDIDATES,
            ..Entry::default()
        };
    };
    let candidates = view
        .rows
        .iter()
        .map(|row| Candidate {
            label: c_string(&row.label),
            text: c_string(&row.text),
            comment: row.annotation.as_deref().map(c_string),
        })
        .collect();
    Entry {
        kind: GLIMMER_OUTPUT_CANDIDATES,
        candidates,
        highlight: i32::try_from(view.highlight).unwrap_or(0),
        vertical: view.vertical,
        has_prev: view.page > 0,
        has_next: view.page + 1 < view.page_count,
        ..Entry::default()
    }
}

/// 转 C 字符串；文字里混进的 NUL 去掉（C 那边会把它当结尾截断）。
fn c_string(text: &str) -> CString {
    CString::new(text.replace('\0', "")).unwrap_or_default()
}
