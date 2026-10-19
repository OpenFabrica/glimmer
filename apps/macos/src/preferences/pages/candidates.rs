//! 「候选窗口」页：外观、配色（方案 + 自定义背景色 / 高亮底色）、排布、渲染引擎、字体（可搜索的列表）、
//! 文字大小、拼音显示位置。

use glimmer_platform::{
    CandidateRenderer, ColorScheme, Config, DEFAULT_FONT_SIZE, GeneralConfig, LayoutMode,
    MAX_FONT_SIZE, MIN_FONT_SIZE, PreeditMode, ThemeMode,
};
use objc2::rc::Retained;
use objc2::{MainThreadMarker, MainThreadOnly};
use objc2_app_kit::{
    NSAppearanceNameAqua, NSAppearanceNameDarkAqua, NSApplication, NSColorWell, NSPopUpButton,
};
use objc2_foundation::NSArray;

use crate::candidates::{CandidateColors, available_families};
use crate::preferences::controls::{
    button, caption, color_well, note, row_popup, select, set_well_color,
};
use crate::preferences::font_picker::FontPicker;
use crate::preferences::layout::{CONTROL_X, LABEL_WIDTH, Layout, PAGE_PADDING, ROW_HEIGHT};
use crate::preferences::setting::Setting;
use crate::preferences::target::PreferencesTarget;

pub struct CandidatesPage {
    /// 外观：跟随系统 / 浅色 / 深色。
    theme: Retained<NSPopUpButton>,

    /// 配色方案。
    color_scheme: Retained<NSPopUpButton>,

    /// 自定义背景色；没设时显示配色方案当前外观下的颜色。
    background: Retained<NSColorWell>,

    /// 自定义高亮底色；同上。
    highlight: Retained<NSColorWell>,

    /// 竖排 / 横排。
    layout_mode: Retained<NSPopUpButton>,

    /// 微明渲染器 / 系统绘制。
    renderer: Retained<NSPopUpButton>,

    /// 候选窗字体：搜索框 + 列表。
    font: FontPicker,

    /// 文字大小（候选词字号，点）。
    font_size: Retained<NSPopUpButton>,

    /// 拼音显示位置。
    preedit: Retained<NSPopUpButton>,
}

impl CandidatesPage {
    pub fn build(layout: &mut Layout, mtm: MainThreadMarker, target: &PreferencesTarget) -> Self {
        let theme_titles: Vec<String> = ThemeMode::ALL
            .iter()
            .map(|t| t.label().to_owned())
            .collect();
        let theme = row_popup(layout, mtm, "外观", &theme_titles, Setting::Theme, target);
        let scheme_titles: Vec<String> = ColorScheme::ALL
            .iter()
            .map(|s| s.label().to_owned())
            .collect();
        let color_scheme = row_popup(
            layout,
            mtm,
            "配色",
            &scheme_titles,
            Setting::ColorScheme,
            target,
        );
        let background = row_color_well(layout, mtm, "背景颜色", Setting::BackgroundColor, target);
        let reset = button(mtm, "恢复配色的颜色", Setting::ResetColors, target);
        let highlight = row_color_well(layout, mtm, "高亮颜色", Setting::HighlightColor, target);
        // 恢复按钮单独一行，与取色器左对齐
        layout.place(&reset, CONTROL_X, 150.0, ROW_HEIGHT);
        layout.next_row(ROW_HEIGHT);
        note(
            layout,
            mtm,
            "每套配色有浅色、深色两版，跟着外观切换；自己挑的颜色不分深浅，字色按底色自动选。只对微明渲染器生效。",
        );
        let layout_titles: Vec<String> = LayoutMode::ALL
            .iter()
            .map(|l| l.label().to_owned())
            .collect();
        let layout_mode = row_popup(layout, mtm, "排布", &layout_titles, Setting::Layout, target);
        note(layout, mtm, "横排时只给高亮的候选显示译词。");
        let renderer_titles: Vec<String> = CandidateRenderer::ALL
            .iter()
            .map(|r| r.label().to_owned())
            .collect();
        let renderer = row_popup(
            layout,
            mtm,
            "渲染引擎",
            &renderer_titles,
            Setting::Renderer,
            target,
        );
        note(layout, mtm, "微明渲染器让候选窗口在各平台一致。");
        let font = FontPicker::build(layout, mtm, "字体", available_families(mtm));
        note(
            layout,
            mtm,
            "只对微明渲染器生效；没装的字体自动回到系统字体。",
        );
        let font_size_titles: Vec<String> = (MIN_FONT_SIZE..=MAX_FONT_SIZE)
            .map(|size| {
                if size == DEFAULT_FONT_SIZE {
                    format!("{size}（缺省）")
                } else {
                    size.to_string()
                }
            })
            .collect();
        let font_size = row_popup(
            layout,
            mtm,
            "文字大小",
            &font_size_titles,
            Setting::FontSize,
            target,
        );
        note(
            layout,
            mtm,
            "候选词的字号，译词与序号按比例跟着变；只对微明渲染器生效。",
        );
        let preedit_titles: Vec<String> = PreeditMode::ALL
            .iter()
            .map(|p| p.label().to_owned())
            .collect();
        let preedit = row_popup(
            layout,
            mtm,
            "拼音显示",
            &preedit_titles,
            Setting::Preedit,
            target,
        );
        note(
            layout,
            mtm,
            "「只在候选窗口」时正在敲的拼音不显示在应用里，终端或行内拼音显示不正常的应用可以选它。",
        );
        Self {
            theme,
            color_scheme,
            background,
            highlight,
            layout_mode,
            renderer,
            font,
            font_size,
            preedit,
        }
    }

