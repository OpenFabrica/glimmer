//! 候选窗口与状态条的配色：配置里的配色方案与自定义颜色，画时按深浅色解析成主题。

use glimmer_platform::ColorScheme;
use glimmer_render::{Color, Palette, Theme};

use crate::dispatch::RenderSettings;

/// 配置里的配色，画的时候再按深浅色解析成 [`Palette`]（外观跟随系统时深浅色随时会变）。
#[derive(Debug, Clone, Copy)]
pub(super) struct Colors {
    /// 配色方案（`[general] color_scheme`）。
    scheme: ColorScheme,

    /// 自定义背景色；`None` 用配色方案的。
    background: Option<Color>,

    /// 自定义高亮底色；`None` 用配色方案的。
    highlight: Option<Color>,
}

impl Colors {
    /// 从画法设置里取配色；自定义颜色写错按没设处理。
    pub(super) fn from_settings(settings: &RenderSettings) -> Self {
        Self {
            scheme: settings.color_scheme,
            background: Color::from_hex(&settings.background_color),
            highlight: Color::from_hex(&settings.highlight_color),
        }
    }

    /// 按深浅色取配色方案的那一版，再叠上自定义颜色，建出主题（缺省字号）。与 macOS 壳的解析一致。
    pub(super) fn theme(self, dark: bool) -> Theme {
        let palette = match self.scheme {
            ColorScheme::Default => Palette::standard(dark),
            ColorScheme::Blue => Palette::blue(dark),
            ColorScheme::Paper => Palette::paper(dark),
            ColorScheme::Rose => Palette::rose(dark),
            ColorScheme::Mono => Palette::mono(dark),
        };
        Theme::with_palette(palette.with_custom(self.background, self.highlight))
    }
}
