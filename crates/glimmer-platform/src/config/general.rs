use glimmer_core::{PunctuationMode, ShuangpinScheme, WubiVariant};
use serde::{Deserialize, Serialize};

use super::scheme::{Scheme, scheme_label};
use super::{CandidateRenderer, ColorScheme, LayoutMode, LogLevel, PreeditMode, ThemeMode};

/// 每页最多几个候选：数字键只有 1–9。
pub const MAX_PAGE_SIZE: usize = 9;

/// 翻页键对的可选值，第一项是缺省：第一个键向前、第二个向后。
/// 缺省不用 `,` `.`：组句中敲逗号句号应该把首选上屏再补一个全角标点（`nihao,zaima` 一气打完），
/// 拿它们翻页就得先按空格再敲标点。选 `-` `=` 时组句中的 `-` 是翻页，不再进英文直输段（#43）。
pub const PAGE_KEY_OPTIONS: [&str; 3] = ["[]", ",.", "-="];

/// 候选窗口文字大小（点）的可调范围与缺省值：小于 12 在 Retina 以外的屏上发虚，大于 28 一页九条会高过半屏。
pub const MIN_FONT_SIZE: u32 = 12;

/// 候选窗口文字大小上限，见 [`MIN_FONT_SIZE`]。
pub const MAX_FONT_SIZE: u32 = 28;

/// 候选窗口文字大小缺省值，等于改成可调之前写死的字号。
pub const DEFAULT_FONT_SIZE: u32 = 16;

/// 缺省翻页键对，与 [`PAGE_KEY_OPTIONS`] 第一项一致。
pub const DEFAULT_PAGE_KEYS: (char, char) = ('[', ']');

/// `[general]` 分节：与具体功能无关的常规项。
/// `learning_language` 写这个值表示不显示译文。
pub const LEARNING_LANGUAGE_OFF: &str = "off";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct GeneralConfig {
    /// 学习语言（ISO 639-1，`en` / `ja` / `es`；`off` 不显示译文）：候选旁显示哪种语言的译文。要有对应的释义表文件才生效。
    pub learning_language: String,

    /// 候选旁的译词带不带读音：英语是美式音标、日语是假名（缺省带）。学习语言关着时无意义。
    pub translation_reading: bool,

    /// 每页候选数，1–9。
    pub page_size: usize,

    /// 翻页键对，两个字符：前一个上一页、后一个下一页。
    pub page_keys: String,

    /// 候选窗口外观。
    pub theme: ThemeMode,

    /// 候选窗口配色方案，浅色 / 深色两版跟着 `theme` 切换。只对微明渲染器生效。
    pub color_scheme: ColorScheme,

    /// 自定义候选窗口背景色，`#RRGGBB`；空串用配色方案的。设了就不分浅色深色，字色按背景深浅自动选。
    /// 只对微明渲染器生效，写错了按空串处理。
    pub background_color: String,

    /// 自定义当前候选的高亮底色，`#RRGGBB` 或带不透明度的 `#RRGGBBAA`；空串用配色方案的。
    /// 高亮上的字色按底色深浅自动选。只对微明渲染器生效，写错了按空串处理。
    pub highlight_color: String,

    /// 候选窗口竖排 / 横排。
    pub layout: LayoutMode,

    /// 候选窗口由微明渲染器还是系统原生绘制。
    pub renderer: CandidateRenderer,

    /// 候选窗口字体的字族名；空为系统字体。只对微明渲染器生效，没装这个字体时回到系统字体。
    pub font: String,

    /// 候选窗口里候选词的字号（点），译文与序号按比例跟着变；12–28，超出夹到边上。只对微明渲染器生效。
    pub font_size: u32,

    /// 组句中的拼音显示在行内、候选窗口还是两处都显示。
    pub preedit: PreeditMode,

    /// 英文模式（Caps Lock 亮着）是否给英文候选（补全与拼错纠正）。关掉就是纯直通。
    pub english_candidates: bool,

    /// 繁体输出模式。
    pub traditional: bool,
    /// 中文模式下中英混输时中文候选总排在英文词前面。缺省关：拼音不像话的输入（`hello`）英文词排第一，
    /// 常在中文模式里打英文词的人不受影响；想要中文永远在前的自己打开。
    pub chinese_first: bool,

    /// 中文模式下（中英混输）是否给英文词与英文补全候选（`hello`、`compa` → company）。缺省开。
    /// 与 `english_candidates` 不同：那个管英文模式（Caps Lock 亮着）。
    pub mixed_english_candidates: bool,

    /// 是否给 emoji 候选（`kaixin` → 😄）。缺省开；嫌挤占候选位置的可以关掉。
    pub emoji_candidates: bool,

    /// 中文模式下不在组句时敲的标点转成全角（`，。？！` 等，数字后的 `.` 保持半角）。
    /// Windows 悬浮状态条上可点切换；macOS 在偏好设置中选择默认模式。
    pub full_width_punctuation: bool,

    /// 英文模式下的同一件事，中英各记一份；缺省半角。只有 Windows 用（macOS 英文模式一律半角）。
    pub english_full_width_punctuation: bool,

    /// 组句中敲半角标点怎么办：`raw` 进英文直输段（缺省）、`commit` 先把候选上屏、`auto` 像英文才直输。
    pub punctuation_mode: PunctuationMode,

    /// 拼音侧方案：`pinyin`（全拼，缺省）/ `xiaohe` / `ziranma` / `microsoft` / `sogou` / `xiaolang` / `abc`
    /// / `zhuyin` / `none`（关，只用五笔），见 [`Scheme`]。用不认识的写法时按全拼并警告。
    /// 缺省是空串：文件里没写这一项时要去看旧键，见 [`Self::scheme`]。
    pub scheme: String,

    /// 五笔：空串关，`86` / `98` / `xsj`（新世纪，也可写 `06`）选版本（见 [`WubiVariant`]）。
    /// **与拼音侧同时开着就是混输**，见 [`Self::mixed`]。
    pub wubi: String,

    /// 旧键（`scheme` 之前的 `[general] shuangpin`，空串为全拼）：只在 [`Self::scheme`] 里用来推断方案，
    /// 新写的配置不用它；`scheme` 写了值就不看它。当时 `shuangpin` 与 `zhuyin` 是两个字段表达同一个维度。
    pub shuangpin: String,

    /// 旧键（同上的 `[general] zhuyin`）：同上。
    pub zhuyin: bool,

    /// 日志级别，缺省 info（不含用户敲的内容）。
    pub log_level: LogLevel,

    /// 输入日志：每次上屏记一行到数据目录的 `input-log.jsonl`（敲的键、看到的候选、选了什么），只写本机，
    /// 给离线回归评测与个人模型用。缺省开；关掉就不记，「高级」页可清空。
    pub input_log: bool,

    /// 学习输入习惯：按选择调整候选顺序、记新词与敲错纠正。关掉后不再记，已学的仍参与排序。
    pub learning: bool,

    /// 把系统的文本替换（macOS「键盘 → 文本替换」）并进自定义短语：输入码敲全后短语占该码最靠前的空位。只有 macOS 用。
    pub system_text_replacements: bool,
}

