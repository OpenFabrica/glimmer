//! 「快捷键」页：中英切换、翻页、前缀、译词上屏 / 删候选 / 翻译选中文字 / 开关英文辅助的组合键。

use glimmer_core::ModeKeys;
use glimmer_platform::{Config, PAGE_KEY_OPTIONS};
use objc2::MainThreadMarker;
use objc2::rc::Retained;
use objc2_app_kit::{NSButton, NSPopUpButton};

use crate::preferences::controls::{
    button, checkbox, note, note_full, page_keys_label, row_checkbox, row_popup, row_recorder,
    select, set_checked,
};
use crate::preferences::key_recorder::KeyRecorder;
use crate::preferences::layout::{Layout, PAGE_PADDING, ROW_HEIGHT};
use crate::preferences::setting::Setting;
use crate::preferences::target::PreferencesTarget;

pub struct ShortcutsPage {
    /// 中英文切换的组合键。
    mode_switch: Retained<KeyRecorder>,

    /// 翻页键对。
    page_keys: Retained<NSPopUpButton>,

    /// 表达式模式键。
    expression: Retained<NSPopUpButton>,

    /// 问字模式键。
    question: Retained<NSPopUpButton>,

    /// 没在组句时敲 `?` 也进问字。
    question_mark: Retained<NSButton>,

    /// 上屏第一个译词的修饰键。
    translation: Retained<KeyRecorder>,

    /// 上屏第二个译词的修饰键。
    translation_second: Retained<KeyRecorder>,

    /// 删除候选的修饰键。
    delete_candidate: Retained<KeyRecorder>,

    /// 翻译选中文字的组合键。
    translate_selection: Retained<KeyRecorder>,

    /// 开关英文辅助（英文模式的补全、纠错、预测）的组合键。
    english_assist: Retained<KeyRecorder>,
}

