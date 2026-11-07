//! 「翻译选中文字」与「开关英文辅助」两个快捷键登记成 TSF **保留键**（preserved key）。带 Alt 的组合是系统键，不经击键 sink
//! （真机：Ctrl+Alt+T 在 `OnTestKeyDown` 里从没出现过）；保留键由 TSF 在应用之前匹配、回调 `OnPreservedKey`，
//! UWP 里也一样。组合来自 `[shortcut] translate_selection` / `english_assist`，激活时读一次配置（AppContainer 读不到用户目录时用缺省
//! Ctrl+Alt+T / Ctrl+Shift+E）；改了配置要切走再切回输入法才重新登记。命中后都当普通按键送给 Server，由 Router 按组合认。

use windows::Win32::UI::TextServices::{
    ITfKeystrokeMgr, TF_MOD_ALT, TF_MOD_CONTROL, TF_MOD_SHIFT, TF_PRESERVEDKEY,
};
use windows::core::{GUID, Result};

use glimmer_platform::protocol::{KeyEvent, KeyModifiers};
use glimmer_platform::{Config, KeyCombo, ShortcutConfig};

use crate::com::log::log;

/// 「翻译选中文字」保留键的标识，`OnPreservedKey` 按它认。
pub(crate) const GUID_TRANSLATE: GUID = GUID::from_u128(0x6ad46a6d_4275_4572_95cd_705d1248dc4c);

/// 「开关英文辅助」保留键的标识。
pub(crate) const GUID_ENGLISH_ASSIST: GUID =
    GUID::from_u128(0x3f8c2b7e_9d41_4a6e_b5c0_8e27d1f4a903);

/// msctf.h 的 `TF_MOD_LWIN`（windows crate 没导出）。
const TF_MOD_LWIN: u32 = 0x08;

/// 读 `%APPDATA%\Glimmer\config.toml` 的 `[shortcut]`；读不到 / 解析失败用缺省。
pub(crate) fn load_shortcuts() -> ShortcutConfig {
    let Some(path) = glimmer_platform::dirs::config_path() else {
        return ShortcutConfig::default();
    };
    match Config::load(&path) {
        Ok(config) => config.shortcut,
        Err(error) => {
            log(&format!("读配置取快捷键失败，用缺省: {error}"));
            ShortcutConfig::default()
        }
    }
}

fn preserved_key(combo: KeyCombo) -> TF_PRESERVEDKEY {
    let m = combo.modifiers;
    let mut modifiers = 0;
    if m.control {
        modifiers |= TF_MOD_CONTROL;
    }
    if m.option {
        modifiers |= TF_MOD_ALT;
    }
    if m.shift {
        modifiers |= TF_MOD_SHIFT;
    }
    if m.command {
        modifiers |= TF_MOD_LWIN;
    }
    TF_PRESERVEDKEY {
        uVKey: combo.key.to_ascii_uppercase() as u32,
        uModifiers: modifiers,
    }
}

/// 登记一个保留键；`description` 是 TSF 里给人看的名字。
pub(crate) fn register(
    keystroke: &ITfKeystrokeMgr,
    tid: u32,
    guid: &GUID,
    combo: KeyCombo,
    description: &str,
) -> Result<()> {
    let key = preserved_key(combo);
    let description: Vec<u16> = description.encode_utf16().collect();
    unsafe { keystroke.PreserveKey(tid, guid, &key, &description) }
}

/// 撤掉一个保留键（停用时）；失败无所谓。
pub(crate) fn unregister(keystroke: &ITfKeystrokeMgr, guid: &GUID, combo: KeyCombo) {
    let key = preserved_key(combo);
    let _ = unsafe { keystroke.UnpreserveKey(guid, &key) };
}

/// 保留键命中时喂给 Server 的按键：Router 按字符 + 物理修饰键与配置比对。
pub(crate) fn key_event(combo: KeyCombo, english_mode: bool) -> KeyEvent {
    let m = combo.modifiers;
    KeyEvent::new(
        combo.key.to_ascii_uppercase() as u32,
        Some(combo.key),
        KeyModifiers {
            ctrl: m.control,
            shift: m.shift,
            alt: m.option,
            win: m.command,
            caps: false,
            english_mode,
        },
    )
}
