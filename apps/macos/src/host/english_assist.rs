//! 「英文辅助」开关：英文模式带不带补全、纠错、下一词预测（`[general] english_candidates`）。
//! 快捷键（`[shortcut] english_assist`）与菜单项都走 [`Host::toggle_english_assist`]：只写配置文件，生效靠 `apply_config`。

use super::Host;

impl Host {
    /// 翻转英文辅助并写回配置文件，返回翻转后的值；写不成时什么都不动，返回原值。
    /// 中文模式下也能切（只改设置，下次进英文模式生效）。
    ///
    /// 正在组句的英文词不在这里动：快捷键那边借用 Host 之前已经把它原样上屏；菜单那边够不着应用
    /// （找应用说话要放在借用之外），留给下一键走 `handle_text` 里「候选被关掉」的路径原样上屏，字母不丢。
    pub fn toggle_english_assist(&mut self) -> bool {
        let on = !self.settings.config().general.english_candidates;
        if !self.settings.set_bool("general", "english_candidates", on) {
            return self.english_candidates;
        }
        tracing::info!(on, "开关英文辅助");
        // 挂着的预测和攒下的英文上文都属于切换之前：关了不能再留着预测，开了也从干净的上文起
        self.dismiss_next_word("开关英文辅助");
        self.engine.break_english_context();
        self.apply_config(false);
        on
    }

    /// 菜单栏「英 / 英+」按当前应用里英文辅助实际开没开：全局开关开着，且应用不在 `[apps] english_candidates_off` 里。
    pub fn sync_english_assist_indicator(&mut self) {
        let on = self.english_candidates_in(self.engine.application());
        self.indicator.set_english_assist(on);
    }
}
