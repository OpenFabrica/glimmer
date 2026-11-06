//! 英文下一词预测的显示态：英文词上屏、敲了空格之后组句是空的，候选窗只显示 Core 给的下一个词，Tab 接受。
//! 预测出不出、出什么全在 Core（[`Engine::english_predictions`](glimmer_core::Engine::english_predictions)）；
//! 这里只管什么时候取、Tab 怎么接、什么时候收。照「翻译选中文字」的先例：没有组句、帧里只有候选。
//!
//! 只有 Tab 被接管。其他键先收起预测，再照原来的路走（返回值与预测没显示时一样）。

use glimmer_core::{CandidateKind, CandidateList};
use glimmer_platform::protocol::{Frame, KeyEvent, KeyModifiers};

use super::Router;
use super::key::TAB;

/// 预测帧里第一行的序号：只有 Tab 能接受（数字键照常交给应用），显示「1」会让人去按 1。其余行不标序号。与 macOS 壳一致。
pub const NEXT_WORD_LABEL: &str = "⇥";

/// 这一帧是不是英文下一词预测：没有 preedit、候选全是英文词。翻译评审帧同样没有 preedit，但候选是云端样式，不算；
/// 组句中的帧总有 preedit。协议的 `Frame` 没有专门的标记，各壳（Windows 候选窗、Linux 前端）按这个认。
pub fn is_next_word_frame(frame: &Frame) -> bool {
    frame.preedit.is_empty()
        && !frame.candidates.items.is_empty()
        && frame
            .candidates
            .items
            .iter()
            .all(|candidate| candidate.kind == CandidateKind::English)
}

/// 预测帧第 `position` 行（页内，从 0 起）的序号：第一行 [`NEXT_WORD_LABEL`]，其余为空。
pub fn next_word_label(position: usize) -> &'static str {
    if position == 0 { NEXT_WORD_LABEL } else { "" }
}

impl Router {
    /// 按键处理完（含 Tab 接受）后取一次预测。Core 只在「刚直通了空格」等条件都满足时给，其余时候为空，所以每键都调也不会多出。
    /// 私密输入框（密码 / PIN / 应用要求不记录）里不出：与 macOS 在 Secure Input 时不出一致，Tab 照常交给应用。
    /// 新出的预测先收窗口、丢掉旧位置：旧位置是组句范围或上一个词处的插入点，等壳报来新的插入点再画，免得在旧位置闪一下。
    pub(super) fn refresh_english_prediction(&mut self) {
        self.english_next = None;
        if self.translation.is_some()
            || !self.engine.composition().is_empty()
            || self.engine.is_private()
        {
            return;
        }
        let list = self.engine.english_predictions();
        if list.items.is_empty() {
            return;
        }
        self.hide_candidate_window();
        self.english_next = Some(list);
    }

    /// 预测用户真看得见了：候选窗由壳自己画时随回话就算；由 Server 画时要等插入点位置到了、窗口真画出来
    /// （Windows 上屏走异步编辑会话，位置晚到时 Tab 可能先到）。没画出来时按的 Tab 不接受，照常交给应用，免得接受一个没看见的词。
    pub(super) fn english_prediction_visible(&self) -> bool {
        !self.candidates.draws()
            || self
                .last_shown
                .as_ref()
                .is_some_and(|(frame, _)| is_next_word_frame(frame))
    }

    /// 预测显示着（`shown`）时按了不带修饰键的 Tab：让 Core 接受第一个，返回要上屏的「词 + 空格」。
    /// 带 Shift / Ctrl 的 Tab、Core 不接（过期了、上文变了）都返回 `None`，Tab 照原来的路交给应用。
    pub(super) fn accept_english_prediction_key(
        &mut self,
        shown: bool,
        event: &KeyEvent,
    ) -> Option<String> {
        if !shown || !is_plain_tab(event) {
            return None;
        }
        self.engine.accept_english_prediction(0)
    }

    /// 收起预测显示态（不碰 Core、不收窗口）；之前显示着返回 `true`。
    pub(super) fn end_english_prediction(&mut self) -> bool {
        self.english_next.take().is_some()
    }

    /// 轮询时看一眼：Core 那边上文过期了（停手太久），或这个输入框后来报成了私密，就收起。同一个上文 Core 有缓存，反复调不重算。
    pub(super) fn expire_english_prediction(&mut self) {
        if self.english_next.is_some()
            && (self.engine.is_private() || self.engine.english_predictions().items.is_empty())
        {
            tracing::debug!("英文预测过期，收起");
            self.english_next = None;
        }
    }

    /// 预测帧：没有 preedit、候选就是预测（带释义）、高亮第一个、不分页。
    pub(super) fn english_prediction_frame(&self, list: &CandidateList) -> Frame {
        let mut candidates = list.clone();
        self.engine.annotate(&mut candidates);
        Frame {
            preedit: Vec::new(),
            preedit_mode: self.config.preedit,
            cursor: 0,
            candidates,
            highlight: 0,
            page: 0,
            page_count: 1,
            layout: self.config.layout,
            theme: self.config.theme,
            sentence: None,
            notice: None,
        }
    }
}

/// 不带任何修饰键的 Tab（Caps / 中英模式两个状态位不算）；Shift+Tab 是反向跳焦点，不接受预测。
fn is_plain_tab(event: &KeyEvent) -> bool {
    event.virtual_key == TAB && event.modifiers.chord() == KeyModifiers::default()
}
