//! 按键：`ProcessKeyEvent` 的处理，单击 Shift 与属性点击的中英切换，开关英文辅助的组合键。

use glimmer_platform::protocol::{ClientMessage, KeyEvent, KeyModifiers};

use super::Session;
use crate::frontend::output::Output;
use crate::key::{
    accepts_prediction, is_english_assist, is_modifier, is_release, key_event, should_forward,
};

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
        // 带 Ctrl / Alt / Super 的字母平时归应用，只有英文辅助这一个组合先送 Router 判断
        if is_english_assist(&event, self.english_assist_keys()) {
            return self.forward_english_assist(event);
        }
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
        out.push(self.mode_output());
        tracing::info!(english, "切换中英模式");
        out
    }

    /// 点了英文辅助属性：合成一次配置的组合键送 Router，与按快捷键走同一条路（组着英文时 Router 先把字母上屏）。
    /// 组合键被配置禁用时 Router 认不出，什么都不做。
    pub(super) fn toggle_english_assist(&mut self) -> Vec<Output> {
        let Some(combo) = self.english_assist_keys() else {
            tracing::info!("英文辅助快捷键已禁用，属性点击不切换");
            return Vec::new();
        };
        let modifiers = KeyModifiers {
            english_mode: self.english,
            ..KeyModifiers::from(combo.modifiers)
        };
        let vk = u32::from(combo.key.to_ascii_uppercase());
        self.forward_english_assist(KeyEvent::new(vk, Some(combo.key), modifiers))
            .1
    }

    /// 开关英文辅助的组合键送 Router：字母统一成小写再送（X 按着 Shift 报的是大写 keysym，Router 按配置里的小写比）。
    /// Router 吃了就刷属性（「英」↔「英+」）；回 Passthrough（比如配置刚改掉了这个组合）就照常归应用。
    fn forward_english_assist(&mut self, mut event: KeyEvent) -> (bool, Vec<Output>) {
        event.character = event.character.map(|c| c.to_ascii_lowercase());
        let (consumed, mut out) = self.forward(event);
        if consumed {
            out.push(self.mode_output());
            tracing::info!(assist = self.english_assist(), "快捷键开关英文辅助");
        }
        (consumed, out)
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
