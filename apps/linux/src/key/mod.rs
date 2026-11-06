//! 按键翻译：IBus 送来的 X11 keysym + 修饰键状态 → 协议的 [`KeyEvent`]（Windows VK 语义），
//! 以及单击 Shift 切中英（[`ShiftTap`]）与「送不送 Router」的判定（[`should_forward`]）。

pub mod keysym;
pub mod modifiers;

mod filter;
mod shift_tap;
mod virtual_key;

use glimmer_platform::protocol::KeyEvent;

pub use filter::{accepts_prediction, is_modifier, should_forward};
pub use modifiers::{is_release, key_modifiers};
pub use shift_tap::ShiftTap;
pub use virtual_key::virtual_key;

/// 把一次按下翻成协议按键；不认识的键（没有 VK）返回 `None`，由调用方放行。
/// 字符直接取 keysym 的 Unicode：X 已按 Shift / Caps Lock 算好大小写与符号，与 Windows `ToUnicode` 的结果一致。
pub fn key_event(keyval: u32, state: u32, english_mode: bool) -> Option<KeyEvent> {
    let vk = virtual_key(keyval)?;
    let modifiers = key_modifiers(state, english_mode);
    Some(KeyEvent::new(vk, keysym::to_char(keyval), modifiers))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn letter_with_caps_lock_keeps_the_keysym_case() {
        let event = key_event(u32::from('A'), modifiers::LOCK_MASK, false).unwrap();
        assert_eq!(event.virtual_key, 0x41);
        assert_eq!(event.character, Some('A'));
        assert!(event.modifiers.caps);
    }

    #[test]
    fn return_has_no_character() {
        let event = key_event(keysym::RETURN, 0, true).unwrap();
        assert_eq!(event.virtual_key, 0x0D);
        assert_eq!(event.character, None);
        assert!(event.modifiers.english_mode);
    }

    #[test]
    fn unknown_key_is_none() {
        assert!(key_event(0x1008ff13, 0, false).is_none());
    }
}
