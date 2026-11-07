//! 前缀模式与平台快捷键配置。

use glimmer_core::ModeKeys;
use serde::{Deserialize, Serialize};

use super::ModeSwitch;
use super::key_combo::KeyCombo;
use super::modifiers::Modifiers;

/// 配置文件 `[shortcut]` 分节：前缀模式键（Core 的 [`ModeKeys`]）加壳层的修饰键组合。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct ShortcutConfig {
    /// 表达式 / 问字模式键，键名与以前一样直接在分节下（`expression` / `question`）。
    #[serde(flatten)]
    pub mode: ModeKeys,

    /// 数字键配这些修饰键：上屏候选的第一个译词。
    pub translation: Modifiers,

    /// 数字键配这些修饰键：上屏候选的第二个译词（候选右侧有两个译词时）。
    pub translation_second: Modifiers,

    /// 把应用里选中的文字译成学习语言（需要云服务开着）。
    pub translate_selection: KeyCombo,

    /// macOS 中英文切换：单击 Shift，或修饰键加字母，不能与翻译选中文字相同。
    pub mode_switch: ModeSwitch,

    /// 开关「英文模式带补全、纠错、预测」：各壳收到就翻转 `[general] english_candidates` 并写回配置文件。
    pub english_assist: KeyCombo,

    /// 数字键配这些修饰键：删掉候选（用户词整个删掉，词库词清掉对它的学习）。
    pub delete_candidate: Modifiers,
}

impl Default for ShortcutConfig {
    fn default() -> Self {
        // Windows 上 Alt+数字被系统当菜单快捷键截走（TSF 收不到），译词键缺省用 Ctrl；macOS 用 Option。
        #[cfg(windows)]
        let (translation, translation_second) = (Modifiers::CONTROL, Modifiers::SHIFT_CONTROL);
        #[cfg(not(windows))]
        let (translation, translation_second) = (Modifiers::OPTION, Modifiers::SHIFT_OPTION);
        Self {
            mode: ModeKeys::default(),
            translation,
            translation_second,
            translate_selection: KeyCombo::TRANSLATE_DEFAULT,
            mode_switch: ModeSwitch::default(),
            english_assist: KeyCombo::ENGLISH_ASSIST_DEFAULT,
            delete_candidate: Modifiers::SHIFT,
        }
    }
}

impl ShortcutConfig {
    /// 支持单击 Shift 或修饰键加字母，避免抢占候选序号；无效或与翻译键冲突时禁用。
    /// 手工配置与设置窗口使用同一规则，不能让冲突键触发错误动作。
    pub fn mode_switch_keys(&self) -> Option<ModeSwitch> {
        self.mode_switch
            .combo()
            .is_none_or(|combo| {
                combo.key.is_ascii_alphabetic()
                    && !combo.modifiers.is_empty()
                    && combo != self.translate_selection
            })
            .then_some(self.mode_switch)
    }

    /// 开关英文辅助的组合键；不是修饰键加字母、或与翻译键 / 中英切换组合键相同时禁用。
    /// 冲突时 mode_switch 优先（它不检查本键），这里让路，保证一个组合键只触发一个动作。
    pub fn english_assist_keys(&self) -> Option<KeyCombo> {
        let combo = self.english_assist;
        (combo.key.is_ascii_alphabetic()
            && !combo.modifiers.is_empty()
            && combo != self.translate_selection
            && self.mode_switch.combo() != Some(combo))
        .then_some(combo)
    }

    /// 删候选的修饰键；为空或与任一组译词键撞了就退回缺省。
    pub fn delete_keys(&self) -> Modifiers {
        let (first, second) = self.translation_keys();
        if self.delete_candidate.is_empty()
            || self.delete_candidate == first
            || self.delete_candidate == second
        {
            Self::default().delete_candidate
        } else {
            self.delete_candidate
        }
    }

