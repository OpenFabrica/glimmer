//! 颜色：sRGB 8 位 + alpha，与平台无关；到 tiny-skia / cosmic-text 的换算集中在这里。

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Color {
    pub r: u8,

    pub g: u8,

    pub b: u8,

    /// 不透明度，255 为完全不透明。
    pub a: u8,
}

impl Color {
    pub const fn rgba(r: u8, g: u8, b: u8, a: u8) -> Self {
        Self { r, g, b, a }
    }

    pub const fn rgb(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b, a: 255 }
    }

    /// 纯黑 / 纯白带不透明度，系统语义色（label / secondaryLabel…）都是这个形态。
    pub const fn gray(level: u8, alpha: u8) -> Self {
        Self {
            r: level,
            g: level,
            b: level,
            a: alpha,
        }
    }

    /// 解析 `#RRGGBB` / `#RRGGBBAA`（`#` 可省、大小写都行）；格式不对返回 `None`。配置文件里的自定义颜色用它。
    pub fn from_hex(text: &str) -> Option<Self> {
        let hex = text.trim();
        let hex = hex.strip_prefix('#').unwrap_or(hex);
        if !matches!(hex.len(), 6 | 8) || !hex.is_ascii() {
            return None;
        }
        let byte = |i: usize| u8::from_str_radix(&hex[i..i + 2], 16).ok();
        let alpha = if hex.len() == 8 { byte(6)? } else { 255 };
        Some(Self::rgba(byte(0)?, byte(2)?, byte(4)?, alpha))
    }

    /// 写成 `#RRGGBB`，不透明时不带 alpha，否则 `#RRGGBBAA`；与 [`Self::from_hex`] 互逆。
    pub fn to_hex(self) -> String {
        if self.a == 255 {
            format!("#{:02X}{:02X}{:02X}", self.r, self.g, self.b)
        } else {
            format!("#{:02X}{:02X}{:02X}{:02X}", self.r, self.g, self.b, self.a)
        }
    }

    /// 换掉不透明度。
    pub const fn with_alpha(self, a: u8) -> Self {
        Self { a, ..self }
    }

    /// 叠在不透明的 `under` 上之后看到的颜色（结果不透明）。
    pub fn over(self, under: Self) -> Self {
        let blend = |top: u8, bottom: u8| {
            let a = u32::from(self.a);
            ((u32::from(top) * a + u32::from(bottom) * (255 - a) + 127) / 255) as u8
        };
        Self::rgb(
            blend(self.r, under.r),
            blend(self.g, under.g),
            blend(self.b, under.b),
        )
    }

    /// 亮度 0–1（Rec. 709 系数直接乘 sRGB 值，不做线性化）：只用来分「深底配浅字 / 浅底配深字」，够用。
    pub fn luminance(self) -> f32 {
        (0.2126 * f32::from(self.r) + 0.7152 * f32::from(self.g) + 0.0722 * f32::from(self.b))
            / 255.0
    }

    pub(crate) fn to_skia(self) -> tiny_skia::Color {
        tiny_skia::Color::from_rgba8(self.r, self.g, self.b, self.a)
    }

    pub(crate) fn to_cosmic(self) -> cosmic_text::Color {
        cosmic_text::Color::rgba(self.r, self.g, self.b, self.a)
    }

    /// 乘上一层覆盖率（字形遮罩的像素值）后的预乘颜色。
    pub(crate) fn premultiplied(self, coverage: u8) -> tiny_skia::PremultipliedColorU8 {
        let alpha = mul_u8(self.a, coverage);
        premultiply(self.r, self.g, self.b, alpha)
    }
}

/// 8 位定点乘法：`a * b / 255`，四舍五入。
pub(crate) fn mul_u8(a: u8, b: u8) -> u8 {
    ((u32::from(a) * u32::from(b) + 127) / 255) as u8
}

/// 直通 RGBA → 预乘。
pub(crate) fn premultiply(r: u8, g: u8, b: u8, a: u8) -> tiny_skia::PremultipliedColorU8 {
    tiny_skia::PremultipliedColorU8::from_rgba(mul_u8(r, a), mul_u8(g, a), mul_u8(b, a), a)
        .unwrap_or(tiny_skia::PremultipliedColorU8::TRANSPARENT)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn premultiplies_with_coverage() {
        let c = Color::rgba(255, 0, 0, 255).premultiplied(128);
        assert_eq!((c.red(), c.green(), c.blue(), c.alpha()), (128, 0, 0, 128));
        let half = Color::gray(0, 128).premultiplied(255);
        assert_eq!((half.red(), half.alpha()), (0, 128));
        assert_eq!(mul_u8(255, 255), 255);
        assert_eq!(mul_u8(0, 255), 0);
    }

    #[test]
    fn parses_and_writes_hex() {
        assert_eq!(Color::from_hex("#007AFF"), Some(Color::rgb(0, 122, 255)));
        assert_eq!(Color::from_hex(" 007aff "), Some(Color::rgb(0, 122, 255)));
        assert_eq!(
            Color::from_hex("#B0CE7D7F"),
            Some(Color::rgba(176, 206, 125, 127))
        );
        for bad in ["", "#", "#12345", "#1234567", "#GG0000", "#日本語"] {
            assert_eq!(Color::from_hex(bad), None, "{bad}");
        }
        assert_eq!(Color::rgb(0, 122, 255).to_hex(), "#007AFF");
        assert_eq!(Color::rgba(176, 206, 125, 127).to_hex(), "#B0CE7D7F");
    }

    #[test]
    fn blends_over_opaque_background() {
        let white = Color::rgb(255, 255, 255);
        assert_eq!(Color::rgb(10, 20, 30).over(white), Color::rgb(10, 20, 30));
        assert_eq!(Color::gray(0, 0).over(white), white);
        assert_eq!(Color::gray(0, 128).over(white), Color::rgb(127, 127, 127));
    }
}
