//! 配色方案与高亮字色的选择。

use super::Palette;
use crate::color::Color;

const WHITE: Color = Color::rgb(255, 255, 255);

#[test]
fn default_palettes_keep_text_color_on_highlight() {
    for dark in [false, true] {
        let palette = Palette::standard(dark);
        assert_eq!(palette.highlight_text, None);
        assert_eq!(palette.on_highlight(), palette);
        // 缺省两套由 from_colors 重新配出来也一样：判定阈值不会误伤缺省的样子
        assert_eq!(
            Palette::from_colors(palette.background, palette.highlight),
            palette
        );
    }
}

#[test]
fn solid_dark_highlight_gets_white_text() {
    let palette = Palette::blue(false);
    assert_eq!(palette.highlight_text, Some(WHITE));
    let on = palette.on_highlight();
    assert_eq!(on.text, WHITE);
    assert_eq!((on.index.r, on.index.g, on.index.b), (255, 255, 255));
    assert!(on.index.a < on.gloss.a && on.gloss.a < on.text.a);
    // 背景与高亮本身不变
    assert_eq!(on.background, palette.background);
    assert_eq!(on.highlight, palette.highlight);
}

#[test]
fn bright_highlight_on_dark_background_gets_pure_white_tones() {
    for palette in [Palette::blue(true), Palette::rose(true)] {
        assert_eq!(palette.highlight_text, Some(WHITE));
        // 高亮上的词性比深色那套原来的灰浓得多
        assert!(palette.on_highlight().pos.a > Palette::dark().pos.a * 2);
    }
}

#[test]
fn light_highlight_on_dark_background_gets_dark_text() {
    let palette = Palette::mono(true);
    assert_eq!(palette.text, Palette::dark().text);
    assert_eq!(palette.highlight_text, Some(Color::gray(0, 216)));
}

#[test]
fn every_preset_picks_text_tones_by_background() {
    for dark in [false, true] {
        for palette in [
            Palette::blue(dark),
            Palette::paper(dark),
            Palette::rose(dark),
            Palette::mono(dark),
        ] {
            let expected = if dark {
                Palette::dark()
            } else {
                Palette::light()
            };
            assert_eq!(palette.text, expected.text, "dark={dark}");
            assert_eq!(palette.background.a, 255);
        }
    }
}

#[test]
fn custom_colors_override_the_scheme() {
    let base = Palette::paper(false);
    assert_eq!(base.with_custom(None, None), base);
    // 只换背景：深背景换深色那套字色，高亮沿用方案的
    let custom = base.with_custom(Some(Color::rgb(20, 24, 40)), None);
    assert_eq!(custom.background, Color::rgb(20, 24, 40));
    assert_eq!(custom.highlight, base.highlight);
    assert_eq!(custom.text, Palette::dark().text);
    // 背景带的 alpha 被去掉
    let opaque = base.with_custom(Some(Color::rgba(250, 250, 250, 10)), None);
    assert_eq!(opaque.background.a, 255);
    // 只换高亮：深色实心高亮配白字
    let custom = base.with_custom(None, Some(Color::rgb(180, 30, 30)));
    assert_eq!(custom.background, base.background);
    assert_eq!(custom.highlight_text, Some(WHITE));
}
