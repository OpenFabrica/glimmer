//! 按键之外的 IBus 事件：面板上的翻页 / 光标移动 / 点选候选、属性点击、光标位置、光标前文、输入框类型、轮询。

use glimmer_platform::protocol::{
    ClientMessage, KeyEvent, KeyModifiers, ScreenRect, ServerMessage,
};

use super::{SURROUNDING_CHARS, Session};
use crate::frontend::MODE_PROPERTY;
use crate::frontend::output::Output;

/// ibus `IBusInputPurpose` 里的密码与 PIN（`src/ibustypes.h`）。
const PURPOSE_PASSWORD: u32 = 8;

/// PIN 输入框。
const PURPOSE_PIN: u32 = 9;

/// ibus `IBUS_INPUT_HINT_PRIVATE`：应用要求不学习（浏览器无痕窗口）。
const HINT_PRIVATE: u32 = 1 << 11;

impl Session {
    /// 组句中取一次异步结果（云联想 / 整句重排），显示着英文预测时看它过没过期；帧与上次画的相同就什么都不做。
    pub fn poll(&mut self) -> Vec<Output> {
        let mut out = Vec::new();
        let (true, Some(session)) = (self.polling(), self.id) else {
            return out;
        };
        if let Some(reply) = self.call(ClientMessage::Poll { session }) {
            let unchanged =
                matches!(&reply, ServerMessage::Update { frame, .. } if *frame == self.last_frame);
            if !unchanged {
                self.apply(reply, &mut out);
            }
        }
        out
    }

    /// 面板上的翻页按钮：当作按了 PageUp / PageDown。
    pub fn page(&mut self, next: bool) -> Vec<Output> {
        self.synthesize(if next { 0x22 } else { 0x21 }, None)
    }

    /// 面板上的上下移动：当作按了方向键。
    pub fn move_cursor(&mut self, down: bool) -> Vec<Output> {
        self.synthesize(if down { 0x28 } else { 0x26 }, None)
    }

    /// 鼠标点了第 `index` 个候选（页内，从 0 起）：当作按了对应的数字键；第十个以后没有数字键可按，忽略。
    pub fn candidate_clicked(&mut self, index: u32) -> Vec<Output> {
        if index >= 9 {
            return Vec::new();
        }
        let digit = char::from(b'1' + index as u8);
        self.synthesize(u32::from(digit), Some(digit))
    }

    /// 点了属性：中英模式属性切换模式，其余忽略。
    pub fn property_activate(&mut self, name: &str) -> Vec<Output> {
        if name != MODE_PROPERTY {
            return Vec::new();
        }
        self.set_english(!self.english)
    }

    /// 应用报来光标矩形（屏幕坐标）；组句中或显示着预测时转给 Router。
    pub fn set_cursor_location(&mut self, x: i32, y: i32, width: i32, height: i32) {
        self.cursor = Some(ScreenRect {
            left: x,
            top: y,
            right: x.saturating_add(width),
            bottom: y.saturating_add(height),
        });
        if self.polling() {
            self.report_cursor();
        }
    }

    /// 应用报来光标周围的文字，`cursor` 是光标处的字符下标；只留光标前最多 [`SURROUNDING_CHARS`] 字。
    pub fn set_surrounding_text(&mut self, text: &str, cursor: u32) {
        let before: Vec<char> = text.chars().take(cursor as usize).collect();
        let start = before.len().saturating_sub(SURROUNDING_CHARS);
        let before: String = before[start..].iter().collect();
        self.before_cursor = (!before.is_empty()).then_some(before);
    }

    /// 应用报来输入框的用途与提示（`ContentType` 属性）：密码 / PIN / 私密提示算私密。
    pub fn set_content_type(&mut self, purpose: u32, hints: u32) {
        self.set_private(
            matches!(purpose, PURPOSE_PASSWORD | PURPOSE_PIN) || hints & HINT_PRIVATE != 0,
        );
    }

    /// 直接设私密与否（Fcitx5 从能力位算好了送来）；有会话且变了才报给 Router。
    pub fn set_private(&mut self, private: bool) {
        self.private = private;
        self.report_privacy();
    }

    /// 组句中把光标矩形报给 Router。
    pub(super) fn report_cursor(&self) {
        if let (Some(session), Some(rect)) = (self.id, self.cursor) {
            self.notify(ClientMessage::PositionCandidates { session, rect });
        }
    }

    /// 面板操作合成的按键；只在组句中送。
    fn synthesize(&mut self, virtual_key: u32, character: Option<char>) -> Vec<Output> {
        if !self.composing {
            return Vec::new();
        }
        let modifiers = KeyModifiers {
            english_mode: self.english,
            ..KeyModifiers::default()
        };
        self.forward(KeyEvent::new(virtual_key, character, modifiers))
            .1
    }
}