impl Default for GeneralConfig {
    fn default() -> Self {
        Self {
            learning_language: "en".to_owned(),
            translation_reading: true,
            page_size: MAX_PAGE_SIZE,
            page_keys: PAGE_KEY_OPTIONS[0].to_owned(),
            theme: ThemeMode::default(),
            color_scheme: ColorScheme::default(),
            background_color: String::new(),
            highlight_color: String::new(),
            layout: LayoutMode::default(),
            renderer: CandidateRenderer::default(),
            font: String::new(),
            font_size: DEFAULT_FONT_SIZE,
            preedit: PreeditMode::default(),
            english_candidates: true,
            traditional: false,
            chinese_first: false,
            mixed_english_candidates: true,
            emoji_candidates: true,
            full_width_punctuation: true,
            english_full_width_punctuation: false,
            punctuation_mode: PunctuationMode::default(),
            scheme: String::new(),
            wubi: String::new(),
            shuangpin: String::new(),
            zhuyin: false,
            log_level: LogLevel::default(),
            input_log: true,
            learning: true,
            system_text_replacements: true,
        }
    }
}

impl GeneralConfig {
    /// 学习语言关着（`learning_language = "off"`）：候选旁不显示译文，生词标记与释义兜底也停。
    pub fn learning_language_off(&self) -> bool {
        self.learning_language
            .trim()
            .eq_ignore_ascii_case(LEARNING_LANGUAGE_OFF)
    }

    /// 拼音侧方案。`scheme` 没写时用旧键（`shuangpin` / `zhuyin`）推，都没有就是全拼。
    pub fn scheme(&self) -> Scheme {
        let key = self.scheme.trim();
        if !key.is_empty() {
            return match key.parse() {
                Ok(scheme) => scheme,
                Err(_) => {
                    tracing::warn!(key, "不认识的拼音方案，按全拼");
                    Scheme::Pinyin
                }
            };
        }
        if self.zhuyin {
            tracing::info!("[general] zhuyin 已并入 scheme，可改成 scheme = \"zhuyin\"");
            return Scheme::Zhuyin;
        }
        let legacy = self.shuangpin.trim();
        if legacy.is_empty() {
            return Scheme::Pinyin;
        }
        match legacy.parse::<ShuangpinScheme>() {
            Ok(scheme) => {
                tracing::info!(
                    key = scheme.key(),
                    "[general] shuangpin 已并入 scheme，可改成它"
                );
                Scheme::Shuangpin(scheme)
            }
            Err(_) => {
                tracing::warn!(key = legacy, "不认识的双拼方案，按全拼");
                Scheme::Pinyin
            }
        }
    }

