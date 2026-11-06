//! 候选窗口一次绘制要用的全部内容，由帧换算而来；渲染器要的帧由 [`RenderData::render_frame`] 再换一次。

use std::rc::Rc;

use glimmer_platform::protocol::{Frame, PreeditKind};
use glimmer_platform::{LayoutMode, ThemeMode};
use glimmer_render::{Preedit, PreeditSegment, PreeditStyle, Row};
use glimmer_server::dispatch::{is_next_word_frame, next_word_label};

use super::row;
use super::theme::Theme;

/// 一次绘制要用的全部内容。
pub(crate) struct RenderData {
    /// 配色与字体（随 DPI / 深浅重建）。
    pub(super) theme: Rc<Theme>,

    /// 顶部拼音行的各段。
    pub(super) preedit: Vec<(String, PreeditKind)>,

    /// 光标在拼音行里的字符位置。
    pub(super) cursor: usize,

    /// 候选行。
    pub(super) rows: Vec<Row>,

    /// 高亮行下标（页内）。
    pub(super) highlight: usize,

    /// 页码，只有多页时有。
    pub(super) footer: Option<String>,

    /// 整句补全，画在拼音行右侧。
    pub(super) sentence: Option<String>,

    /// 屏幕提示（删候选后的「已删除…」），画在拼音行下方。
    pub(super) notice: Option<String>,

    /// 候选排布。
    pub(super) layout: LayoutMode,

    /// 外观模式；`System` 由窗口按系统主题解析。
    pub(super) theme_mode: ThemeMode,
}

impl RenderData {
    pub(super) fn empty(theme: Rc<Theme>) -> Self {
        Self {
            theme,
            preedit: Vec::new(),
            cursor: 0,
            rows: Vec::new(),
            highlight: usize::MAX,
            footer: None,
            sentence: None,
            notice: None,
            layout: LayoutMode::default(),
            theme_mode: ThemeMode::default(),
        }
    }

    /// 按帧换算出这次要画的内容。英文下一词预测只有 Tab 能接：第一行序号换成 ⇥、其余不标，免得用户去按数字。
    pub(super) fn set(&mut self, frame: &Frame) {
        self.layout = frame.layout;
        self.theme_mode = frame.theme;
        self.preedit = window_preedit(frame);
        self.cursor = frame.cursor;
        let next_word = is_next_word_frame(frame);
        self.rows = frame
            .candidates
            .items
            .iter()
            .enumerate()
            .map(|(i, candidate)| {
                let mut row = row::from_candidate(i, candidate);
                if next_word {
                    row.index = next_word_label(i).to_owned();
                }
                row
            })
            .collect();
        self.highlight = frame.highlight;
        self.footer =
            (frame.page_count > 1).then(|| format!("{}/{}", frame.page + 1, frame.page_count));
        self.sentence = frame.sentence.clone();
        self.notice = frame.notice.clone();
    }

    /// 渲染器要的帧。提示（删了什么词）在渲染器里画在拼音行右侧，与 macOS 一致。
    pub(super) fn render_frame(&self) -> glimmer_render::Frame {
        let preedit = (!self.preedit.is_empty()).then(|| Preedit {
            segments: self
                .preedit
                .iter()
                .map(|(text, kind)| PreeditSegment {
                    text: text.clone(),
                    style: match kind {
                        PreeditKind::Typed => PreeditStyle::Typed,
                        PreeditKind::Rest => PreeditStyle::Rest,
                        PreeditKind::Corrected => PreeditStyle::Struck,
                    },
                })
                .collect(),
            cursor: self.cursor,
        });
        glimmer_render::Frame {
            preedit,
            rows: self.rows.clone(),
            // 协议里 usize::MAX 表示不高亮。
            highlighted: (self.highlight != usize::MAX).then_some(self.highlight),
            footer: self.footer.clone(),
            sentence: self.sentence.clone(),
            status: self.notice.clone(),
        }
    }
}

/// 窗口顶部要画的拼音行：`[general] preedit` 配成「只在行内」时为空（拼音已经在应用里）；
/// 整句补全仍画在这一行上，所以空行时窗口仍可能留出这条线。
fn window_preedit(frame: &Frame) -> Vec<(String, PreeditKind)> {
    if !frame.preedit_mode.in_window() {
        return Vec::new();
    }
    frame
        .preedit
        .iter()
        .map(|segment| (segment.text.clone(), segment.kind))
        .collect()
}

#[cfg(test)]
mod tests {
    use glimmer_platform::PreeditMode;
    use glimmer_platform::protocol::{Frame, PreeditKind, PreeditSegment};

    use super::window_preedit;

    fn frame(mode: PreeditMode) -> Frame {
        Frame {
            preedit: vec![PreeditSegment {
                text: "ni'hao".to_owned(),
                kind: PreeditKind::Typed,
            }],
            preedit_mode: mode,
            ..Frame::default()
        }
    }

    /// 只有「只在行内」不给窗口拼音行；另两档窗口都得画。
    #[test]
    fn window_keeps_the_pinyin_row_unless_inline_only() {
        assert_eq!(window_preedit(&frame(PreeditMode::Both)).len(), 1);
        assert_eq!(window_preedit(&frame(PreeditMode::Window)).len(), 1);
        assert!(window_preedit(&frame(PreeditMode::Inline)).is_empty());
    }
}
