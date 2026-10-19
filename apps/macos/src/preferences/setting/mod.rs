mod value;

pub use value::SettingValue;

use glimmer_core::{FuzzyRules, WubiVariant};
use objc2_foundation::NSInteger;

/// 偏好设置里可选的五笔版本，弹出菜单第 0 项「关」之后按这个顺序列。
pub const WUBI_VARIANTS: &[WubiVariant] = &WubiVariant::ALL;

/// 模糊音勾选框的 tag 起点，后面加规则在 [`FuzzyRules::NAMES`] 里的下标。
const FUZZY_TAG_BASE: NSInteger = 100;

/// 附加词库开关的 tag 起点，后面加词库在列表里的下标。
const DICTIONARY_ENABLED_TAG_BASE: NSInteger = 200;

/// 附加词库「移除」按钮的 tag 起点。
const DICTIONARY_REMOVE_TAG_BASE: NSInteger = 300;

/// 一页最多列多少本附加词库（tag 段的宽度）。
pub const MAX_DICTIONARIES: usize = 100;

/// 设置窗口里的每个控件对应的配置项。编码进控件的 tag，`changed:` 里再解出来。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Setting {
    /// `[general] learning_language`，弹出菜单，选项是打进包里的释义表语言。
    LearningLanguage,

    /// `[general] page_size`，弹出菜单 1–9。
    PageSize,

    /// `[general] page_keys`，弹出菜单。
    PageKeys,

    /// `[general] theme`，弹出菜单。
    Theme,

    /// `[general] renderer`，弹出菜单：微明渲染器 / 系统绘制。
    Renderer,

    /// `[general] color_scheme`，弹出菜单：配色方案。
    ColorScheme,

    /// `[general] background_color`，取色器，值是 `#RRGGBB`。
    BackgroundColor,

    /// `[general] highlight_color`，取色器，值是 `#RRGGBB`。
    HighlightColor,

    /// 「恢复配色的颜色」按钮：清掉自定义背景色与高亮底色。
    ResetColors,

    /// `[general] font`，字体列表选中的字族名；「系统默认」为系统字体。
    Font,

    /// `[general] font_size`，弹出菜单：第 i 项是 `MIN_FONT_SIZE + i` 点。
    FontSize,

    /// `[shortcut] expression`，弹出菜单 v / u / i。
    ExpressionKey,

    /// `[shortcut] question`，弹出菜单 v / u / i。
    QuestionKey,

    /// `[shortcut] question_mark`，勾选框：没在组句时敲 `?` 也进问字。
    QuestionMark,

    /// `[fuzzy]` 里的一条规则，值是 [`FuzzyRules::NAMES`] 的下标。
    Fuzzy(usize),

    /// `[predict] enabled`。
    CloudEnabled,

    /// `[model] enabled`。
    LocalModelEnabled,

    /// 默认中文标点模式。
    FullWidthPunctuation,

    /// 选择已有自定义短语。
    SelectPhrase,

    /// 仅编辑草稿，不立即保存。
    PhraseDraft,

    /// 保存自定义短语。
    SavePhrase,

    /// 删除当前自定义短语。
    DeletePhrase,

    /// 新增文本。
    NewPhrase,

    /// 编辑选中行。
    EditPhrase,

    /// 关闭编辑表单。
    CancelPhraseEdit,

    /// `[general] system_text_replacements`，勾选框：系统的文本替换并进自定义短语。
    SystemTextReplacements,

    /// `[predict] base_url`。
    BaseUrl,

    /// `[predict] model`。
    Model,

    /// 密钥，写到 `.env`。
    ApiKey,

    /// 「在编辑器中打开配置文件」按钮。
    OpenConfigFile,

    /// `[general] layout`，弹出菜单 竖排 / 横排。
    Layout,

    /// `[general] preedit`，弹出菜单。
    Preedit,

    /// `[general] punctuation_mode`，弹出菜单：组句中敲标点进直输 / 先上屏 / 自动。
    PunctuationMode,

    /// `[general] english_candidates`，勾选框。
    EnglishCandidates,

    /// `[general] chinese_first`，勾选框：中英混输时中文候选排在英文词前。
    ChineseFirst,

    /// `[general] mixed_english_candidates`，勾选框：中文模式下给英文词候选。
    MixedEnglishCandidates,

    /// `[general] emoji_candidates`，勾选框：给 emoji 候选。
    EmojiCandidates,

    /// `[general] translation_reading`，勾选框：译词带读音（英语音标 / 日语假名）。
    TranslationReading,

    /// `[shortcut] translation`，快捷键录制按钮（只记修饰键）。
    TranslationKeys,

    /// `[shortcut] translation_second`，同上。
    TranslationSecondKeys,

    /// `[shortcut] translate_selection`，快捷键录制按钮（修饰键 + 字母）。
    TranslateSelectionKeys,

    /// `[shortcut] mode_switch`，中英文切换组合键。
    ModeSwitchKeys,

    /// 「恢复默认快捷键」按钮：翻页、前缀、中英切换、译词、删候选与翻译键全部回缺省。
    ResetShortcuts,

    /// 「导入词库…」按钮：选文件，转成 `.qj` 放进用户目录 `dicts/`。
    ImportDictionary,

    /// 第 N 本附加词库的启用勾选框（下标是 Host 词库列表里的位置）。
    DictionaryEnabled(usize),

    /// 第 N 本附加词库的「移除」按钮。
    DictionaryRemove(usize),

    /// `[general] scheme`，弹出菜单：全拼 + 六套双拼 + 大千注音 + 关（只用五笔），按 `Scheme::ALL` 的顺序。
    Scheme,

    /// [general] traditional，勾选框：繁体输出。
    Traditional,

    /// `[general] log_level`，勾选框：勾上是 debug。
    VerboseLog,

    /// 「关于」页「打开日志目录」按钮。
    OpenLogDirectory,

    /// 「关于」页「复制诊断信息」按钮。
    CopyDiagnostics,

    /// 「关于」「高级」页「打包日志到桌面」按钮。
    ExportLogs,

    /// `[predict] slots`，弹出菜单 0–4：第一页末尾留给云端词的格数。
    CloudSlots,

    /// `[apps] english_candidates_off`，勾选框：勾上写缺省的终端 / 编辑器列表，去掉写空表。
    EnglishCandidatesOffInApps,

    /// `[shortcut] delete_candidate`，快捷键录制按钮（只记修饰键）。
    DeleteCandidateKeys,

    /// `[general] input_log`，勾选框。
    InputLog,

    /// 学习输入习惯开关。
    Learning,

    /// 「高级」页「清空输入日志」按钮。
    ClearInputLog,

    /// 「高级」页「从文件学习…」按钮：选自己写过的文本，学进个人 n-gram。
    LearnText,

    /// 「云服务」页「测试连接」按钮。
    TestCloud,

    /// 「关于」页「官网」按钮。
    OpenWebsite,

    /// 「关于」页「GitHub」按钮。
    OpenRepository,

    /// `[general] wubi`，弹出菜单：关 + [`WUBI_VARIANTS`] 里的版本。
    Wubi,

    /// `[wubi] auto_select`，勾选框：敲满四码命中全码就自动上屏。
    WubiAutoSelect,

    /// `[wubi] sentence`，勾选框：连着打编码由引擎切词出整句，空格上屏。
    WubiSentence,

    /// `[wubi] hint`，勾选框：逐键提示候选右侧显示完整编码。
    WubiHint,

    /// `[update] check`，勾选框：启动时自动检查更新。
    UpdateCheck,

    /// 「关于」页「检查更新」按钮。
    CheckUpdate,

    /// 「关于」页「下载并安装」按钮（有新版本才显示）。
    InstallUpdate,
}

