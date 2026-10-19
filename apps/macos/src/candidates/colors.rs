//! 候选窗口配色的配置：配色方案加可选的自定义背景色 / 高亮底色，到渲染器 `Palette` 的换算集中在这里。

use glimmer_platform::{ColorScheme, GeneralConfig};
use glimmer_render::{Color, Palette};

/// `[general] color_scheme` / `background_color` / `highlight_color` 解析后的样子。只对微明渲染器生效。
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct CandidateColors {
    /// 配色方案。
    pub scheme: ColorScheme,

    /// 自定义背景色；`None` 用方案的。
    pub background: Option<Color>,

    /// 自定义高亮底色；`None` 用方案的。
    pub highlight: Option<Color>,
}

impl CandidateColors {
    /// 从 `[general]` 读出；颜色写错了按没写处理（用方案的），只记一条警告。
    pub fn from_config(general: &GeneralConfig) -> Self {
        Self {
            scheme: general.color_scheme,
            background: parse("background_color", &general.background_color),
            highlight: parse("highlight_color", &general.highlight_color),
        }
    }

    /// 按浅色 / 深色配出整套颜色。
    pub fn palette(self, dark: bool) -> Palette {
        let base = match self.scheme {
            ColorScheme::Default => Palette::standard(dark),
            ColorScheme::Blue => Palette::blue(dark),
            ColorScheme::Paper => Palette::paper(dark),
            ColorScheme::Rose => Palette::rose(dark),
            ColorScheme::Mono => Palette::mono(dark),
        };
        base.with_custom(self.background, self.highlight)
    }
}

/// 解析一个配置里的颜色；空串是「没设」，不报警。
fn parse(key: &str, text: &str) -> Option<Color> {
    if text.trim().is_empty() {
        return None;
    }
    let color = Color::from_hex(text);
    if color.is_none() {
        tracing::warn!(key, text, "颜色格式不对（要 #RRGGBB），按没设处理");
    }
    color
}
