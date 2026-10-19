//! 根组件的 Reactor 生命周期：建状态、按消息落盘、画左侧导航 + 当前页。

use glimmer_platform::{
    CandidateRenderer, ColorScheme, Config, DEFAULT_ENGLISH_CANDIDATES_OFF_WINDOWS, LayoutMode,
    LogLevel, MAX_FONT_SIZE, MIN_FONT_SIZE, PreeditMode, PunctuationMode, ThemeMode,
};
use windows_reactor::*;

use super::cloud_status::CloudStatus;
use super::controls::{export_logs, log_dir, open_in_editor, open_with_explorer};
use super::pages::{about, cloud, dictionaries, general, shortcut};
use super::update_status::UpdateStatus;
use super::{Message, Settings};

impl Settings {
    /// 起一次后台检查；正在查或正在下载就不重复起。
    fn start_update_check(&mut self, context: &ComponentContext<Self>, manual: bool) {
        if matches!(
            self.update,
            UpdateStatus::Checking { .. } | UpdateStatus::Downloading(_)
        ) {
            return;
        }
        self.update = UpdateStatus::Checking { manual };
        let config = self.config.update.clone();
        context.spawn_background(move |_cancel| Message::UpdateChecked(about::run_check(&config)));
    }
}

impl Component for Settings {
    type Input = ();
    type Message = Message;

    fn create(_input: &(), context: &ComponentContext<Self>) -> Self {
        let path = Self::config_path();
        Self::ensure_config_file(&path);
        let config = Config::load(&path).unwrap_or_default();
        let mut settings = Self {
            config,
            path,
            page: "general".to_string(),
            cloud_status: CloudStatus::Idle,
            update: UpdateStatus::Idle,
            dictionary_status: String::new(),
            families: glimmer_render::system_fonts::families(),
            font_query: None,
            background_draft: None,
            highlight_draft: None,
        };
        // 开着自动检查且 12 小时内没查过：设置程序一打开就在后台查一次，有新版本「关于」页会显示
        if settings.config.update.check && about::check_due() {
            settings.start_update_check(context, false);
        }
        settings
    }

