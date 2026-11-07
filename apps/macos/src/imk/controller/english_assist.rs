//! 开关英文辅助的组合键（`[shortcut] english_assist`，缺省 ⌃⇧E）的按键一侧：
//! 正在组的英文词先原样上屏，再交给 [`crate::host::Host::toggle_english_assist`] 翻开关。

use super::GlimmerInputController;
use crate::host;
use crate::imk::TextClient;

impl GlimmerInputController {
    /// 命中开关英文辅助的组合键：英文模式里正在组的词先原样上屏（不丢字母），再翻开关。
    /// 长按只切一次，重复事件照样吞掉，不漏进应用。总是返回 true（已处理）。
    pub(super) fn toggle_english_assist(&self, repeat: bool, client: TextClient<'_>) -> bool {
        if repeat {
            return true;
        }
        // Core 的英文模式只在英文辅助开着时打开：这时缓冲区里是英文词，关掉之前要先把它上屏；
        // 中文组句不动（开关只影响英文模式），上屏要找应用说话，放在借用 Host 之外
        let english_word =
            host::with(|h| h.engine.english_mode() && !h.engine.composition().is_empty())
                .unwrap_or(false);
        if english_word {
            self.commit_raw(client);
        }
        host::with(|h| h.toggle_english_assist());
        true
    }
}
