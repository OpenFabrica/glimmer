//! 一套配色。缺省两套取自 macOS 系统语义色在 sRGB 下的实测值（label 0.847、secondaryLabel 0.498…）；
//! 其余配色方案在 `preset.rs`，用户自定义的背景 / 高亮色经 [`Palette::with_custom`] 叠上来。

mod preset;

use crate::color::Color;

/// 高亮底色叠到背景上后亮度低于它、而正文是深字时，高亮上改用白字。
const DARK_HIGHLIGHT: f32 = 0.5;

/// 高亮底色叠到背景上后亮度高于它、而正文是浅字时，高亮上改用深字。
const LIGHT_HIGHLIGHT: f32 = 0.6;

/// 深底上高亮比背景亮出这么多（实心蓝、玫红）时，高亮上改用纯白一系：深色那套序号、词性的灰太淡，压不住有色底。
/// 缺省深色的墨绿高亮只亮 0.135，不受影响。
const BRIGHT_HIGHLIGHT_STEP: f32 = 0.15;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Palette {
    /// 候选词。
    pub text: Color,

    /// 译文。
    pub gloss: Color,

    /// 词性，比译文更浅。
    pub pos: Color,

    /// 生词译文：比普通译文醒目，看熟了就回到译文色。
    pub fresh: Color,

    /// 序号。
    pub index: Color,

    /// 云联想的云朵与文字：比译文醒目一点，但不抢候选词。
    pub cloud: Color,

    /// 窗口背景。
    pub background: Color,

    /// 当前候选的高亮底色。
    pub highlight: Color,

    /// 高亮条上的字色；`None` 与别的行一样。高亮底色和正文字色对比不够（蓝底黑字）时才有值，
    /// 序号、译文等按它降低不透明度，见 [`Palette::on_highlight`]。
    pub highlight_text: Option<Color>,
}

impl Palette {
    pub const fn light() -> Self {
        Self {
            text: Color::gray(0, 216),
            gloss: Color::gray(0, 127),
            pos: Color::gray(0, 66),
            fresh: Color::rgb(255, 141, 40),
            index: Color::gray(0, 66),
            cloud: Color::rgb(0, 195, 208),
            background: Color::rgb(255, 255, 255),
            highlight: Color::rgba(176, 206, 125, 127),
            highlight_text: None,
        }
    }

    pub const fn dark() -> Self {
        Self {
            text: Color::gray(255, 216),
            gloss: Color::gray(255, 140),
            pos: Color::gray(255, 63),
            fresh: Color::rgb(255, 146, 48),
            index: Color::gray(255, 63),
            cloud: Color::rgb(0, 210, 224),
            background: Color::rgb(30, 30, 30),
            highlight: Color::rgba(36, 76, 36, 255),
            highlight_text: None,
        }
    }

    /// 由背景色与高亮底色配出一整套：背景浅用浅色那套字色、背景深用深色那套，高亮上的字色按对比度选。
    /// 背景一律当不透明用：半透明的候选窗压在别的文字上没法读。
    pub fn from_colors(background: Color, highlight: Color) -> Self {
        let background = background.with_alpha(255);
        let base = if background.luminance() < 0.5 {
            Self::dark()
        } else {
            Self::light()
        };
        let mut palette = Self {
            background,
            highlight,
            ..base
        };
        palette.highlight_text = palette.contrasting_highlight_text();
        palette
    }

    /// 叠上用户自定义的背景色 / 高亮底色（`None` 保持配色方案的）；两个都没给时原样返回。
    pub fn with_custom(self, background: Option<Color>, highlight: Option<Color>) -> Self {
        if background.is_none() && highlight.is_none() {
            return self;
        }
        Self::from_colors(
            background.unwrap_or(self.background),
            highlight.unwrap_or(self.highlight),
        )
    }

    /// 画高亮那一项用的配色：高亮上要换字色时，正文、序号、译文、云朵全换成同一色的不同深浅，
    /// 否则原样返回（缺省配色的高亮够浅，字色不用变）。
    pub fn on_highlight(&self) -> Self {
        let Some(color) = self.highlight_text else {
            return *self;
        };
        // 不透明度按缺省浅色配色里各级相对正文的比例：译文约 0.6、序号与词性约 0.3，这里略提一点，有色底上更好认
        let faded = |ratio: f32| color.with_alpha((f32::from(color.a) * ratio).round() as u8);
        Self {
            text: color,
            gloss: faded(0.75),
            pos: faded(0.55),
            fresh: color,
            index: faded(0.6),
            cloud: color,
            ..*self
        }
    }

    /// 高亮上该用的字色：正文是深字而高亮叠出来很深 → 白字；正文是浅字而高亮叠出来很浅 → 深字；
    /// 正文是浅字而高亮比背景亮出一截 → 纯白一系；其余不换。
    fn contrasting_highlight_text(&self) -> Option<Color> {
        let background = self.background.luminance();
        let shown = self.highlight.over(self.background).luminance();
        let dark_text = self.text.over(self.background).luminance() < 0.5;
        if dark_text && shown < DARK_HIGHLIGHT {
            Some(Color::rgb(255, 255, 255))
        } else if !dark_text && shown > LIGHT_HIGHLIGHT {
            Some(Color::gray(0, 216))
        } else if !dark_text && shown - background > BRIGHT_HIGHLIGHT_STEP {
            Some(Color::rgb(255, 255, 255))
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests;