impl Setting {
    pub fn tag(self) -> NSInteger {
        match self {
            Self::LearningLanguage => 1,
            Self::PageSize => 2,
            Self::PageKeys => 3,
            Self::Theme => 4,
            Self::ExpressionKey => 5,
            Self::QuestionKey => 6,
            Self::QuestionMark => 48,
            Self::CloudEnabled => 7,
            Self::BaseUrl => 8,
            Self::Model => 9,
            Self::ApiKey => 10,
            Self::OpenConfigFile => 11,
            Self::Layout => 12,
            Self::Preedit => 13,
            Self::EnglishCandidates => 14,
            Self::ChineseFirst => 51,
            Self::MixedEnglishCandidates => 53,
            Self::EmojiCandidates => 54,
            Self::TranslationReading => 56,
            Self::TranslationKeys => 15,
            Self::TranslationSecondKeys => 16,
            Self::TranslateSelectionKeys => 17,
            Self::ResetShortcuts => 18,
            Self::ImportDictionary => 19,
            Self::Scheme => 20,
            Self::Traditional => 52,
            Self::VerboseLog => 21,
            Self::OpenLogDirectory => 22,
            Self::CopyDiagnostics => 23,
            Self::ExportLogs => 57,
            Self::CloudSlots => 24,
            Self::EnglishCandidatesOffInApps => 25,
            Self::DeleteCandidateKeys => 26,
            Self::InputLog => 27,
            Self::Learning => 49,
            Self::ClearInputLog => 28,
            Self::LearnText => 61,
            Self::TestCloud => 29,
            Self::OpenWebsite => 30,
            Self::OpenRepository => 31,
            Self::LocalModelEnabled => 32,
            Self::FullWidthPunctuation => 33,
            Self::SelectPhrase => 34,
            Self::PhraseDraft => 35,
            Self::SavePhrase => 36,
            Self::DeletePhrase => 37,
            Self::NewPhrase => 38,
            Self::EditPhrase => 39,
            Self::CancelPhraseEdit => 40,
            Self::ModeSwitchKeys => 41,
            Self::PunctuationMode => 42,
            Self::Wubi => 43,
            Self::WubiAutoSelect => 44,
            Self::WubiSentence => 62,
            Self::WubiHint => 45,
            Self::Renderer => 46,
            Self::Font => 47,
            Self::FontSize => 55,
            Self::ColorScheme => 63,
            Self::BackgroundColor => 64,
            Self::HighlightColor => 65,
            Self::ResetColors => 66,
            Self::SystemTextReplacements => 50,
            Self::UpdateCheck => 58,
            Self::CheckUpdate => 59,
            Self::InstallUpdate => 60,
            Self::Fuzzy(index) => FUZZY_TAG_BASE + index as NSInteger,
            Self::DictionaryEnabled(index) => DICTIONARY_ENABLED_TAG_BASE + index as NSInteger,
            Self::DictionaryRemove(index) => DICTIONARY_REMOVE_TAG_BASE + index as NSInteger,
        }
    }

