//! 设计好的几套配色方案，各有浅色、深色两版；只定背景与高亮底色，字色由 [`Palette::from_colors`] 按深浅配。

use super::Palette;
use crate::color::Color;

impl Palette {
    /// 缺省配色：白底浅绿高亮 / 深灰底墨绿高亮。
    pub const fn standard(dark: bool) -> Self {
        if dark { Self::dark() } else { Self::light() }
    }

    /// 海蓝：实心系统蓝高亮配白字，像 macOS 原生菜单。
    pub fn blue(dark: bool) -> Self {
        if dark {
            Self::from_colors(Color::rgb(30, 30, 30), Color::rgb(10, 132, 255))
        } else {
            Self::from_colors(Color::rgb(255, 255, 255), Color::rgb(0, 122, 255))
        }
    }

    /// 暖纸：米黄纸色底、浅褐高亮；深色是咖啡底、深褐高亮。
    pub fn paper(dark: bool) -> Self {
        if dark {
            Self::from_colors(Color::rgb(38, 34, 29), Color::rgb(96, 78, 50))
        } else {
            Self::from_colors(Color::rgb(250, 246, 236), Color::rgb(232, 218, 186))
        }
    }

    /// 樱粉：淡粉底、樱粉高亮；深色是暗紫红底、玫红高亮。
    pub fn rose(dark: bool) -> Self {
        if dark {
            Self::from_colors(Color::rgb(42, 30, 35), Color::rgb(128, 60, 86))
        } else {
            Self::from_colors(Color::rgb(255, 247, 249), Color::rgb(248, 200, 214))
        }
    }

    /// 素灰：浅灰底、深灰高亮配白字；深色反过来，深灰底、浅灰高亮配深字。
    pub fn mono(dark: bool) -> Self {
        if dark {
            Self::from_colors(Color::rgb(30, 30, 30), Color::rgb(228, 228, 228))
        } else {
            Self::from_colors(Color::rgb(246, 246, 246), Color::rgb(64, 64, 64))
        }
    }
}
