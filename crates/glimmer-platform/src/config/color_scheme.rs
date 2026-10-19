//! 候选窗口配色方案：几套设计好的背景色与高亮底色，各有浅色、深色两版，跟着外观（`theme`）切换。

use serde::{Deserialize, Serialize};

/// 候选窗口配色。具体颜色在渲染器里（`glimmer_render::Palette`），这里只是配置里的名字。
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ColorScheme {
    /// 白底（深色时深灰底）、浅绿高亮，即加配色之前的样子。
    #[default]
    Default,

    /// 白底、实心蓝高亮配白字，像 macOS 原生菜单。
    Blue,

    /// 米黄纸色底、浅褐高亮，长时间看不刺眼。
    Paper,

    /// 淡粉底、樱粉高亮。
    Rose,

    /// 浅灰底、深灰高亮配白字；深色时反过来。
    Mono,
}

impl ColorScheme {
    /// 全部取值，设置界面按这个顺序列出。
    pub const ALL: [Self; 5] = [
        Self::Default,
        Self::Blue,
        Self::Paper,
        Self::Rose,
        Self::Mono,
    ];

    /// 配置文件里的写法。
    pub fn key(self) -> &'static str {
        match self {
            Self::Default => "default",
            Self::Blue => "blue",
            Self::Paper => "paper",
            Self::Rose => "rose",
            Self::Mono => "mono",
        }
    }

    /// 界面上的名字。
    pub fn label(self) -> &'static str {
        match self {
            Self::Default => "默认",
            Self::Blue => "海蓝",
            Self::Paper => "暖纸",
            Self::Rose => "樱粉",
            Self::Mono => "素灰",
        }
    }
}
