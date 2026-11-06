//! 「这个键送不送 Router」的判定，照 tsf `key_sink.rs` 的 `would_eat` 搬过来（翻译评审与密码框那两支除外）。
//! Router 的分派假定只收到这些键；其余键前端直接放行给应用，不碰组句。

use glimmer_platform::protocol::{KeyEvent, KeyModifiers};

/// 主键盘区字母键的 VK 范围。
const LETTERS: std::ops::RangeInclusive<u32> = 0x41..=0x5A;

/// 带 Ctrl / Alt / Win 时只有组句中的「修饰键 + 主键盘数字」送去（译词 / 删候选）；
/// 字母只有「中文模式、没在组句、按住 Shift 的大写」归应用；组句中退格 / Tab / 回车 / Esc / 空格 / 数字、
/// 翻页与方向键、可打印字符都送；没在组句时数字 / 标点也先送去转全角，Router 不转会回 Passthrough。
pub fn should_forward(event: &KeyEvent, composing: bool) -> bool {
    let modifiers = event.modifiers;
    let vk = event.virtual_key;
    if modifiers.has_command_key() {
        return composing && (0x31..=0x39).contains(&vk);
    }
    if LETTERS.contains(&vk) {
        return modifiers.caps || modifiers.english_mode || !modifiers.shift || composing;
    }
    if composing {
        return is_edit(vk) || (0x21..=0x28).contains(&vk) || event.character.is_some();
    }
    event
        .character
        .is_some_and(|c| c.is_ascii_punctuation() || c.is_ascii_digit())
}

/// 显示着英文预测时多送的键：不带任何修饰键的 Tab（接受预测）。与 Router 的判定一致，Shift+Tab 不算。
pub fn accepts_prediction(event: &KeyEvent) -> bool {
    event.virtual_key == 0x09 && event.modifiers.chord() == KeyModifiers::default()
}

/// 单独按下的修饰键（Shift / Ctrl / Alt / Super / Caps Lock 的 VK）：不收起英文预测，收起交给后面那个键。
pub fn is_modifier(vk: u32) -> bool {
    matches!(vk, 0x10..=0x12 | 0x14 | 0x5B | 0x5C | 0xA0..=0xA5)
}

/// 组句中要吃的编辑键：退格 / Tab / 回车 / Esc / 空格 / 主键盘数字。
fn is_edit(vk: u32) -> bool {
    matches!(vk, 0x08 | 0x09 | 0x0D | 0x1B | 0x20) || (0x30..=0x39).contains(&vk)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn event(vk: u32, character: Option<char>, modifiers: KeyModifiers) -> KeyEvent {
        KeyEvent::new(vk, character, modifiers)
    }

    #[test]
    fn plain_letters_are_forwarded() {
        let plain = KeyModifiers::default();
        assert!(should_forward(&event(0x41, Some('a'), plain), false));
    }

    #[test]
    fn shifted_letter_idle_in_chinese_mode_passes_through() {
        let shift = KeyModifiers {
            shift: true,
            ..KeyModifiers::default()
        };
        assert!(!should_forward(&event(0x41, Some('A'), shift), false));
        assert!(should_forward(&event(0x41, Some('A'), shift), true));
    }

    #[test]
    fn command_chords_only_digits_while_composing() {
        let ctrl = KeyModifiers {
            ctrl: true,
            ..KeyModifiers::default()
        };
        assert!(!should_forward(&event(0x43, Some('c'), ctrl), false));
        assert!(!should_forward(&event(0x43, Some('c'), ctrl), true));
        assert!(should_forward(&event(0x32, Some('2'), ctrl), true));
    }

    #[test]
    fn edit_keys_only_while_composing() {
        let plain = KeyModifiers::default();
        assert!(!should_forward(&event(0x08, None, plain), false));
        assert!(should_forward(&event(0x08, None, plain), true));
        assert!(should_forward(&event(0x22, None, plain), true));
        assert!(!should_forward(&event(0x20, Some(' '), plain), false));
    }

    #[test]
    fn only_plain_tab_accepts_a_prediction() {
        let plain = KeyModifiers::default();
        let shift = KeyModifiers {
            shift: true,
            ..KeyModifiers::default()
        };
        let english = KeyModifiers {
            english_mode: true,
            ..KeyModifiers::default()
        };
        assert!(accepts_prediction(&event(0x09, None, plain)));
        assert!(accepts_prediction(&event(0x09, None, english)));
        assert!(!accepts_prediction(&event(0x09, None, shift)));
        assert!(!accepts_prediction(&event(0x20, Some(' '), plain)));
        assert!(is_modifier(0xA0) && is_modifier(0x14) && !is_modifier(0x08));
    }

    #[test]
    fn idle_punctuation_is_forwarded_for_full_width() {
        let plain = KeyModifiers::default();
        assert!(should_forward(&event(0xBC, Some(','), plain), false));
        assert!(should_forward(&event(0x31, Some('1'), plain), false));
    }
}
