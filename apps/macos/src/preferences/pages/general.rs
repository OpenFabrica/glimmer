//! 「通用」页：学习语言与译词读音、每页候选数、拼音方案 / 五笔、英文模式候选与下一词预测、中文模式英文词与 emoji 候选开关。

use glimmer_core::{Language, PunctuationMode};
use glimmer_platform::{Config, MAX_PAGE_SIZE, Scheme};
use objc2::MainThreadMarker;
use objc2::rc::Retained;
use objc2_app_kit::{NSButton, NSPopUpButton, NSTextField};
use objc2_foundation::NSString;

use crate::preferences::controls::{
    checkbox, language_label, note, row_aligned_checkbox, row_popup, row_sub_checkbox, select,
    set_checked, sub_note,
};
use crate::preferences::layout::Layout;
use crate::preferences::setting::{Setting, WUBI_VARIANTS};
use crate::preferences::target::PreferencesTarget;

pub struct GeneralPage {
    /// 学习语言。
    learning_language: Retained<NSPopUpButton>,

    /// 译词带读音（英语音标 / 日语假名）；不显示译文时置灰。
    translation_reading: Retained<NSButton>,

    /// 每页候选数。
    page_size: Retained<NSPopUpButton>,

    /// 拼音方案（按 `Scheme::ALL` 的顺序，末项是「关」）。
    scheme: Retained<NSPopUpButton>,

    /// 五笔版本（第 0 项是关）；与拼音方案同时开着就是混输。
    wubi: Retained<NSPopUpButton>,

    /// 五笔：敲满四码命中全码就自动上屏；整句开着时置灰。
    wubi_auto_select: Retained<NSButton>,

    /// 五笔：连着打编码由引擎切词出整句，空格上屏。
    wubi_sentence: Retained<NSButton>,

    /// 五笔：逐键提示候选右侧显示完整编码。
    wubi_hint: Retained<NSButton>,
    /// 繁体输出模式。
    traditional: Retained<NSButton>,

    /// 英文模式带补全、纠错、预测（英文辅助）。
    english: Retained<NSButton>,

    /// 英文辅助下面的说明，写着配置里实际的开关快捷键。
    english_note: Retained<NSTextField>,

    /// 终端 / 编辑器里不给英文候选。
    english_off_in_apps: Retained<NSButton>,

    /// 英文模式下预测下一个词（Tab 接受）；不给英文候选时置灰。
    english_prediction: Retained<NSButton>,

    /// 中文模式下给英文词候选。
    mixed_english: Retained<NSButton>,

    /// 中英混输时中文候选排在英文词前；不给英文词候选时置灰。
    chinese_first: Retained<NSButton>,

    /// 给 emoji 候选。
    emoji: Retained<NSButton>,

    /// 学习语言弹出菜单里各项对应的语言。
    languages: Vec<Language>,

    /// 默认中文标点模式。
    punctuation: Retained<NSPopUpButton>,

    /// 组句中敲标点怎么办。
    punctuation_mode: Retained<NSPopUpButton>,
}