    fn update(&mut self, message: Message, context: &ComponentContext<Self>) {
        match message {
            Message::Navigate(Some(tag)) => self.page = tag,
            Message::Navigate(None) => {}

            // 通用页
            Message::LearningLanguage(Some(i)) if i < general::LANGUAGES.len() => {
                self.save("general", "learning_language", general::LANGUAGES[i].1);
            }
            Message::PageSize(Some(value)) => {
                let size = (value.round() as i64).clamp(1, 9);
                self.save("general", "page_size", size);
            }
            Message::Scheme(Some(i)) if i < general::SCHEME.len() => {
                self.save("general", "scheme", general::SCHEME[i].1);
                // 旧键（scheme 之前的 shuangpin / zhuyin）留着不改变行为（写了 scheme 就不看它们），
                // 但手改配置的人会以为两处都管用，所以写新值时顺手清掉；save 一次只写一个键，分开写。
                if !self.config.general.shuangpin.is_empty() {
                    self.save("general", "shuangpin", "");
                }
                if self.config.general.zhuyin {
                    self.save("general", "zhuyin", false);
                }
            }
            Message::Wubi(Some(i)) if i < general::WUBI.len() => {
                self.save("general", "wubi", general::WUBI[i].1);
            }
            Message::WubiSentence(on) => self.save("wubi", "sentence", on),
            Message::WubiAutoSelect(on) => self.save("wubi", "auto_select", on),
            Message::WubiHint(on) => self.save("wubi", "hint", on),
            Message::Traditional(on) => self.save("general", "traditional", on),
            Message::EnglishCandidates(on) => self.save("general", "english_candidates", on),
            Message::ChineseFirst(on) => self.save("general", "chinese_first", on),
            Message::MixedEnglishCandidates(on) => {
                self.save("general", "mixed_english_candidates", on);
            }
            Message::EmojiCandidates(on) => self.save("general", "emoji_candidates", on),
            Message::TranslationReading(on) => self.save("general", "translation_reading", on),
            Message::FullWidthPunctuation(on) => {
                self.save("general", "full_width_punctuation", on);
            }
            Message::EnglishFullWidthPunctuation(on) => {
                self.save("general", "english_full_width_punctuation", on);
            }
            Message::EnglishOffInApps(on) => {
                let list: Vec<String> = if on {
                    DEFAULT_ENGLISH_CANDIDATES_OFF_WINDOWS
                        .iter()
                        .map(|s| (*s).to_owned())
                        .collect()
                } else {
                    Vec::new()
                };
                self.save_array("apps", "english_candidates_off", &list);
            }

            // 候选窗口页
            Message::Theme(Some(i)) if i < ThemeMode::ALL.len() => {
                self.save("general", "theme", ThemeMode::ALL[i].key());
            }
            Message::ColorScheme(Some(i)) if i < ColorScheme::ALL.len() => {
                self.save("general", "color_scheme", ColorScheme::ALL[i].key());
            }
            Message::BackgroundColor(text) => {
                self.background_draft = self.save_color("background_color", &text);
            }
            Message::HighlightColor(text) => {
                self.highlight_draft = self.save_color("highlight_color", &text);
            }
            Message::Layout(Some(i)) if i < LayoutMode::ALL.len() => {
                self.save("general", "layout", LayoutMode::ALL[i].key());
            }
            Message::PunctuationMode(Some(i)) if i < PunctuationMode::ALL.len() => {
                self.save("general", "punctuation_mode", PunctuationMode::ALL[i].key());
            }
            Message::Preedit(Some(i)) if i < PreeditMode::ALL.len() => {
                self.save("general", "preedit", PreeditMode::ALL[i].key());
            }
            Message::Renderer(Some(i)) if i < CandidateRenderer::ALL.len() => {
                self.save("general", "renderer", CandidateRenderer::ALL[i].key());
            }
            Message::FontQuery(text) => {
                let text = text.trim().to_owned();
                let exact = self
                    .families
                    .iter()
                    .find(|family| family.eq_ignore_ascii_case(&text))
                    .cloned();
                match exact {
                    Some(family) => {
                        self.font_query = None;
                        self.save("general", "font", family);
                    }
                    None if text.is_empty() => {
                        self.font_query = None;
                        self.save("general", "font", "");
                    }
                    None => self.font_query = Some(text),
                }
            }
            Message::Font(family) => {
                self.font_query = None;
                self.save("general", "font", family);
            }
            Message::FontSize(Some(value)) => {
                let size = (value.round() as i64)
                    .clamp(i64::from(MIN_FONT_SIZE), i64::from(MAX_FONT_SIZE));
                self.save("general", "font_size", size);
            }
            Message::StatusBar(on) => self.save("status_bar", "enabled", on),

            // 云服务页
            Message::LocalModel(on) => self.save("model", "enabled", on),
            Message::CloudEnabled(on) => self.save("predict", "enabled", on),
            Message::CloudApiKey(value) => self.save("predict", "api_key", value),
            Message::CloudModel(value) => self.save("predict", "model", value),
            Message::CloudBaseUrl(value) => self.save("predict", "base_url", value),
            Message::CloudSlots(Some(value)) => {
                let slots = (value.round() as i64).clamp(0, 9);
                self.save("predict", "slots", slots);
            }
            Message::CloudSentence(on) => self.save("predict", "sentence", on),
            Message::TestConnection => {
                if matches!(self.cloud_status, CloudStatus::Testing) {
                    return;
                }
                self.cloud_status = CloudStatus::Testing;
                let config = self.config.predict.clone();
                context.spawn_background(move |cancel| {
                    Message::CloudTestDone(cloud::run_test(&config, &cancel))
                });
            }
            Message::CloudTestDone(result) => {
                self.cloud_status = match result {
                    Ok(message) => CloudStatus::Ok(message),
                    Err(message) => CloudStatus::Failed(message),
                };
            }

            // 快捷键页
            Message::PageKeys(Some(i)) if i < shortcut::PAGE_KEYS.len() => {
                self.save("general", "page_keys", shortcut::PAGE_KEYS[i].1);
            }
            Message::ModeExpression(Some(i)) if i < shortcut::MODE_KEYS.len() => {
                self.save("shortcut", "expression", shortcut::MODE_KEYS[i]);
            }
            Message::ModeQuestion(Some(i)) if i < shortcut::MODE_KEYS.len() => {
                self.save("shortcut", "question", shortcut::MODE_KEYS[i]);
            }
            Message::QuestionMark(on) => self.save("shortcut", "question_mark", on),
            Message::Translation(Some(i)) if i < shortcut::MODIFIERS.len() => {
                self.save("shortcut", "translation", shortcut::MODIFIERS[i].1);
            }
            Message::TranslationSecond(Some(i)) if i < shortcut::MODIFIERS.len() => {
                self.save("shortcut", "translation_second", shortcut::MODIFIERS[i].1);
            }
            Message::DeleteCandidate(Some(i)) if i < shortcut::MODIFIERS.len() => {
                self.save("shortcut", "delete_candidate", shortcut::MODIFIERS[i].1);
            }
            Message::TranslateSelection(Some(i)) if i < shortcut::MODIFIERS.len() => {
                let key = self.config.shortcut.translate_selection.key;
                let combo = format!("{}+{key}", shortcut::MODIFIERS[i].1);
                self.save("shortcut", "translate_selection", combo);
            }

            // 模糊音页
            Message::Fuzzy(key, on) => self.save("fuzzy", key, on),

            // 词库页
            Message::ToggleDomain(name, on) => {
                let mut domains = self.config.dictionaries.domains.clone();
                if on {
                    if !domains.contains(&name) {
                        domains.push(name);
                    }
                } else {
                    domains.retain(|d| d != &name);
                }
                self.save_array("dictionaries", "domains", &domains);
            }
            Message::ToggleUserDict(name, on) => {
                // 用户词库缺省启用，`disabled` 列的是关掉的。
                let mut disabled = self.config.dictionaries.disabled.clone();
                if on {
                    disabled.retain(|d| d != &name);
                } else if !disabled.contains(&name) {
                    disabled.push(name);
                }
                self.save_array("dictionaries", "disabled", &disabled);
            }
            Message::RemoveUserDict(name) => {
                dictionaries::remove_user_dict(self, &name);
                self.reload();
            }
            Message::ImportDictionary => {
                dictionaries::import(self);
                self.reload();
            }

            // 高级页
            Message::VerboseLog(on) => {
                let level = if on { LogLevel::Debug } else { LogLevel::Info };
                self.save("general", "log_level", level.key());
            }
            Message::InputLog(on) => self.save("general", "input_log", on),
            Message::Learning(on) => self.save("general", "learning", on),
            Message::OpenConfigFile => {
                Self::ensure_config_file(&self.path);
                open_in_editor(&self.path);
            }
            Message::OpenDataDir => {
                Self::ensure_config_file(&self.path);
                open_with_explorer(&self.data_dir().to_string_lossy());
            }
            Message::OpenLogDir => {
                if let Some(logs) = log_dir() {
                    open_with_explorer(&logs.to_string_lossy());
                }
            }
            Message::ExportLogs => export_logs(),
            Message::ClearInputLog => {
                let log = self.data_dir().join("input-log.jsonl");
                if let Err(error) = std::fs::remove_file(&log)
                    && error.kind() != std::io::ErrorKind::NotFound
                {
                    crate::log::warn(format!("清空输入日志失败: {error}"));
                }
            }

            // 关于页
            Message::OpenWebsite => open_with_explorer(about::WEBSITE_URL),
            Message::OpenRepository => open_with_explorer(about::REPOSITORY_URL),
            Message::CheckUpdate => self.start_update_check(context, true),
            Message::UpdateChecked(result) => {
                let manual = matches!(self.update, UpdateStatus::Checking { manual: true });
                self.update = match result {
                    Ok(Some(update)) => UpdateStatus::Available(update),
                    Ok(None) => UpdateStatus::UpToDate,
                    // 自动查失败不打扰：关于页保持没查过的样子，日志里有原因
                    Err(_) if !manual => UpdateStatus::Idle,
                    Err(message) => UpdateStatus::Failed(message, None),
                };
            }
            Message::InstallUpdate => match self.update.clone() {
                // 已经下好：再拉一次安装器就行
                UpdateStatus::Downloaded(path) => open_with_explorer(&path.to_string_lossy()),
                UpdateStatus::Available(update) | UpdateStatus::Failed(_, Some(update)) => {
                    self.update = UpdateStatus::Downloading(update.clone());
                    context.spawn_background(move |_cancel| {
                        Message::UpdateDownloaded(about::run_download(&update))
                    });
                }
                _ => {}
            },
            Message::UpdateDownloaded(result) => {
                let UpdateStatus::Downloading(update) = self.update.clone() else {
                    return;
                };
                self.update = match result {
                    Ok(path) => {
                        // 交给 ShellExecute 拉起 Setup.exe（会弹 UAC）；安装器会先结束 Server 与本程序
                        open_with_explorer(&path.to_string_lossy());
                        UpdateStatus::Downloaded(path)
                    }
                    Err(message) => UpdateStatus::Failed(message, Some(update)),
                };
            }
            Message::AutoUpdateCheck(on) => self.save("update", "check", on),

            // 下拉被清空 / 越界：不改
            _ => {}
        }
    }

