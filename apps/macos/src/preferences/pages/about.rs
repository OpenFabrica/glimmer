//! 「关于」页：版本与构建、检查更新、许可证、随包数据的来源与署名、隐私说明与反馈方式。
//!
//! 文案集中在这里的常量里，改措辞不用碰布局代码。第三方数据的许可证要求署名在分发物里可见，这一页就是放它的地方。

use glimmer_platform::Config;
use objc2::MainThreadMarker;
use objc2::rc::Retained;
use objc2_app_kit::{NSButton, NSFont, NSTextField};
use objc2_foundation::NSString;

use crate::preferences::controls::{
    NOTE_HEIGHT, button, checkbox, note_full, row_checkbox, set_checked, small_label,
};
use crate::preferences::layout::{Layout, PAGE_PADDING, ROW_HEIGHT};
use crate::preferences::setting::Setting;
use crate::preferences::target::PreferencesTarget;

/// 许可说明，与仓库根目录 `LICENSE` 一致。
pub const LICENSE_NOTE: &str = "自由软件，GPL-3.0-or-later 许可证：可以自由使用、修改与再分发，修改后分发须同样开源。官方渠道免费。";

/// 随包数据的来源与许可证。改数据来源时同步改这里和 `apps/macos/scripts/bundle.sh` 里 `pack` 的署名。
pub const ATTRIBUTIONS: &[(&str, &str)] = &[
    (
        "词库",
        "通用规范汉字表；现代汉语常用词表（liuxilu 校对版）；THUOCL（清华大学自然语言处理实验室，MIT）；读音取自 Unihan（Unicode License v3）。",
    ),
    (
        "语言模型",
        "中文维基百科（CC BY-SA 4.0）与 LCCC（清华大学 CoAI，MIT）语料统计。",
    ),
    (
        "五笔码表",
        "86 版 rime-wubi（LGPL-3.0，Gong Chen、Yu Yuwei；极点五笔码表 Wozy、Chen Xing）；98 版 yanhuacuo/98wubi（LGPL-3.0）；新世纪版 GuoBinyong/wubixinshiji（郭斌勇，LGPL）；字根与编码 王永民（公有领域）。许可证全文随包附带。",
    ),
    ("释义表", "由大语言模型（DeepSeek）生成，微明自建。"),
    ("emoji", "Unicode CLDR annotations（Unicode License v3）。"),
    (
        "英文词表",
        "ESDB / SCOWL（© Kevin Atkinson，按其许可保留版权声明）；CSpell 词典（MIT）。",
    ),
    (
        "词汇等级",
        "The CEFR-J Wordlist Version 1.5（Yukio Tono，Tokyo University of Foreign Studies，cefr-j.org）；Octanove Vocabulary Profile C1/C2（CC BY-SA 4.0）；JLPT 词表（tanos.co.uk，CC BY；经 elzup/jlpt-word-list 整理，MIT）。",
    ),
];

/// 官网。
pub const WEBSITE_URL: &str = "https://glimmerinput.app";

/// 源码与问题反馈。
pub const REPOSITORY_URL: &str = "https://github.com/OpenFabrica/glimmer";

/// 隐私说明。
pub const PRIVACY_NOTE: &str = "微明不上传任何数据。开着云联想或翻译时，光标附近的文字与拼音会发给你在「云服务」页填的 AI 服务商（缺省 DeepSeek）的服务器，不经过作者。检查更新只从 GitHub 下载一份版本清单，不带任何个人信息，可以关掉。「高级」页的输入日志只写在这台电脑的数据目录里，可以关掉或清空。";

/// 「自动检查更新」勾选框下面的说明。
const UPDATE_NOTE: &str = "每 12 小时最多一次，只下载一份几 KB 的版本清单，不带任何个人信息。有新版本只在这里和菜单上提示，下载与安装都要你自己点。";

/// 检查结果那行字最多显示几行（下载进度一行、版本一行、更新日志一行）。
const STATUS_LINES: f64 = 3.0;

/// 反馈方式。
pub const FEEDBACK_NOTE: &str = "遇到问题点「打包日志到桌面」，把生成的 zip 发给作者即可（含日志与配置文件，不含密钥），再附上「复制诊断信息」的内容。缺省日志不含你敲的内容；排查排序问题时作者可能请你在「高级」页临时打开详细日志。";

/// 「关于」页里会变的几个控件：检查更新的结果与按钮、自动检查开关。
pub struct AboutPage {
    /// 检查结果 / 下载进度 / 错误，多行灰字。
    status: Retained<NSTextField>,

