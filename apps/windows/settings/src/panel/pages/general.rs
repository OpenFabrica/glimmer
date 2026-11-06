//! 「通用」页：学习语言与译词读音、每页候选数、拼音方案 / 五笔、英文模式候选。

use glimmer_platform::{MAX_PAGE_SIZE, PunctuationMode};
use windows_reactor::*;

use crate::panel::controls::{field, index_of, page};
use crate::panel::{Message, Settings};

/// 学习语言：界面名 + 配置写法。
pub(crate) const LANGUAGES: [(&str, &str); 4] = [
    ("英语", "en"),
    ("日语", "ja"),
    ("西班牙语", "es"),
    ("不显示译文", "off"),
];

/// 拼音方案：界面名 + 配置写法（`[general] scheme`）。
/// 首项起逐项对齐 `glimmer_platform::Scheme::ALL`（顺序与文案由底部单测守住）。
/// `Scheme::label` / `key` 不是 const fn，没法直接由 `ALL` 生成，所以抄一份再用单测守。
pub(crate) const SCHEME: [(&str, &str); 9] = [
    ("全拼", "pinyin"),
    ("小鹤双拼", "xiaohe"),
    ("自然码", "ziranma"),
    ("微软双拼", "microsoft"),
    ("搜狗双拼", "sogou"),
    ("小浪双拼", "xiaolang"),
    ("智能ABC", "abc"),
    ("大千注音", "zhuyin"),
    ("关（只用五笔）", "none"),
];

/// 五笔：界面名 + 配置写法（空串关）。与拼音方案是两条独立的轴，同时开着就是混输。
/// 首项之后逐项对齐 `glimmer_core::WubiVariant::ALL`（顺序与文案由底部单测守住）。
pub(crate) const WUBI: [(&str, &str); 4] = [
    ("关", ""),
    ("86 五笔", "86"),
    ("98 五笔", "98"),
    ("新世纪五笔", "xsj"),
];

fn string_combo(
    options: &'static [(&str, &str)],
    current: &str,
    callback: Callback<Option<usize>>,
) -> ComboBox {
    ComboBox::new()
        .items_source(options.iter().map(|(label, _)| *label))
        .selected_index(index_of(options, current))
        .on_selection_changed(callback)
}