    fn view(&self, _input: &(), context: &mut ViewContext<Self>) -> View {
        context.window_title("微明设置");
        let item = |tag: &str, label: &str, symbol| {
            KeyedView::new(
                tag,
                NavigationViewItem::new()
                    .tag(tag)
                    .is_selected(self.page == tag)
                    .slots([
                        SlotView::new(
                            NavigationViewItemSlot::Icon,
                            SymbolIcon::new().symbol(symbol),
                        ),
                        SlotView::new(NavigationViewItemSlot::Content, label),
                    ]),
            )
        };
        let items = [
            item("general", "通用", Symbol::Setting),
            item("candidates", "候选窗口", Symbol::View),
            item("shortcut", "快捷键", Symbol::Keyboard),
            item("cloud", "云服务", Symbol::World),
            item("fuzzy", "模糊音", Symbol::Audio),
            item("dictionaries", "词库", Symbol::Library),
            item("usage", "统计", Symbol::List),
            item("advanced", "高级", Symbol::Repair),
            item("about", "关于", Symbol::Help),
        ];
        NavigationView::new()
            .pane_display_mode(NavigationViewPaneDisplayMode::Left)
            .pane_title("微明")
            .open_pane_length(220.0)
            .is_pane_open(true)
            .is_pane_toggle_button_visible(false)
            .is_back_button_visible(NavigationViewBackButtonVisible::Collapsed)
            .is_settings_visible(false)
            .on_selected_tag_changed(context.callback(Message::Navigate))
            .slots([
                SlotView::collection(NavigationViewSlot::MenuItems, items),
                SlotView::new(NavigationViewSlot::Content, self.page_content(context)),
            ])
    }
}