    pub fn from_tag(tag: NSInteger) -> Option<Self> {
        Some(match tag {
            1 => Self::LearningLanguage,
            2 => Self::PageSize,
            3 => Self::PageKeys,
            4 => Self::Theme,
            46 => Self::Renderer,
            47 => Self::Font,
            55 => Self::FontSize,
            63 => Self::ColorScheme,
            64 => Self::BackgroundColor,
            65 => Self::HighlightColor,
            66 => Self::ResetColors,
            5 => Self::ExpressionKey,
            6 => Self::QuestionKey,
            48 => Self::QuestionMark,
            7 => Self::CloudEnabled,
            8 => Self::BaseUrl,
            9 => Self::Model,
            10 => Self::ApiKey,
            11 => Self::OpenConfigFile,
            12 => Self::Layout,
            13 => Self::Preedit,
            14 => Self::EnglishCandidates,
            51 => Self::ChineseFirst,
            53 => Self::MixedEnglishCandidates,
            54 => Self::EmojiCandidates,
            56 => Self::TranslationReading,
            15 => Self::TranslationKeys,
            16 => Self::TranslationSecondKeys,
            17 => Self::TranslateSelectionKeys,
            18 => Self::ResetShortcuts,
            19 => Self::ImportDictionary,
            20 => Self::Scheme,
            52 => Self::Traditional,
            21 => Self::VerboseLog,
            22 => Self::OpenLogDirectory,
            23 => Self::CopyDiagnostics,
            57 => Self::ExportLogs,
            24 => Self::CloudSlots,
            25 => Self::EnglishCandidatesOffInApps,
            26 => Self::DeleteCandidateKeys,
            27 => Self::InputLog,
            49 => Self::Learning,
            28 => Self::ClearInputLog,
            61 => Self::LearnText,
            29 => Self::TestCloud,
            30 => Self::OpenWebsite,
            31 => Self::OpenRepository,
            32 => Self::LocalModelEnabled,
            33 => Self::FullWidthPunctuation,
            34 => Self::SelectPhrase,
            35 => Self::PhraseDraft,
            36 => Self::SavePhrase,
            37 => Self::DeletePhrase,
            38 => Self::NewPhrase,
            39 => Self::EditPhrase,
            40 => Self::CancelPhraseEdit,
            41 => Self::ModeSwitchKeys,
            42 => Self::PunctuationMode,
            43 => Self::Wubi,
            44 => Self::WubiAutoSelect,
            62 => Self::WubiSentence,
            45 => Self::WubiHint,
            50 => Self::SystemTextReplacements,
            58 => Self::UpdateCheck,
            59 => Self::CheckUpdate,
            60 => Self::InstallUpdate,
            _ if tag >= DICTIONARY_REMOVE_TAG_BASE => {
                let index = usize::try_from(tag - DICTIONARY_REMOVE_TAG_BASE).ok()?;
                (index < MAX_DICTIONARIES).then_some(Self::DictionaryRemove(index))?
            }
            _ if tag >= DICTIONARY_ENABLED_TAG_BASE => {
                let index = usize::try_from(tag - DICTIONARY_ENABLED_TAG_BASE).ok()?;
                (index < MAX_DICTIONARIES).then_some(Self::DictionaryEnabled(index))?
            }
            _ => {
                let index = usize::try_from(tag.checked_sub(FUZZY_TAG_BASE)?).ok()?;
                (index < FuzzyRules::NAMES.len()).then_some(Self::Fuzzy(index))?
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tags_round_trip() {
        let all = [
            Setting::LearningLanguage,
            Setting::PageSize,
            Setting::PageKeys,
            Setting::Theme,
            Setting::Renderer,
            Setting::Font,
            Setting::FontSize,
            Setting::ColorScheme,
            Setting::BackgroundColor,
            Setting::HighlightColor,
            Setting::ResetColors,
            Setting::ExpressionKey,
            Setting::QuestionKey,
            Setting::CloudEnabled,
            Setting::LocalModelEnabled,
            Setting::BaseUrl,
            Setting::Model,
            Setting::ApiKey,
            Setting::OpenConfigFile,
            Setting::Layout,
            Setting::Preedit,
            Setting::PunctuationMode,
            Setting::EnglishCandidates,
            Setting::ChineseFirst,
            Setting::MixedEnglishCandidates,
            Setting::EmojiCandidates,
            Setting::TranslationReading,
            Setting::TranslationKeys,
            Setting::TranslationSecondKeys,
            Setting::TranslateSelectionKeys,
            Setting::ModeSwitchKeys,
            Setting::ResetShortcuts,
            Setting::ImportDictionary,
            Setting::Scheme,
            Setting::Traditional,
            Setting::VerboseLog,
            Setting::OpenLogDirectory,
            Setting::CopyDiagnostics,
            Setting::ExportLogs,
            Setting::CloudSlots,
            Setting::EnglishCandidatesOffInApps,
            Setting::DeleteCandidateKeys,
            Setting::InputLog,
            Setting::SystemTextReplacements,
            Setting::ClearInputLog,
            Setting::LearnText,
            Setting::TestCloud,
            Setting::OpenWebsite,
            Setting::OpenRepository,
            Setting::Wubi,
            Setting::WubiAutoSelect,
            Setting::WubiSentence,
            Setting::WubiHint,
            Setting::UpdateCheck,
            Setting::CheckUpdate,
            Setting::InstallUpdate,
            Setting::DictionaryEnabled(0),
            Setting::DictionaryEnabled(MAX_DICTIONARIES - 1),
            Setting::DictionaryRemove(3),
            Setting::Fuzzy(0),
            Setting::Fuzzy(FuzzyRules::NAMES.len() - 1),
        ];
        for setting in all {
            assert_eq!(Setting::from_tag(setting.tag()), Some(setting));
        }
        assert_eq!(Setting::from_tag(0), None);
        assert_eq!(
            Setting::from_tag(FUZZY_TAG_BASE + FuzzyRules::NAMES.len() as NSInteger),
            None
        );
        assert_eq!(
            Setting::from_tag(DICTIONARY_REMOVE_TAG_BASE + MAX_DICTIONARIES as NSInteger),
            None
        );
    }
}