pub(crate) fn view(settings: &Settings, context: &mut ViewContext<Settings>) -> View {
    let g = &settings.config.general;
    let wubi = &settings.config.wubi;
    // 五笔的两个子项只在五笔开着时有意义；拼音方案不再受它影响（两边都开就是混输）
    let wubi_on = g.wubi().is_some();
    let english_off = !settings.config.apps.english_candidates_off.is_empty();
    let rows = [
        field(
            "学习语言",
            "候选词右侧显示哪种语言的译词，只列出装了释义表的语言；「不显示译文」同时关掉生词标记与释义兜底。",
            string_combo(
                &LANGUAGES,
                &g.learning_language,
                context.callback(Message::LearningLanguage),
            ),
        ),
        field(
            "显示译文音标",
            "英文译词后面的美式音标（develop (dɪˈveləp)），日语译词的假名注音也随它；「不显示译文」时不可用。",
            ToggleSwitch::new()
                .is_on(g.translation_reading)
                .is_enabled(!g.learning_language_off())
                .on_toggled(context.callback(Message::TranslationReading)),
        ),
        field(
            "每页候选数",
            "",
            NumberBox::new()
                .minimum(1.0)
                .maximum(MAX_PAGE_SIZE as f64)
                .value(g.page_size as f64)
                .on_value_changed(context.callback(Message::PageSize)),
        ),
        field(
            "拼音方案",
            "全拼、六套双拼、大千注音，或关（只用五笔）。开双拼后 v、u、i 是音节键，表达式与问字模式改用 Shift+V、Shift+U 进；微软、搜狗方案的 ; 键是 ing。注音的容错设定（ㄢㄤ、ㄣㄥ 不分等）在「模糊音」页开。",
            string_combo(&SCHEME, g.scheme().key(), context.callback(Message::Scheme)),
        ),
        field(
            "五笔",
            "a–y 是编码键，最长四码；z 开头是拼音反查。与拼音方案同时开着就是混输：五笔候选在前，打不出的字直接打拼音，第 5 个字母起只剩拼音；混输时 z 反查关掉（z 是声母）。单用五笔请把拼音方案选「关（只用五笔）」。",
            string_combo(&WUBI, &g.wubi, context.callback(Message::Wubi)),
        ),
        field(
            "整句输入",
            "连着打编码不按空格，超过四码后首选是整句，空格上屏。开着时不再四码自动上屏；只打五笔时有效，混输时不生效。",
            ToggleSwitch::new()
                .is_on(wubi.sentence)
                .is_enabled(wubi_on)
                .on_toggled(context.callback(Message::WubiSentence)),
        ),
        field(
            "四码自动上屏",
            "敲满四码且有全码命中时首选直接上屏，不用再按空格；混输时不生效（niha 也可能是拼音）。",
            // 整句开着时四码不自动上屏，这个开关落盘了也不起作用，直接置灰
            ToggleSwitch::new()
                .is_on(wubi.auto_select)
                .is_enabled(wubi_on && !wubi.sentence)
                .on_toggled(context.callback(Message::WubiAutoSelect)),
        ),
        field(
            "显示编码提示",
            "逐键提示的候选右侧显示完整编码。",
            ToggleSwitch::new()
                .is_on(wubi.hint)
                .is_enabled(wubi_on)
                .on_toggled(context.callback(Message::WubiHint)),
        ),
        field(
            "繁体输出",
            "打字时将候选词转换为繁体中文。",
            ToggleSwitch::new()
                .is_on(g.traditional)
                .on_toggled(context.callback(Message::Traditional)),
        ),
        field(
            "中文模式标点转全角",
            "没在打拼音时敲 , . ? ! 等出「，。？！」，数字后面的点保持半角；悬浮状态条的「，。」格也能切，切的是当前模式那份。",
            ToggleSwitch::new()
                .is_on(g.full_width_punctuation)
                .on_toggled(context.callback(Message::FullWidthPunctuation)),
        ),
        field(
            "打拼音时敲标点",
            "「进入英文直输」能直接打 hello, world 这样带标点的英文；「先上屏候选再出标点」是 nihao, 出「你好，」；「自动」按拼音切不切得开来定。翻页键不受影响。",
            ComboBox::new()
                .items_source(PunctuationMode::ALL.iter().map(|m| m.label()))
                .selected_index(
                    PunctuationMode::ALL
                        .iter()
                        .position(|m| *m == g.punctuation_mode)
                        .unwrap_or(0),
                )
                .on_selection_changed(context.callback(Message::PunctuationMode)),
        ),
        field(
            "英文模式标点转全角",
            "中英各记一份，缺省英文半角。",
            ToggleSwitch::new()
                .is_on(g.english_full_width_punctuation)
                .on_toggled(context.callback(Message::EnglishFullWidthPunctuation)),
        ),
        field(
            "英文模式（Caps Lock）也给候选",
            "Tab 或方向键选词；空格、回车、标点仍原样上屏敲的字母，不选词时与直接打字一样。",
            ToggleSwitch::new()
                .is_on(g.english_candidates)
                .on_toggled(context.callback(Message::EnglishCandidates)),
        ),
        field(
            "预测下一个词（Tab 接受）",
            "",
            ToggleSwitch::new()
                .is_on(g.english_prediction)
                .is_enabled(g.english_candidates)
                .on_toggled(context.callback(Message::EnglishPrediction)),
        ),
        field(
            "但在终端和代码编辑器里不给",
            "终端、Windows Terminal、VS Code、Cursor、JetBrains 等，那里的候选窗口会挡住应用自己的补全；名单可在配置文件里改。",
            ToggleSwitch::new()
                .is_on(english_off)
                .is_enabled(g.english_candidates)
                .on_toggled(context.callback(Message::EnglishOffInApps)),
        ),
        field(
            "输入拼音时中文候选排在英文词前面",
            "开着时整段输入是英文词时（hello、key）英文词排第二，空格上屏的仍是中文；关着（缺省）拼音不成立的输入英文词排第一。",
            ToggleSwitch::new()
                .is_on(g.chinese_first)
                .is_enabled(g.mixed_english_candidates)
                .on_toggled(context.callback(Message::ChineseFirst)),
        ),
        field(
            "输入拼音时也给英文词候选",
            "hello 给出 hello、compa 补全成 company 这类英文词；关掉后只给中文。与上面的「英文模式也给候选」无关。",
            ToggleSwitch::new()
                .is_on(g.mixed_english_candidates)
                .on_toggled(context.callback(Message::MixedEnglishCandidates)),
        ),
        field(
            "给 emoji 候选",
            "kaixin 在「开心」后面给出 😄；关掉后候选里不出 emoji。",
            ToggleSwitch::new()
                .is_on(g.emoji_candidates)
                .on_toggled(context.callback(Message::EmojiCandidates)),
        ),
    ];
    page("通用", StackPanel::new().spacing(16.0).children(rows))
}

#[cfg(test)]
mod tests {
    use glimmer_core::WubiVariant;
    use glimmer_platform::Scheme;

    use super::{SCHEME, WUBI};

    /// 界面的拼音方案下拉必须跟着平台层的方案表走：新方案没加进 SCHEME，或者文案 / 配置写法对不上，这里拦住。
    #[test]
    fn scheme_options_match_platform_schemes() {
        assert_eq!(SCHEME.len(), Scheme::ALL.len());
        for (option, scheme) in SCHEME.iter().zip(Scheme::ALL) {
            assert_eq!(option.0, scheme.label());
            assert_eq!(option.1, scheme.key());
        }
    }

    /// 界面的五笔下拉必须跟着 Core 的版本表走：多一个版本没加进 WUBI，或者文案 / 配置写法
    /// 与 Core 对不上，用户选了就会写出 Core 不认的 `[general] wubi`，这里直接拦住。
    #[test]
    fn wubi_options_match_core_variants() {
        assert_eq!(WUBI.len(), WubiVariant::ALL.len() + 1);
        for (option, variant) in WUBI[1..].iter().zip(WubiVariant::ALL) {
            assert_eq!(option.0, variant.label());
            assert_eq!(option.1, variant.config_key());
        }
    }
}