    /// 两组译词修饰键；两组相同或有一组为空时整个退回缺省，不做一半。
    pub fn translation_keys(&self) -> (Modifiers, Modifiers) {
        if self.translation == self.translation_second
            || self.translation.is_empty()
            || self.translation_second.is_empty()
        {
            let default = Self::default();
            (default.translation, default.translation_second)
        } else {
            (self.translation, self.translation_second)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mode_switch_accepts_a_single_shift() {
        let config: ShortcutConfig = toml::from_str("mode_switch = \"shift\"").unwrap();
        assert_eq!(config.mode_switch.key_string(), "shift");
        assert!(config.mode_switch_keys().is_some());
    }

    #[test]
    fn old_files_without_modifier_keys_still_parse_and_get_defaults() {
        // 缺省分平台（Windows 是 Ctrl 系，其余 Option 系），断言跟着平台的 Default 走
        let default = ShortcutConfig::default();
        let parsed: ShortcutConfig = toml::from_str("expression = \"i\"\n").unwrap();
        assert_eq!(parsed.mode.expression, 'i');
        assert_eq!(
            parsed.translation_keys(),
            (default.translation, default.translation_second)
        );
        let same: ShortcutConfig =
            toml::from_str("translation = \"option\"\ntranslation_second = \"option\"\n").unwrap();
        assert_eq!(
            same.translation_keys(),
            (default.translation, default.translation_second)
        );
        let swapped: ShortcutConfig =
            toml::from_str("translation = \"control+option\"\ntranslation_second = \"option\"\n")
                .unwrap();
        assert_eq!(swapped.translation_keys().1, Modifiers::OPTION);
    }

    #[test]
    fn delete_keys_fall_back_when_clashing_with_translation_keys() {
        let default = ShortcutConfig::default();
        let parsed: ShortcutConfig = toml::from_str("").unwrap();
        assert_eq!(parsed.delete_keys(), Modifiers::SHIFT);
        // 与本平台缺省的译词键撞上才算冲突
        let clash: ShortcutConfig = toml::from_str(&format!(
            "delete_candidate = \"{}\"\n",
            default.translation.key()
        ))
        .unwrap();
        assert_eq!(clash.delete_keys(), Modifiers::SHIFT);
        let free = [Modifiers::OPTION, Modifiers::CONTROL]
            .into_iter()
            .find(|m| *m != default.translation && *m != default.translation_second)
            .unwrap();
        let custom: ShortcutConfig =
            toml::from_str(&format!("delete_candidate = \"{}\"\n", free.key())).unwrap();
        assert_eq!(custom.delete_keys(), free);
    }

    #[test]
    fn mode_switch_defaults_round_trip_and_conflicts() {
        let mut parsed: ShortcutConfig = toml::from_str("").unwrap();
        assert_eq!(parsed.mode_switch_keys(), Some(ModeSwitch::Shift));
        parsed.mode_switch = "control+shift+e".parse().unwrap();
        let saved = toml::to_string(&parsed).unwrap();
        assert_eq!(toml::from_str::<ShortcutConfig>(&saved).unwrap(), parsed);
        parsed.mode_switch = ModeSwitch::Combo(parsed.translate_selection);
        assert_eq!(parsed.mode_switch_keys(), None);
        parsed.mode_switch = "option+1".parse().unwrap();
        assert_eq!(parsed.mode_switch_keys(), None);
    }

    #[test]
    fn english_assist_defaults_when_missing_and_parses() {
        let old: ShortcutConfig = toml::from_str("expression = \"v\"\n").unwrap();
        assert_eq!(old.english_assist, KeyCombo::ENGLISH_ASSIST_DEFAULT);
        assert_eq!(
            old.english_assist_keys(),
            Some(KeyCombo::ENGLISH_ASSIST_DEFAULT)
        );
        assert_eq!(old.english_assist.key_string(), "control+shift+e");
        // Windows 键名读回来等价
        let custom: ShortcutConfig = toml::from_str("english_assist = \"ctrl+alt+g\"\n").unwrap();
        assert_eq!(
            custom.english_assist_keys().unwrap().key_string(),
            "control+option+g"
        );
    }

    #[test]
    fn english_assist_gives_way_on_conflicts() {
        let mut config = ShortcutConfig {
            english_assist: KeyCombo::TRANSLATE_DEFAULT,
            ..ShortcutConfig::default()
        };
        assert_eq!(config.english_assist_keys(), None);
        // 中英切换是组合键且与本键相同：本键让路，中英切换照常
        config.english_assist = KeyCombo::ENGLISH_ASSIST_DEFAULT;
        config.mode_switch = ModeSwitch::Combo(KeyCombo::ENGLISH_ASSIST_DEFAULT);
        assert_eq!(config.english_assist_keys(), None);
        assert_eq!(config.mode_switch_keys(), Some(config.mode_switch));
        // 只有字母没修饰键、或键是数字，都禁用
        config.mode_switch = ModeSwitch::Shift;
        config.english_assist = KeyCombo {
            modifiers: Modifiers::default(),
            key: 'e',
        };
        assert_eq!(config.english_assist_keys(), None);
        config.english_assist = "control+shift+9".parse().unwrap();
        assert_eq!(config.english_assist_keys(), None);
    }
}