    /// 「下载并安装」，有新版本才显示。
    install: Retained<NSButton>,

    /// `[update] check`。
    auto_check: Retained<NSButton>,
}

impl AboutPage {
    /// 把「关于」页的控件摆进 `layout`。
    pub fn build(
        layout: &mut Layout,
        mtm: MainThreadMarker,
        target: &PreferencesTarget,
        version: &str,
        build: &str,
    ) -> Self {
        let title =
            NSTextField::labelWithString(&NSString::from_str(&format!("微明 {version}")), mtm);
        title.setFont(Some(&NSFont::boldSystemFontOfSize(15.0)));
        layout.place(&title, PAGE_PADDING, layout.inner_width(), ROW_HEIGHT);
        layout.next_row(ROW_HEIGHT);
        let build_label = small_label(mtm, &format!("构建 {build}"));
        layout.place(
            &build_label,
            PAGE_PADDING,
            layout.inner_width(),
            ROW_HEIGHT * 0.7,
        );
        layout.next_row(ROW_HEIGHT * 0.7);
        let website = button(mtm, "官网", Setting::OpenWebsite, target);
        let repository = button(mtm, "GitHub", Setting::OpenRepository, target);
        layout.place(&website, PAGE_PADDING, 150.0, ROW_HEIGHT + 4.0);
        layout.place(&repository, PAGE_PADDING + 160.0, 150.0, ROW_HEIGHT + 4.0);
        layout.next_row(ROW_HEIGHT + 4.0);
        note_full(layout, mtm, LICENSE_NOTE);
        layout.end_group();

        // 更新：两个按钮一行，下面留三行给结果 / 进度，再是自动检查的开关
        let check = button(mtm, "检查更新", Setting::CheckUpdate, target);
        let install = button(mtm, "下载并安装", Setting::InstallUpdate, target);
        install.setHidden(true);
        layout.place(&check, PAGE_PADDING, 150.0, ROW_HEIGHT + 4.0);
        layout.place(&install, PAGE_PADDING + 160.0, 150.0, ROW_HEIGHT + 4.0);
        layout.next_row(ROW_HEIGHT + 4.0);
        let status = small_label(mtm, "");
        status.setUsesSingleLineMode(false);
        if let Some(cell) = status.cell() {
            cell.setWraps(true);
        }
        let status_height = NOTE_HEIGHT * STATUS_LINES;
        layout.place(&status, PAGE_PADDING, layout.inner_width(), status_height);
        layout.next_row(status_height);
        let auto_check = checkbox(mtm, "自动检查更新", Setting::UpdateCheck, target);
        row_checkbox(layout, &auto_check);
        note_full(layout, mtm, UPDATE_NOTE);
        layout.end_group();

        let heading = small_label(mtm, "数据来源与署名");
        layout.place(
            &heading,
            PAGE_PADDING,
            layout.inner_width(),
            ROW_HEIGHT * 0.7,
        );
        layout.next_row(ROW_HEIGHT * 0.7);
        for (name, text) in ATTRIBUTIONS {
            note_full(layout, mtm, &format!("{name}：{text}"));
        }
        layout.end_group();

        note_full(layout, mtm, PRIVACY_NOTE);
        note_full(layout, mtm, FEEDBACK_NOTE);
        let open = button(mtm, "打开日志目录", Setting::OpenLogDirectory, target);
        let export = button(mtm, "打包日志到桌面", Setting::ExportLogs, target);
        let copy = button(mtm, "复制诊断信息", Setting::CopyDiagnostics, target);
        layout.place(&open, PAGE_PADDING, 150.0, ROW_HEIGHT + 4.0);
        layout.place(&export, PAGE_PADDING + 160.0, 150.0, ROW_HEIGHT + 4.0);
        layout.place(&copy, PAGE_PADDING + 320.0, 150.0, ROW_HEIGHT + 4.0);
        layout.next_row(ROW_HEIGHT + 4.0);
        Self {
            status,
            install,
            auto_check,
        }
    }

    /// 按配置刷新自动检查的勾选框。
    pub fn sync(&self, config: &Config) {
        set_checked(&self.auto_check, config.update.check);
    }

    /// 显示检查 / 下载的状态；`installable` 为真时露出「下载并安装」。
    pub fn set_update(&self, text: &str, installable: bool) {
        self.status.setStringValue(&NSString::from_str(text));
        self.install.setHidden(!installable);
    }
}