impl GeneralPage {
    /// `languages` 是打进包里的释义表语言。
    pub fn build(
        layout: &mut Layout,
        mtm: MainThreadMarker,
        target: &PreferencesTarget,
        languages: &[Language],
    ) -> Self {
        // 最后一项是关
        let language_titles: Vec<String> = languages
            .iter()
            .map(|l| language_label(*l).to_owned())
            .chain(std::iter::once("不显示译文".to_owned()))
            .collect();
        let learning_language = row_popup(
            layout,
            mtm,
            "学习语言",
            &language_titles,
            Setting::LearningLanguage,
            target,
        );
        note(
            layout,
            mtm,
            "候选词右侧显示哪种语言的译词，只列出安装了释义表的语言；「不显示译文」同时关掉生词标记与释义兜底。",
        );
        let translation_reading =
            checkbox(mtm, "显示译文音标", Setting::TranslationReading, target);
        row_sub_checkbox(layout, &translation_reading);
        sub_note(
            layout,
            mtm,
            "英文译词后面的美式音标（develop (dɪˈveləp)），日语译词的假名注音也随它；选「不显示译文」时此项不可用。",
        );
        let page_size_titles: Vec<String> = (1..=MAX_PAGE_SIZE).map(|n| n.to_string()).collect();
        let page_size = row_popup(
            layout,
            mtm,
            "每页候选数",
            &page_size_titles,
            Setting::PageSize,
            target,
        );
        let scheme_titles: Vec<String> = Scheme::ALL.iter().map(|s| s.label().to_owned()).collect();
        let scheme = row_popup(
            layout,
            mtm,
            "拼音方案",
            &scheme_titles,
            Setting::Scheme,
            target,
        );
        note(
            layout,
            mtm,
            "全拼、六套双拼、大千注音，或「关（只用五笔）」——选「关」就只剩下面的五笔。双拼下 v、u、i 是音节键，表达式与问字模式改用 Shift+V、Shift+U 进（微软、搜狗方案的 ; 键是 ing）；注音下数字键与 - ; , . / 都是注音符号，选词改按 Enter、Space 是一声。",
        );
        let wubi_titles: Vec<String> = std::iter::once("关".to_owned())
            .chain(WUBI_VARIANTS.iter().map(|v| v.label().to_owned()))
            .collect();
        let wubi = row_popup(layout, mtm, "五笔", &wubi_titles, Setting::Wubi, target);
        note(
            layout,
            mtm,
            "三版五笔按自己学的那一版选，候选右侧注五笔码。与上面的拼音方案同时开着就是混输：五笔候选在前，打不出的字直接打拼音，第 5 个字母起五笔没有更长的编码，自然只剩拼音。只用五笔请把拼音方案选「关」，那时 z 开头是拼音反查。",
        );
        let wubi_auto_select = checkbox(mtm, "四码自动上屏", Setting::WubiAutoSelect, target);
        row_sub_checkbox(layout, &wubi_auto_select);
        let wubi_sentence = checkbox(
            mtm,
            "整句输入（连着打编码，空格上屏）",
            Setting::WubiSentence,
            target,
        );
        row_sub_checkbox(layout, &wubi_sentence);
        let wubi_hint = checkbox(mtm, "显示编码提示", Setting::WubiHint, target);
        row_sub_checkbox(layout, &wubi_hint);
        sub_note(
            layout,
            mtm,
            "敲满四码且命中全码时首选直接上屏，不用按空格；混输下不生效（四个字母也可能是一段拼音）。编码提示是逐键提示候选右侧的完整编码。",
        );
        let punctuation = row_popup(
            layout,
            mtm,
            "默认中文标点",
            &["全角（，；：）".to_owned(), "半角（,;:）".to_owned()],
            Setting::FullWidthPunctuation,
            target,
        );
        note(
            layout,
            mtm,
            "仅影响标点，字母和数字保持半角；自定义短语原样输出。设置会保存。 ",
        );
        let traditional = checkbox(mtm, "繁体输出", Setting::Traditional, target);
        row_aligned_checkbox(layout, &traditional);
        let mode_titles: Vec<String> = PunctuationMode::ALL
            .iter()
            .map(|m| m.label().to_owned())
            .collect();
        let punctuation_mode = row_popup(
            layout,
            mtm,
            "打拼音时敲标点",
            &mode_titles,
            Setting::PunctuationMode,
            target,
        );
        note(
            layout,
            mtm,
            "「进入英文直输」能直接打 hello, world 这样带标点的英文；「先上屏候选再出标点」是 nihao, 出「你好，」；「自动」按拼音切不切得开来定，切不开的 hello 直输、切得开的 nihao 上屏。翻页键不受影响。",
        );
        let english = checkbox(
            mtm,
            "英文模式带补全、纠错、预测",
            Setting::EnglishCandidates,
            target,
        );
        row_aligned_checkbox(layout, &english);
        // 先按最长的组合键写一遍量出高度，sync 再换成配置里实际的键，换了键也不会挤出这一行
        let english_note = note(layout, mtm, &english_assist_note(Some("⌃⌥⇧⌘E")));
        let english_off_in_apps = checkbox(
            mtm,
            "但在终端和代码编辑器里不给",
            Setting::EnglishCandidatesOffInApps,
            target,
        );
        row_sub_checkbox(layout, &english_off_in_apps);
        sub_note(
            layout,
            mtm,
            "终端、iTerm、Warp、Ghostty、VS Code、Cursor、Zed、JetBrains、Xcode 等，那里的候选窗口会挡住应用自己的补全；名单可在配置文件里改。",
        );
        let english_prediction = checkbox(mtm, "预测下一个词", Setting::EnglishPrediction, target);
        row_sub_checkbox(layout, &english_prediction);
        let mixed_english = checkbox(
            mtm,
            "输入拼音时也给英文词候选",
            Setting::MixedEnglishCandidates,
            target,
        );
        row_aligned_checkbox(layout, &mixed_english);
        note(
            layout,
            mtm,
            "hello 给出 hello、compa 补全成 company 这类英文词；不勾后只给中文。与上面的「英文模式带补全、纠错、预测」无关。",
        );
        let chinese_first = checkbox(
            mtm,
            "输入拼音时中文候选排在英文词前面",
            Setting::ChineseFirst,
            target,
        );
        row_sub_checkbox(layout, &chinese_first);
        sub_note(
            layout,
            mtm,
            "勾上后整段输入是英文词时（hello、key）英文词排第二，空格上屏的仍是中文；不勾（缺省）拼音不成立的输入英文词排第一。",
        );
        let emoji = checkbox(mtm, "给 emoji 候选", Setting::EmojiCandidates, target);
        row_aligned_checkbox(layout, &emoji);
        note(
            layout,
            mtm,
            "kaixin 在「开心」后面给出 😄；不勾后候选里不出 emoji。",
        );
        Self {
            learning_language,
            translation_reading,
            page_size,
            scheme,
            wubi,
            wubi_auto_select,
            wubi_sentence,
            wubi_hint,
            traditional,
            english,
            english_note,
            english_off_in_apps,
            english_prediction,
            chinese_first,
            mixed_english,
            emoji,
            languages: languages.to_vec(),
            punctuation,
            punctuation_mode,
        }
    }