    /// 当前方案是双拼时是哪一套；不是双拼时为 `None`。
    pub fn shuangpin(&self) -> Option<ShuangpinScheme> {
        self.scheme().shuangpin()
    }

    /// 当前方案是不是大千注音。
    pub fn is_zhuyin(&self) -> bool {
        self.scheme() == Scheme::Zhuyin
    }

    /// 五笔版本；没开或写得不认识时为 `None`。
    /// 早先把五笔写在 `scheme` 里（那时它是单选的方案），[`Self::scheme`] 会把那种写法解成
    /// [`Scheme::Off`]，这里跟着认下来，免得老配置升级后两个轴都关着、一个候选都不出。
    pub fn wubi(&self) -> Option<WubiVariant> {
        let key = self.wubi.trim();
        if key.is_empty() {
            return self.scheme.trim().parse().ok();
        }
        match key.parse() {
            Ok(variant) => Some(variant),
            Err(_) => {
                tracing::warn!(key, "不认识的五笔版本，按关");
                None
            }
        }
    }

    /// 拼音与五笔同时开着 = 混输：两边都出候选，编码打全的五笔词在前。
    pub fn mixed(&self) -> bool {
        self.scheme().is_on() && self.wubi().is_some()
    }

    /// 状态条上显示的输入方案名；见 [`scheme_label`]。
    pub fn scheme_label(&self) -> String {
        scheme_label(self.scheme(), self.wubi())
    }

    /// 夹到合法范围的每页候选数。
    pub fn page_size(&self) -> usize {
        self.page_size.clamp(1, MAX_PAGE_SIZE)
    }

    /// 夹到合法范围的候选窗口字号（点）。
    pub fn font_size(&self) -> u32 {
        self.font_size.clamp(MIN_FONT_SIZE, MAX_FONT_SIZE)
    }

