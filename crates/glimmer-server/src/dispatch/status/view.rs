use glimmer_platform::ThemeMode;

/// 状态条一次要显示的内容。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StatusView {
    /// 英文模式（`false` 中文）。
    pub english: bool,

    /// 是否啟用大千注音。
    pub zhuyin: bool,

    /// 开着双拼 / 五笔时的方案名（`小鹤双拼` / `86 五笔`），中文格里跟在「中」后面。
    pub scheme: Option<String>,

    /// 当前模式的全角标点开着（中英各记一份配置）；关着时格子显示 `,.` 画成灰的。
    pub full_width: bool,

    /// 英文模式这时带不带补全、纠错、预测：`[general] english_candidates` 开着、且当前应用没被 `[apps]` 关掉。
    /// 中文模式下也照实填，画不画由各壳定（Windows 英文格写成「英+」）。
    pub english_assist: bool,

    /// 外观模式。
    pub theme: ThemeMode,

    /// 配置里记住的内容左上角物理像素；`None` 首次按屏幕右下角摆。
    pub anchor: Option<(i32, i32)>,
}