    pub fn sync(&self, config: &Config) {
        let general = &config.general;
        select(
            &self.theme,
            ThemeMode::ALL.iter().position(|t| *t == general.theme),
        );
        select(
            &self.color_scheme,
            ColorScheme::ALL
                .iter()
                .position(|s| *s == general.color_scheme),
        );
        let palette = CandidateColors::from_config(general).palette(self.shows_dark(general));
        set_well_color(&self.background, palette.background);
        set_well_color(&self.highlight, palette.highlight);
        select(
            &self.layout_mode,
            LayoutMode::ALL.iter().position(|l| *l == general.layout),
        );
        select(
            &self.renderer,
            CandidateRenderer::ALL
                .iter()
                .position(|r| *r == general.renderer),
        );
        self.font.sync(&general.font);
        select(
            &self.font_size,
            Some((general.font_size() - MIN_FONT_SIZE) as usize),
        );
        select(
            &self.preedit,
            PreeditMode::ALL.iter().position(|p| *p == general.preedit),
        );
    }
}

/// 取色器的宽度（点）。
const WELL_WIDTH: f64 = 44.0;

impl CandidatesPage {
    /// 候选窗口现在按深色画还是浅色画：外观固定时看配置，跟随系统时看应用当前外观。取色器显示对应那版的颜色。
    fn shows_dark(&self, general: &GeneralConfig) -> bool {
        match general.theme {
            ThemeMode::Light => false,
            ThemeMode::Dark => true,
            ThemeMode::System => {
                let appearance =
                    NSApplication::sharedApplication(self.theme.mtm()).effectiveAppearance();
                // SAFETY: 只读 AppKit 导出的常量名
                let names = unsafe {
                    NSArray::from_slice(&[NSAppearanceNameAqua, NSAppearanceNameDarkAqua])
                };
                // SAFETY: 同上
                appearance
                    .bestMatchFromAppearancesWithNames(&names)
                    .is_some_and(|name| unsafe { name.isEqualToString(NSAppearanceNameDarkAqua) })
            }
        }
    }
}

/// 一行「标题 + 取色器」，取色器不占满控件列；不换行，调用方可在右边再放东西。
fn row_color_well(
    layout: &mut Layout,
    mtm: MainThreadMarker,
    title: &str,
    setting: Setting,
    target: &PreferencesTarget,
) -> Retained<NSColorWell> {
    let well = color_well(mtm, setting, target);
    let label = caption(mtm, title);
    layout.place(&label, PAGE_PADDING, LABEL_WIDTH, ROW_HEIGHT);
    layout.place(&well, CONTROL_X, WELL_WIDTH, ROW_HEIGHT);
    layout.next_row(ROW_HEIGHT);
    well
}
