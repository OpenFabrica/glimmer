//! 设置窗口的消息类型：导航切换与各页的「改动」，根组件的 `update` 据此落盘。

/// 设置窗口的消息；「改动」消息带控件新值，`update` 据此落盘。
#[derive(Clone)]
pub(crate) enum Message {
    /// 导航切换分节（`None` 是取消选中，忽略）。
    Navigate(Option<String>),

    // 通用页
    LearningLanguage(Option<usize>),
    PageSize(Option<f64>),
    /// `[general] scheme`：拼音侧方案（全拼 / 双拼 / 注音 / 关），下标对 `general::SCHEME`。
    Scheme(Option<usize>),
    /// `[general] wubi`：形码侧版本，下标对 `general::WUBI`。与 `Scheme` 互不影响，两边都开就是混输。
    Wubi(Option<usize>),
    /// `[wubi] sentence`：五笔整句输入，开着时不再四码自动上屏。
    WubiSentence(bool),
    /// `[wubi] auto_select`。
    WubiAutoSelect(bool),
    /// `[wubi] hint`。
    WubiHint(bool),
    Traditional(bool),
    EnglishCandidates(bool),
    ChineseFirst(bool),
    /// `[general] mixed_english_candidates`：中文模式下给英文词候选。
    MixedEnglishCandidates(bool),
    /// `[general] emoji_candidates`。
    EmojiCandidates(bool),
    /// `[general] translation_reading`：译词带读音（英语音标 / 日语假名）。
    TranslationReading(bool),
    FullWidthPunctuation(bool),
    EnglishFullWidthPunctuation(bool),
    PunctuationMode(Option<usize>),
    /// 开=写入平台默认名单，关=清空。
    EnglishOffInApps(bool),

    // 候选窗口页
    Theme(Option<usize>),
    /// `[general] color_scheme`：下标对 `ColorScheme::ALL`。
    ColorScheme(Option<usize>),
    /// 背景颜色框里的文字变了：空或合法的 `#RRGGBB` / `#RRGGBBAA` 就落盘。
    BackgroundColor(String),
    /// 高亮颜色框里的文字变了：同上。
    HighlightColor(String),
    Layout(Option<usize>),
    Preedit(Option<usize>),
    Renderer(Option<usize>),
    /// 字体框里的文字变了：空或正好是某个字族名就落盘。
    FontQuery(String),
    /// 从提示里选了一个字族。
    Font(String),
    /// `[general] font_size`：数字框的新值。
    FontSize(Option<f64>),
    StatusBar(bool),

    // 云服务页
    LocalModel(bool),
    CloudEnabled(bool),
    CloudApiKey(String),
    CloudModel(String),
    CloudBaseUrl(String),
    CloudSlots(Option<f64>),
    CloudSentence(bool),
    TestConnection,
    CloudTestDone(Result<String, String>),

    // 快捷键页
    PageKeys(Option<usize>),
    ModeExpression(Option<usize>),
    ModeQuestion(Option<usize>),
    QuestionMark(bool),
    Translation(Option<usize>),
    TranslationSecond(Option<usize>),
    DeleteCandidate(Option<usize>),
    /// 只换修饰键，字母键固定用当前的。
    TranslateSelection(Option<usize>),

    // 模糊音页
    /// 配置键 + 新值。
    Fuzzy(&'static str, bool),

    // 词库页
    ToggleDomain(String, bool),
    ToggleUserDict(String, bool),
    /// 挪进 dicts\removed，不真删。
    RemoveUserDict(String),
    ImportDictionary,

    // 高级页
    VerboseLog(bool),
    InputLog(bool),
    /// 学习输入习惯开关。
    Learning(bool),
    OpenConfigFile,
    OpenDataDir,
    OpenLogDir,
    /// 日志目录 + config.toml 打成 zip 放桌面。
    ExportLogs,
    ClearInputLog,

    // 关于页
    OpenWebsite,
    OpenRepository,
    /// 「检查更新」按钮。
    CheckUpdate,
    /// 后台检查完了：`Ok(None)` 已是最新，`Ok(Some)` 有新版本。
    UpdateChecked(Result<Option<glimmer_update::Available>, String>),
    /// 「下载并安装」按钮。
    InstallUpdate,
    /// 后台下载完了：校验过的安装包路径。
    UpdateDownloaded(Result<std::path::PathBuf, String>),
    /// `[update] check`。
    AutoUpdateCheck(bool),
}
