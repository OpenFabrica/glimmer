//! 主题：字体、颜色、间距。所有可视参数都在这里，单位是点；将来从 TOML 读。
//!
//! 视觉层级（产品决定）：候选词最深，译文稍浅，词性最浅，序号弱化。数值对齐 macOS 壳的 AppKit 实现。

mod font_spec;
mod palette;

pub use font_spec::FontSpec;
pub use palette::Palette;

/// 缺省主题里候选词的字号（点）；[`Theme::with_text_size`] 以它为 1 倍缩放。
pub const DEFAULT_TEXT_SIZE: f32 = 16.0;

#[derive(Debug, Clone, PartialEq)]
pub struct Theme {
    /// 候选词字体。
    pub text_font: FontSpec,

    /// 译文与词性字体。
    pub annotation_font: FontSpec,

    /// 序号字体。
    pub index_font: FontSpec,

    /// 配色。
    pub colors: Palette,

    /// 窗口内边距。
    pub padding: f32,

    /// 行内上下留白。
    pub row_padding: f32,

    /// 序号与候选词、候选词与译文之间的间距。
    pub column_gap: f32,

    /// 窗口与高亮条的圆角。
    pub corner_radius: f32,

    /// 最多显示几行。
    pub max_rows: usize,

    /// 文字抗锯齿覆盖率的 gamma：小于 1 笔画显粗。CoreText 对文字有一层类似的加深，深色背景上尤其明显，
    /// 线性混合出来的字会偏细；这个值按真机截图并排调。
    pub text_gamma: f32,
}

impl Theme {
    /// 浅色，对齐 macOS 系统外观。
    pub fn light() -> Self {
        Self::with_palette(Palette::light())
    }

    /// 深色，对齐 macOS 系统外观。
    pub fn dark() -> Self {
        Self::with_palette(Palette::dark())
    }

    /// 用一套配色（配色方案或自定义颜色配出来的）建主题，字体与间距是缺省的。
    /// 文字 gamma 按背景深浅取：深底上 CoreText 的加深更明显，要更小的值才对得上。
    pub fn with_palette(colors: Palette) -> Self {
        let text_gamma = if colors.background.luminance() < 0.5 {
            0.75
        } else {
            0.85
        };
        Self {
            // 行高取 AppKit 系统字体在这几个字号下 NSAttributedString.size() 的高度
            text_font: FontSpec::new(DEFAULT_TEXT_SIZE, 19.0),
            annotation_font: FontSpec::new(12.0, 15.0),
            index_font: FontSpec::new(11.0, 14.0),
            colors,
            padding: 8.0,
            row_padding: 4.0,
            column_gap: 8.0,
            corner_radius: 8.0,
            max_rows: 9,
            text_gamma,
        }
    }

    /// 画高亮那一项用的主题：只换配色，见 [`Palette::on_highlight`]。
    pub(crate) fn on_highlight(&self) -> Self {
        Self {
            colors: self.colors.on_highlight(),
            ..self.clone()
        }
    }

    /// 候选词字号换成 `size` 点（配置 `[general] font_size`），译文与序号字号、三者行高按同一比例缩放。
    /// 行高向上取整：每行与窗口的高度都按行高排，取小了字形会被高亮条和窗口边裁掉。
    /// 间距、圆角、云朵图标不跟着变：它们是窗口的骨架，字大了留白比例略紧但不裁字。
    /// `size` 由调用方夹到合法范围；不是正的有限数时原样返回。
    pub fn with_text_size(mut self, size: f32) -> Self {
        if !size.is_finite() || size <= 0.0 {
            return self;
        }
        let factor = size / self.text_font.size;
        for font in [
            &mut self.text_font,
            &mut self.annotation_font,
            &mut self.index_font,
        ] {
            font.size *= factor;
            font.line_height = (font.line_height * factor).ceil();
        }
        self
    }
}

#[cfg(test)]
mod tests;
