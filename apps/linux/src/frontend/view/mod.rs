//! 把协议的 [`Frame`] 摊成 IBus 能直接画的视图：preedit 行（[`PreeditView`]）、候选表（[`CandidateView`]）、辅助文字。
//! 这一层只做「帧 → 要显示什么」，不碰 D-Bus 编码；编码在 `ibus::variant`。

mod candidates;
mod preedit;
mod row;

use glimmer_core::{Candidate, Sense};
use glimmer_platform::LayoutMode;
use glimmer_platform::protocol::{Frame, PreeditKind};
use glimmer_server::dispatch::{is_next_word_frame, next_word_label};

pub use candidates::CandidateView;
pub use preedit::PreeditView;
pub use row::CandidateRow;

/// preedit 行：跳过被纠错划掉的原字母（IBus 的属性没有删除线），光标按跳过后的字符数重算；
/// 光标之后「只显示不参与候选」的拼音记成淡色区间。没有内容返回 `None`。
pub fn preedit_view(frame: &Frame) -> Option<PreeditView> {
    let mut text = String::new();
    let mut dimmed = Vec::new();
    let mut cursor = frame.cursor;
    let mut consumed = 0;
    for segment in &frame.preedit {
        let chars = segment.text.chars().count();
        if segment.kind == PreeditKind::Corrected {
            // 被跳过的字母若在光标之前，光标跟着前移。
            cursor -= chars.min(frame.cursor.saturating_sub(consumed));
            consumed += chars;
            continue;
        }
        let start = text.chars().count();
        text.push_str(&segment.text);
        if segment.kind == PreeditKind::Rest {
            dimmed.push(start..start + chars);
        }
        consumed += chars;
    }
    if text.is_empty() {
        return None;
    }
    let cursor = cursor.min(text.chars().count());
    Some(PreeditView {
        text,
        cursor,
        dimmed,
    })
}

/// 数字键能选到的候选数，之后的候选不给序号。
const LABELED: usize = 9;

/// 英文下一词预测帧（判定在 Server 那边，Windows 候选窗也用它）。
/// 预测不算组句：不碰应用里的 preedit，送键规则照没在组句，只多送一个 Tab。
pub fn is_prediction(frame: &Frame) -> bool {
    is_next_word_frame(frame)
}

/// 当前页候选；空页返回 `None`。英文下一词预测只有 Tab 能接：第一行序号是 ⇥、其余不标，免得用户去按数字。
pub fn candidate_view(frame: &Frame) -> Option<CandidateView> {
    if frame.candidates.items.is_empty() {
        return None;
    }
    let next_word = is_prediction(frame);
    let rows = frame
        .candidates
        .items
        .iter()
        .enumerate()
        .map(|(position, candidate)| {
            let label = if next_word {
                next_word_label(position).to_owned()
            } else if position < LABELED {
                (position + 1).to_string()
            } else {
                String::new()
            };
            row(label, candidate)
        })
        .collect();
    Some(CandidateView {
        rows,
        highlight: frame.highlight,
        vertical: frame.layout == LayoutMode::Vertical,
        page: frame.page,
        page_count: frame.page_count,
    })
}

/// 辅助文字（候选表上方那一行）：删候选后的提示优先，其次整句补全（按 Tab 上屏）。
pub fn auxiliary_text(frame: &Frame) -> Option<String> {
    if let Some(notice) = &frame.notice {
        return Some(notice.clone());
    }
    frame
        .sentence
        .as_ref()
        .map(|sentence| format!("{sentence}  Tab"))
}

/// 一个候选：译文的各条释义用「; 」连起来当注解，译词带读音时接在后面（開発(かいはつ)する、develop (dɪˈvɛləp)）。
fn row(label: String, candidate: &Candidate) -> CandidateRow {
    let annotation = candidate.translation.as_ref().and_then(|translation| {
        let senses: Vec<String> = translation.senses().iter().map(sense_text).collect();
        (!senses.is_empty()).then(|| senses.join("; "))
    });
    CandidateRow {
        label,
        text: candidate.text.clone(),
        annotation,
    }
}

/// 一条释义的展示文本：汉字段紧跟假名，英文词与音标之间留一个空格。
fn sense_text(sense: &Sense) -> String {
    let mut out = String::new();
    for segment in sense.furigana() {
        let gap = if segment.text.ends_with(|c: char| c.is_ascii_alphanumeric()) {
            " "
        } else {
            ""
        };
        out.push_str(&segment.text);
        if let Some(reading) = segment.reading {
            out.push_str(&format!("{gap}({reading})"));
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use glimmer_platform::protocol::PreeditSegment;

    use super::*;

    fn segment(text: &str, kind: PreeditKind) -> PreeditSegment {
        PreeditSegment {
            text: text.to_owned(),
            kind,
        }
    }

    #[test]
    fn corrected_letters_are_skipped_and_cursor_shifts() {
        let frame = Frame {
            preedit: vec![
                segment("ni'", PreeditKind::Typed),
                segment("g", PreeditKind::Corrected),
                segment("hao", PreeditKind::Typed),
                segment("ma", PreeditKind::Rest),
            ],
            cursor: 7,
            ..Frame::default()
        };
        let view = preedit_view(&frame).unwrap();
        assert_eq!(view.text, "ni'haoma");
        assert_eq!(view.cursor, 6);
        assert_eq!(view.dimmed, vec![6..8]);
    }

    #[test]
    fn empty_frame_has_no_views() {
        let frame = Frame::default();
        assert!(preedit_view(&frame).is_none());
        assert!(candidate_view(&frame).is_none());
        assert!(auxiliary_text(&frame).is_none());
    }

    #[test]
    fn next_word_frame_is_labelled_with_tab_only() {
        use glimmer_core::{CandidateKind, CandidateList};

        let word = |text: &str| Candidate {
            text: text.to_owned(),
            kind: CandidateKind::English,
            syllables: Vec::new(),
            reading: None,
            translation: None,
        };
        let predicted = Frame {
            candidates: CandidateList {
                items: vec![word("world"), word("there")],
            },
            ..Frame::default()
        };
        let labels: Vec<String> = candidate_view(&predicted)
            .unwrap()
            .rows
            .into_iter()
            .map(|row| row.label)
            .collect();
        assert_eq!(labels, ["⇥", ""]);
        let composing = Frame {
            preedit: vec![segment("wor", PreeditKind::Typed)],
            ..predicted
        };
        let labels: Vec<String> = candidate_view(&composing)
            .unwrap()
            .rows
            .into_iter()
            .map(|row| row.label)
            .collect();
        assert_eq!(labels, ["1", "2"]);
    }

    #[test]
    fn notice_wins_over_sentence() {
        let frame = Frame {
            sentence: Some("你好世界".to_owned()),
            ..Frame::default()
        };
        assert_eq!(auxiliary_text(&frame).as_deref(), Some("你好世界  Tab"));
        let frame = Frame {
            notice: Some("已删除".to_owned()),
            ..frame
        };
        assert_eq!(auxiliary_text(&frame).as_deref(), Some("已删除"));
    }
}
