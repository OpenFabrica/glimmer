//! 英文辅助开关：独立快捷键（`[shortcut] english_assist`，缺省 Ctrl+Shift+E）随时翻转 `[general] english_candidates`，
//! 即英文模式带不带补全、纠错、下一词预测。中 / 英切换（`mode_switch`）始终只管直通与否，与这里无关。
//! 中文模式下也能按：只翻设置，下次进英文模式生效。

use glimmer_platform::KeyCombo;
use glimmer_platform::protocol::{KeyEvent, KeyModifiers};

use super::Router;
use super::key::Effect;

impl Router {
    /// 英文模式这时带不带补全、纠错、预测：配置开着且当前聚焦应用没被 `[apps]` 关掉。没有状态条的壳（Linux 属性）据此显示「英+」。
    pub fn english_assist(&self) -> bool {
        self.config.english_candidates_in(self.focused_app())
    }

    /// 开关英文辅助的组合键（跟着热加载走）；`None` 是无效或冲突，已禁用。壳据此决定把哪个带修饰键的字母送来。
    pub fn english_assist_keys(&self) -> Option<KeyCombo> {
        self.config.english_assist
    }

    /// 按键是不是配置的英文辅助组合：修饰键比物理组合（去掉 Caps / 中英模式两个状态位），字母不分大小写
    /// （按着 Shift 时有的壳报上来的是大写）。配置无效或冲突时恒为否，组合键照常交给应用。
    pub(super) fn matches_english_assist_combo(&self, event: &KeyEvent) -> bool {
        self.config.english_assist.is_some_and(|combo| {
            event.character.map(|c| c.to_ascii_lowercase()) == Some(combo.key)
                && event.modifiers.chord() == KeyModifiers::from(combo.modifiers)
        })
    }

    /// 翻转英文辅助并写回配置文件（热加载读回来是同一个值），刷新状态条；按键恒被吃掉。
    pub(super) fn toggle_english_assist(&mut self) -> Effect {
        let on = !self.config.english_candidates;
        self.config.english_candidates = on;
        tracing::info!(on, "快捷键：开关英文辅助");
        self.persist("general", "english_candidates", on);
        // 正在英文组词（只有辅助开着时才有）：照「英文组词中候选被关掉」的路，敲过的字母原样上屏，
        // 不然候选窗还挂着、下一键才上屏，看着像没关。中文组句不动，只翻设置。
        let flushed = (!self.engine.composition().is_empty() && self.engine.english_mode())
            .then(|| self.engine.take_raw());
        // 补全 / 预测换了一种算法，前面记的英文上文不再接得上
        self.engine.break_english_context();
        self.reconcile_status();
        match flushed {
            Some(text) => {
                self.engine.set_english_mode(false);
                Effect::Changed(Some(text))
            }
            None => Effect::Navigated,
        }
    }
}