impl ShortcutsPage {
    pub fn build(layout: &mut Layout, mtm: MainThreadMarker, target: &PreferencesTarget) -> Self {
        let mode_switch = row_recorder(
            layout,
            mtm,
            "切换中文 / 英文",
            Setting::ModeSwitchKeys,
            false,
            target,
        );
        note(
            layout,
            mtm,
            "单击 Shift，或修饰键 + 字母。单击需在半秒内松开；Shift + 字母仍输入大写。切换时未完成的输入原样上屏，当前模式看菜单栏「中 / 英」。",
        );
        layout.end_group();
        let page_key_titles: Vec<String> = PAGE_KEY_OPTIONS
            .iter()
            .map(|k| page_keys_label(k))
            .collect();
        let page_keys = row_popup(
            layout,
            mtm,
            "翻页键",
            &page_key_titles,
            Setting::PageKeys,
            target,
        );
        note(
            layout,
            mtm,
            "选「，  。」时组句中敲逗号句号是翻页，不再是上屏加标点。",
        );
        let key_titles: Vec<String> = ModeKeys::CANDIDATES.iter().map(char::to_string).collect();
        let expression = row_popup(
            layout,
            mtm,
            "表达式模式键",
            &key_titles,
            Setting::ExpressionKey,
            target,
        );
        let question = row_popup(
            layout,
            mtm,
            "问字模式键",
            &key_titles,
            Setting::QuestionKey,
            target,
        );
        note(
            layout,
            mtm,
            "这两个字母开头进模式：v1+2 出 3，usangemu 问「三个木」（需要云服务），u4e00 出对应的字符。两个键不能相同。",
        );
        let question_mark = checkbox(
            mtm,
            "没在输入拼音时敲 ? 也进入问字",
            Setting::QuestionMark,
            target,
        );
        row_checkbox(layout, &question_mark);
        note(
            layout,
            mtm,
            "勾上后 ? 先进问字（中英文模式都行），后面跟字母才是问题，跟空格、回车等其他键时还原成问号；不勾问号就是问号。",
        );
        layout.end_group();
        let translation = row_recorder(
            layout,
            mtm,
            "上屏第一个译词",
            Setting::TranslationKeys,
            true,
            target,
        );
        let translation_second = row_recorder(
            layout,
            mtm,
            "上屏第二个译词",
            Setting::TranslationSecondKeys,
            true,
            target,
        );
        note(
            layout,
            mtm,
            "按住修饰键再按候选序号，上屏的是候选右侧的译词而不是中文；候选有两个译词时第二组键上屏后一个。两组不能相同。",
        );
        layout.end_group();
        let delete_candidate = row_recorder(
            layout,
            mtm,
            "删除候选",
            Setting::DeleteCandidateKeys,
            true,
            target,
        );
        note(
            layout,
            mtm,
            "按住修饰键再按候选序号：自己造的词、云端选过的词整个删掉；词库里的词清掉对它的学习记录，回到原来的排序。组句中要打感叹号先把词上屏。",
        );
        layout.end_group();
        let translate_selection = row_recorder(
            layout,
            mtm,
            "翻译选中的文字",
            Setting::TranslateSelectionKeys,
            false,
            target,
        );
        note(
            layout,
            mtm,
            "在应用里选中一段文字再按这个键，译文（学习语言）出现在候选窗口：回车替换选中的文字，Esc 保留原文。需要开着云服务。",
        );
        layout.end_group();
        let english_assist = row_recorder(
            layout,
            mtm,
            "开关英文辅助",
            Setting::EnglishAssistKeys,
            false,
            target,
        );
        note(
            layout,
            mtm,
            "随时开关英文模式的补全、纠错、预测，开着时菜单栏显示「英+」。不能与中英切换、翻译键相同。",
        );
        layout.end_group();
        note_full(
            layout,
            mtm,
            "改快捷键：点击右边的按钮，再按新的组合键；中英切换也可单击 Shift。Esc 取消。组合键须带 ⌃ ⌥ ⇧ ⌘，避开系统和应用常用键；中英切换不能与翻译键相同。",
        );
        let reset = button(mtm, "恢复默认快捷键", Setting::ResetShortcuts, target);
        layout.place(&reset, PAGE_PADDING, 160.0, ROW_HEIGHT + 4.0);
        layout.next_row(ROW_HEIGHT + 4.0);
        layout.end_group();
        note_full(
            layout,
            mtm,
            "组句中固定的键（不可改）：空格上屏首选，1–9 选词，回车原样上屏，Esc 清空；⌥⌫ 删一个音节，⌘⌫ 删到开头；\
             ⌥← / ⌥→ 按音节跳光标，⌘← / ⌘→ 到开头 / 末尾；上 / 下移动高亮，PageUp / PageDown 与 ⇧Tab 翻页；\
             Tab 接受云端整句补全（没有就翻页）；半角标点进入英文直输段。",
        );
        Self {
            mode_switch,
            page_keys,
            expression,
            question,
            question_mark,
            translation,
            translation_second,
            delete_candidate,
            translate_selection,
            english_assist,
        }
    }

    pub fn sync(&self, config: &Config) {
        let mode_switch = config.shortcut.mode_switch;
        let label = if config.shortcut.mode_switch_keys().is_some() {
            mode_switch.label()
        } else {
            "快捷键冲突或无效，请重新录制".to_owned()
        };
        self.mode_switch.show(&mode_switch.key_string(), &label);
        let (previous, next) = config.general.page_keys();
        let pair = format!("{previous}{next}");
        select(
            &self.page_keys,
            PAGE_KEY_OPTIONS.iter().position(|k| *k == pair),
        );
        let keys = config.shortcut.mode.sanitized();
        select(
            &self.expression,
            ModeKeys::CANDIDATES
                .iter()
                .position(|k| *k == keys.expression),
        );
        select(
            &self.question,
            ModeKeys::CANDIDATES
                .iter()
                .position(|k| *k == keys.question),
        );
        set_checked(&self.question_mark, keys.question_mark);
        let (first, second) = config.shortcut.translation_keys();
        self.translation.show(&first.key(), &first.label());
        self.translation_second.show(&second.key(), &second.label());
        let delete = config.shortcut.delete_keys();
        self.delete_candidate.show(&delete.key(), &delete.label());
        let translate = config.shortcut.translate_selection;
        self.translate_selection
            .show(&translate.key_string(), &translate.label());
        // 与中英切换一样：冲突或无效时 Core 侧已禁用，按钮上说清楚
        let assist = config.shortcut.english_assist;
        let label = if config.shortcut.english_assist_keys().is_some() {
            assist.label()
        } else {
            "快捷键冲突或无效，请重新录制".to_owned()
        };
        self.english_assist.show(&assist.key_string(), &label);
    }
}