    /// 翻页键对；写得不对（不是两个不同的 ASCII 可见字符）时退回缺省。
    pub fn page_keys(&self) -> (char, char) {
        let mut chars = self.page_keys.chars();
        match (chars.next(), chars.next(), chars.next()) {
            (Some(previous), Some(next), None)
                if previous != next
                    && previous.is_ascii_graphic()
                    && next.is_ascii_graphic()
                    && !previous.is_ascii_alphanumeric()
                    && !next.is_ascii_alphanumeric() =>
            {
                (previous, next)
            }
            _ => DEFAULT_PAGE_KEYS,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn page_size_and_keys_are_sanitized() {
        let mut general = GeneralConfig::default();
        assert_eq!(general.page_size(), 9);
        assert_eq!(general.page_keys(), ('[', ']'));
        general.page_size = 0;
        general.page_keys = ",.".to_owned();
        assert_eq!(general.page_size(), 1);
        assert_eq!(general.page_keys(), (',', '.'));
        general.page_size = 42;
        general.page_keys = "ab".to_owned();
        assert_eq!(general.page_size(), 9);
        assert_eq!(general.page_keys(), ('[', ']'));
        general.page_keys = ",,".to_owned();
        assert_eq!(general.page_keys(), ('[', ']'));
    }

    #[test]
    fn font_size_defaults_to_16_and_is_clamped() {
        let mut general = GeneralConfig::default();
        assert_eq!(general.font_size(), DEFAULT_FONT_SIZE);
        general.font_size = 20;
        assert_eq!(general.font_size(), 20);
        general.font_size = 0;
        assert_eq!(general.font_size(), MIN_FONT_SIZE);
        general.font_size = 100;
        assert_eq!(general.font_size(), MAX_FONT_SIZE);
    }

    #[test]
    fn scheme_defaults_to_pinyin_and_unknown_names_fall_back() {
        let mut general = GeneralConfig::default();
        assert_eq!(general.scheme(), Scheme::Pinyin);
        assert_eq!(general.shuangpin(), None);
        general.scheme = "xiaohe".to_owned();
        assert_eq!(general.shuangpin(), Some(ShuangpinScheme::Xiaohe));
        general.scheme = " Sogou ".to_owned();
        assert_eq!(general.shuangpin(), Some(ShuangpinScheme::Sogou));
        general.scheme = "abc".to_owned();
        assert_eq!(general.shuangpin(), Some(ShuangpinScheme::Abc));
        general.scheme = "zhuyin".to_owned();
        assert!(general.is_zhuyin());
        general.scheme = "none".to_owned();
        assert_eq!(general.scheme(), Scheme::Off);
        assert!(!general.scheme().is_on());
        general.scheme = "flypy".to_owned();
        assert_eq!(general.scheme(), Scheme::Pinyin);
    }

    #[test]
    fn the_two_axes_are_independent_and_their_combination_is_mixed_input() {
        let mut general = GeneralConfig::default();
        // 缺省：全拼，不开五笔
        assert_eq!(general.scheme(), Scheme::Pinyin);
        assert_eq!(general.wubi(), None);
        assert!(!general.mixed());
        assert_eq!(general.scheme_label(), "");

        // 只有双拼
        general.scheme = "xiaohe".to_owned();
        assert!(!general.mixed());
        // 只有五笔：拼音侧关掉
        general.scheme = "none".to_owned();
        general.wubi = "98".to_owned();
        assert_eq!(general.wubi(), Some(WubiVariant::Wubi98));
        assert!(!general.mixed());
        assert_eq!(general.scheme_label(), "98 五笔");
        // 组合：两边都开 = 混输
        general.scheme = "xiaohe".to_owned();
        assert!(general.mixed());
        assert_eq!(general.scheme_label(), "98 五笔 + 小鹤双拼");
        // 五笔写得不认识：按关，且不影响拼音侧
        general.wubi = "2000".to_owned();
        assert_eq!(general.wubi(), None);
        assert_eq!(general.scheme(), Scheme::Shuangpin(ShuangpinScheme::Xiaohe));
    }

    #[test]
    fn files_written_before_the_scheme_key_keep_their_scheme() {
        let parse = |text: &str| toml::from_str::<GeneralConfig>(text).unwrap().scheme();
        assert_eq!(
            parse("shuangpin = \"xiaohe\"\n"),
            Scheme::Shuangpin(ShuangpinScheme::Xiaohe)
        );
        assert_eq!(parse("zhuyin = true\n"), Scheme::Zhuyin);
        assert_eq!(parse("shuangpin = \"\"\nzhuyin = false\n"), Scheme::Pinyin);
        assert_eq!(parse(""), Scheme::Pinyin);
        // 新键写了就以它为准
        assert_eq!(
            parse("scheme = \"pinyin\"\nshuangpin = \"xiaohe\"\n"),
            Scheme::Pinyin
        );
        // 老配置把五笔写在 scheme 里：拼音关、五笔跟着开，不然一个候选都不出
        let legacy: GeneralConfig = toml::from_str("scheme = \"wubi86\"\n").unwrap();
        assert_eq!(legacy.scheme(), Scheme::Off);
        assert_eq!(legacy.wubi(), Some(WubiVariant::Wubi86));
        assert!(!legacy.mixed());
    }

    #[test]
    fn wubi_is_off_by_default_and_accepts_known_variants() {
        let mut general = GeneralConfig::default();
        assert_eq!(general.wubi(), None);
        general.wubi = "86".to_owned();
        assert_eq!(general.wubi(), Some(WubiVariant::Wubi86));
        general.wubi = " wubi98 ".to_owned();
        assert_eq!(general.wubi(), Some(WubiVariant::Wubi98));
        general.wubi = "xsj".to_owned();
        assert_eq!(general.wubi(), Some(WubiVariant::Xinshiji));
        general.wubi = "06".to_owned();
        assert_eq!(general.wubi(), Some(WubiVariant::Xinshiji));
        general.wubi = "2000".to_owned();
        assert_eq!(general.wubi(), None);
    }

    #[test]
    fn legacy_shuangpin_and_zhuyin_keys_are_read_but_scheme_wins() {
        let mut general = GeneralConfig {
            scheme: String::new(),
            ..GeneralConfig::default()
        };
        assert_eq!(general.scheme(), Scheme::Pinyin);
        general.shuangpin = "xiaohe".to_owned();
        assert_eq!(general.scheme(), Scheme::Shuangpin(ShuangpinScheme::Xiaohe));
        general.zhuyin = true;
        assert_eq!(general.scheme(), Scheme::Zhuyin);
        assert!(general.is_zhuyin());
        // 旧配置里两个都写是不合法的，新键写了就不看它们
        general.scheme = "pinyin".to_owned();
        assert_eq!(general.scheme(), Scheme::Pinyin);
        assert!(!general.is_zhuyin());
    }
}
