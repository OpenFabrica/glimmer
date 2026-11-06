//! 英文下一词预测的按键一侧：预测显示着时不带修饰键的 Tab 交给 Core 接受，其他键先收起预测再照常处理。
//! 预测出不出、出什么、接受后记什么都在 Core；这里只把 Tab 送过去、把返回的文字插进应用。

use glimmer_platform::Modifiers;

use super::GlimmerInputController;
use crate::host::{self, NextWordPhase};
use crate::imk::TextClient;

/// Tab 的键码（ANSI 布局的物理键）。
const TAB_KEY: u16 = 48;

impl GlimmerInputController {
    /// 按键分发前先过这一道。没有预测时什么都不做。预测在眼前且按的是不带修饰键的 Tab：Core 接了就插入「词 + 空格」、
    /// 接着取下一个，返回 `Some(true)`（吞掉 Tab）。其他键、或 Core 没接的 Tab（过期了、上文变了）：收起预测，
    /// 返回 `None`，按键照现在的路走（Tab 走 `handle_command`，照旧作废英文上文）。
    pub(super) fn intercept_next_word(
        &self,
        key: u16,
        pressed: Modifiers,
        client: TextClient<'_>,
    ) -> Option<bool> {
        // 顺带记下这一键是不是按在预测上：是的话键后的空格不再取预测（见 handle_text）
        let phase = host::with(|h| h.begin_next_word_key()).unwrap_or_default();
        if phase == NextWordPhase::Idle {
            return None;
        }
        if is_accept_key(key, pressed) && phase.accepts_tab() {
            let text = host::with(|h| h.engine.accept_english_prediction(0)).flatten();
            if let Some(text) = text {
                tracing::debug!(%text, "接受下一词预测");
                // 空格 Core 已经记成直通了，这里不再 note_passthrough
                client.insert_text(&text);
                host::with(|h| h.request_next_word());
                return Some(true);
            }
            tracing::debug!("Core 没接这个 Tab（预测已过期或上文变了），Tab 照常交给应用");
        }
        host::with(|h| h.dismiss_next_word("按键"));
        None
    }
}

/// 接受预测的键：只认不带修饰键的 Tab；⇧Tab、⌃Tab、⌥Tab 照常交给应用。
fn is_accept_key(key: u16, pressed: Modifiers) -> bool {
    key == TAB_KEY && pressed.is_empty()
}

#[cfg(test)]
mod tests {
    use super::is_accept_key;
    use glimmer_platform::Modifiers;

    #[test]
    fn only_bare_tab_accepts_a_prediction() {
        assert!(is_accept_key(48, Modifiers::default()));
        let shift = Modifiers {
            shift: true,
            ..Modifiers::default()
        };
        assert!(!is_accept_key(48, shift));
        let control = Modifiers {
            control: true,
            ..Modifiers::default()
        };
        assert!(!is_accept_key(48, control));
        // 空格、回车、数字 1 都不是
        for key in [49, 36, 18] {
            assert!(!is_accept_key(key, Modifiers::default()));
        }
    }
}
