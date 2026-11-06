//! 按键：`ProcessKeyEvent` 的处理，单击 Shift 与属性点击的中英切换。

use glimmer_platform::protocol::{ClientMessage, KeyEvent};

use super::Session;
use crate::frontend::output::Output;
use crate::key::{accepts_prediction, is_modifier, is_release, key_event, should_forward};

impl Session {
    /// 处理一次 IBus 按键，返回 `(吃不吃, 要做的事)`。释放事件只喂单击 Shift 的判定、一律不吃；
    /// 不认识的键与 [`should_forward`] 判为归应用的键不送 Router。显示着英文预测时多送一个 Tab，
    /// 其余不送的键照样归应用，只收起预测（单独按下的修饰键不收）。
    pub fn process_key(&mut self, keyval: u32, state: u32) -> (bool, Vec<Output>) {
        if is_release(state) {
            if self.shift_tap.release(keyval) {
                return (false, self.set_english(!self.english));
            }
            return (false, Vec::new());
        }
        self.shift_tap.press(keyval);
        let Some(event) = key_event(keyval, state, self.english) else {
            return (false, self.end_prediction());
        };
        let accept = self.predicting && accepts_prediction(&event);
        if !accept && !should_forward(&event, self.composing) {
            let out = if is_modifier(event.virtual_key) {
                Vec::new()
            } else {
                self.end_prediction()
            };
            return (false, out);
        }
        self.forward(event)
    }

    /// 切中英模式（对应 tsf 的 `set_english_mode`）：先把组着的拼音原样上屏，再通知 Router、刷属性。
    pub fn set_english(&mut self, english: bool) -> Vec<Output> {
        let mut out = Vec::new();
        self.commit_pending(&mut out);
        self.english = english;
        let session = self.session();
        self.notify(ClientMessage::ModeChanged { session, english });
        out.push(Output::Mode { english });
        tracing::info!(english, "切换中英模式");
        out
    }

    /// 把一个按键送给 Router 并摊开结果。起组句的那一键之后补送光标前文与光标位置（对应 DLL 在编辑会话里的补报）。
    pub(super) fn forward(&mut self, event: KeyEvent) -> (bool, Vec<Output>) {
        let starting = !self.composing;
        let session = self.session();
        let Some(reply) = self.call(ClientMessage::Key { session, event }) else {
            return (false, Vec::new());
        };
        let mut out = Vec::new();
        let consumed = self.apply(reply, &mut out);
        if starting && self.composing {
            if let Some(text) = self.before_cursor.clone() {
                self.notify(ClientMessage::Surrounding { session, text });
            }
            self.report_cursor();
        }
        // 新出的预测：Router 等光标位置再记「显示过」（候选表本身由 IBus / Fcitx5 面板按应用报的光标摆）
        if self.predicting {
            self.report_cursor();
        }
        (consumed, out)
    }
}
