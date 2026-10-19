//! 「候选窗口」页：外观、配色、自定义颜色、排布、渲染引擎、字体、文字大小、拼音显示位置、悬浮状态条。

use glimmer_platform::{
    CandidateRenderer, ColorScheme, LayoutMode, MAX_FONT_SIZE, MIN_FONT_SIZE, PreeditMode,
    ThemeMode,
};
use windows_reactor::*;

use crate::panel::controls::{field, page};
use crate::panel::{Message, Settings};

/// 「背景颜色」框下面的说明。
const BACKGROUND_NOTE: &str = "留空用配色方案的颜色，格式 #RRGGBB；只对微明渲染器生效。";

/// 「高亮颜色」框下面的说明：高亮底色可以带不透明度。
const HIGHLIGHT_NOTE: &str =
    "留空用配色方案的颜色，格式 #RRGGBB，也可写 #RRGGBBAA 带不透明度；只对微明渲染器生效。";

/// 枚举下拉：按 `label()` 列项，选中 `current`（找不到取 0）。
fn mode_combo<T: PartialEq + Copy>(
    all: &'static [T],
    current: T,
    label: fn(T) -> &'static str,
    callback: Callback<Option<usize>>,
) -> ComboBox {
    ComboBox::new()
        .items_source(all.iter().map(|mode| label(*mode)))
        .selected_index(all.iter().position(|mode| *mode == current).unwrap_or(0))
        .on_selection_changed(callback)
}

/// 画「候选窗口」页；颜色框敲到一半时显示草稿，不然显示配置里的值。
pub(crate) fn view(settings: &Settings, context: &mut ViewContext<Settings>) -> View {
    let g = &settings.config.general;
    let font_text = settings
        .font_query
        .clone()
        .unwrap_or_else(|| g.font.clone());
    let background_text = settings
        .background_draft
        .clone()
        .unwrap_or_else(|| g.background_color.clone());
    let highlight_text = settings
        .highlight_draft
        .clone()
        .unwrap_or_else(|| g.highlight_color.clone());
    let query = font_text.to_lowercase();
    let suggestions: Vec<String> = settings
        .families
        .iter()
        .filter(|family| family.to_lowercase().contains(&query))
        .cloned()
        .collect();
    let rows = [
        field(
            "外观",
            "",
            mode_combo(
                &ThemeMode::ALL,
                g.theme,
                ThemeMode::label,
                context.callback(Message::Theme),
            ),
        ),
        field(
            "配色",
            "浅色、深色两版跟着外观切换；只对微明渲染器生效。",
            mode_combo(
                &ColorScheme::ALL,
                g.color_scheme,
                ColorScheme::label,
                context.callback(Message::ColorScheme),
            ),
        ),
        field(
            "背景颜色",
            BACKGROUND_NOTE,
            TextBox::new()
                .width(160.0)
                .text(background_text)
                .placeholder_text("#RRGGBB")
                .on_text_changed(context.callback(Message::BackgroundColor)),
        ),
        field(
            "高亮颜色",
            HIGHLIGHT_NOTE,
            TextBox::new()
                .width(160.0)
                .text(highlight_text)
                .placeholder_text("#RRGGBB")
                .on_text_changed(context.callback(Message::HighlightColor)),
        ),
        field(
            "排布",
            "横排时只给高亮的候选显示译词。",
            mode_combo(
                &LayoutMode::ALL,
                g.layout,
                LayoutMode::label,
                context.callback(Message::Layout),
            ),
        ),
        field(
            "渲染引擎",
            "微明渲染器让候选窗口在各平台一致。",
            mode_combo(
                &CandidateRenderer::ALL,
                g.renderer,
                CandidateRenderer::label,
                context.callback(Message::Renderer),
            ),
        ),
        field(
            "字体",
            "只对微明渲染器生效；留空用系统字体，没装的字体自动回到系统字体。",
            AutoSuggestBox::new()
                .width(260.0)
                .text(font_text)
                .placeholder_text("系统字体")
                .items_source(suggestions)
                .on_text_changed(context.callback(Message::FontQuery))
                .on_suggestion_chosen(context.callback(Message::Font)),
        ),
        field(
            "文字大小",
            "候选词的字号（12 至 28，缺省 16），译词与序号按比例跟着变；只对微明渲染器生效。",
            NumberBox::new()
                .minimum(f64::from(MIN_FONT_SIZE))
                .maximum(f64::from(MAX_FONT_SIZE))
                .value(f64::from(g.font_size()))
                .on_value_changed(context.callback(Message::FontSize)),
        ),
        field(
            "拼音显示",
            "「只在候选窗口」时正在敲的拼音不显示在应用里，终端或行内拼音不正常的应用可以选它。",
            mode_combo(
                &PreeditMode::ALL,
                g.preedit,
                PreeditMode::label,
                context.callback(Message::Preedit),
            ),
        ),
        field(
            "悬浮状态条",
            "桌面上常驻、可拖动的小条：点「中 / 英」切换模式（开着双拼时还显示方案名），点「，。」切全角 / 半角标点，点齿轮打开设置。只在当前输入法是微明时显示，拖到哪下次还在哪。",
            ToggleSwitch::new()
                .is_on(settings.config.status_bar.enabled)
                .on_toggled(context.callback(Message::StatusBar)),
        ),
    ];
    page("候选窗口", StackPanel::new().spacing(16.0).children(rows))
}