    pub fn sync(&self, config: &Config) {
        let general = &config.general;
        select(
            &self.punctuation,
            Some(usize::from(!general.full_width_punctuation)),
        );
        select(
            &self.learning_language,
            if general.learning_language_off() {
                Some(self.languages.len())
            } else {
                self.languages
                    .iter()
                    .position(|l| l.code() == general.learning_language)
            },
        );
        set_checked(&self.translation_reading, general.translation_reading);
        // 译词读音只在显示译文时有意义
        self.translation_reading
            .setEnabled(!general.learning_language_off());
        select(
            &self.punctuation_mode,
            PunctuationMode::ALL
                .iter()
                .position(|m| *m == general.punctuation_mode),
        );
        select(&self.page_size, Some(general.page_size() - 1));
        // 认不出来的方案名 Core 按全拼走，菜单跟着选第一项
        select(
            &self.scheme,
            Some(
                Scheme::ALL
                    .iter()
                    .position(|s| *s == general.scheme())
                    .unwrap_or(0),
            ),
        );
        // 拼音与五笔是两条独立的轴，谁也不置灰谁：两边都开就是混输
        let wubi = general.wubi();
        select(
            &self.wubi,
            Some(wubi.map_or(0, |variant| {
                WUBI_VARIANTS
                    .iter()
                    .position(|v| *v == variant)
                    .map_or(0, |i| i + 1)
            })),
        );
        set_checked(&self.wubi_auto_select, config.wubi.auto_select);
        set_checked(&self.wubi_sentence, config.wubi.sentence);
        set_checked(&self.wubi_hint, config.wubi.hint);
        // 混输下 Core 不做四码自动上屏（四个字母也可能是拼音），整句开着时 Core 也把它停掉，两种情况都置灰说清楚
        self.wubi_auto_select
            .setEnabled(wubi.is_some() && !general.mixed() && !config.wubi.sentence);
        // 整句只在只用形码时做（混输下超过四个字母本来就走拼音整句），混输时置灰
        self.wubi_sentence
            .setEnabled(wubi.is_some() && !general.mixed());
        self.wubi_hint.setEnabled(wubi.is_some());
        set_checked(&self.traditional, general.traditional);
        set_checked(&self.english, general.english_candidates);
        let assist = config
            .shortcut
            .english_assist_keys()
            .map(|combo| combo.label());
        self.english_note
            .setStringValue(&NSString::from_str(&english_assist_note(assist.as_deref())));
        set_checked(
            &self.english_off_in_apps,
            config.apps.has_english_candidates_off(),
        );
        self.english_off_in_apps
            .setEnabled(general.english_candidates);
        // 预测只在英文模式给候选时出（Core 的英文模式跟着 english_candidates 开）
        set_checked(&self.english_prediction, general.english_prediction);
        self.english_prediction
            .setEnabled(general.english_candidates);
        set_checked(&self.chinese_first, general.chinese_first);
        set_checked(&self.mixed_english, general.mixed_english_candidates);
        set_checked(&self.emoji, general.emoji_candidates);
        // 中文优先只在中文模式给英文词时有意义
        self.chinese_first
            .setEnabled(general.mixed_english_candidates);
    }
}

/// 英文辅助勾选框下面的说明；`key` 是开关快捷键的写法，快捷键禁用（冲突或无效）时为 `None`，不提按键。
fn english_assist_note(key: Option<&str>) -> String {
    let base = "关着时按什么上什么，和系统自带的英文一样";
    match key {
        Some(key) => format!("{base}；按 {key} 随时开关。"),
        None => format!("{base}。"),
    }
}
