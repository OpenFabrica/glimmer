//! 「关于」页：版本、检查更新、许可证、随包数据的来源与署名（第三方许可要求署名在分发物里可见）、隐私与反馈。
//! 检查与下载在后台任务里跑（`run_check` / `run_download`），结果经消息回到根组件。

use std::path::PathBuf;

use glimmer_update::{
    Available, CHECK_INTERVAL, STAMP_FILE, Target, UpdateConfig, UpdateError, check_blocking,
    download_blocking, touch,
};
use windows_reactor::*;

use crate::panel::controls::{field, note, page};
use crate::panel::update_status::UpdateStatus;
use crate::panel::{Message, Settings};

/// 下载的安装包放在 `%APPDATA%\Glimmer\updates\`。
const DOWNLOAD_DIR: &str = "updates";

/// 「自动检查更新」开关下面的说明。
const UPDATE_NOTE: &str = "打开设置时查，每 12 小时最多一次，只下载一份几 KB 的版本清单，不带任何个人信息。有新版本只在这里提示，下载与安装都要你自己点。";

/// 标记文件 `%APPDATA%\Glimmer\update-check`：只看修改时间。
fn stamp_path() -> Option<PathBuf> {
    glimmer_platform::dirs::user_dir().map(|dir| dir.join(STAMP_FILE))
}

/// 距上次自动检查是否已过 12 小时（没有数据目录也算到点，查了不记而已）。
pub(crate) fn check_due() -> bool {
    stamp_path().is_none_or(|stamp| glimmer_update::check_due(&stamp, CHECK_INTERVAL))
}

/// 后台跑一次检查：拉清单、与本程序版本比。成功就记标记。
pub(crate) fn run_check(config: &UpdateConfig) -> Result<Option<Available>, String> {
    let target =
        Target::current().ok_or_else(|| "这个架构没有发布安装包，请到官网下载".to_owned())?;
    let outcome =
        check_blocking(config, env!("GLIMMER_VERSION"), target).map_err(|e| describe(&e))?;
    if let Some(stamp) = stamp_path() {
        touch(&stamp);
    }
    Ok(outcome)
}

/// 后台下载并校验安装包，返回它的路径。
pub(crate) fn run_download(update: &Available) -> Result<PathBuf, String> {
    let dir = glimmer_platform::dirs::user_dir()
        .ok_or_else(|| "找不到用户数据目录（APPDATA），没法下载".to_owned())?
        .join(DOWNLOAD_DIR);
    let received = std::sync::atomic::AtomicU64::new(0);
    download_blocking(&update.asset, &dir, &received).map_err(|e| describe(&e))
}

/// 检查更新的错误换成给用户看的中文；网络库的原话保留，方便排查。
fn describe(error: &UpdateError) -> String {
    match error {
        UpdateError::BadCurrentVersion(version) => format!("本机版本号「{version}」认不出来"),
        UpdateError::Http(error) if error.is_timeout() => {
            "连接超时。检查网络或代理：设置程序不读终端里的代理变量".to_owned()
        }
        UpdateError::Http(error) => format!("网络请求失败：{error}"),
        UpdateError::Feed(error) => format!("版本清单读不懂：{error}"),
        UpdateError::Io(error) => format!("读写文件失败：{error}"),
        UpdateError::MissingChecksum(file) => format!("清单里没有 {file} 的校验值，不敢自动安装"),
        UpdateError::Checksum { file, .. } => {
            format!("{file} 校验不符（下载不完整或被篡改），已删掉，请重试")
        }
    }
}

/// 「关于」页状态行的文字，与要不要显示「下载并安装」。
fn update_line(status: &UpdateStatus) -> (String, bool) {
    match status {
        UpdateStatus::Idle => (String::new(), false),
        UpdateStatus::Checking { .. } => ("正在检查更新…".to_owned(), false),
        UpdateStatus::UpToDate => (
            format!("已是最新版本（{}）", env!("GLIMMER_VERSION")),
            false,
        ),
        UpdateStatus::Available(update) => (available_text(update), true),
        UpdateStatus::Downloading(update) => (format!("正在下载 {}…", update.asset.file), false),
        UpdateStatus::Downloaded(path) => (
            format!(
                "安装包已下载并通过校验，已打开安装器：{}。装完后已打开的程序要重启才用新版本",
                path.display()
            ),
            true,
        ),
        UpdateStatus::Failed(message, update) => (format!("失败：{message}"), update.is_some()),
    }
}

/// 「有新版本 0.1.0-alpha.7（2026-09-20）：第一条更新日志…」。
fn available_text(update: &Available) -> String {
    let mut text = format!("有新版本 {}", update.label());
    if let Some(first) = update.notes.first() {
        text.push('：');
        text.push_str(first);
        if update.notes.len() > 1 {
            text.push_str(&format!("（还有 {} 条，见官网）", update.notes.len() - 1));
        }
    }
    text
}

pub(crate) const WEBSITE_URL: &str = "https://glimmerinput.app";

pub(crate) const REPOSITORY_URL: &str = "https://github.com/OpenFabrica/glimmer";

/// 与仓库根 `LICENSE` 一致。
const LICENSE_NOTE: &str = "自由软件，GPL-3.0-or-later 许可证：可以自由使用、修改与再分发，修改后分发须同样开源。官方渠道免费。";

/// 与 macOS「关于」页一致。
const ATTRIBUTIONS: &[(&str, &str)] = &[
    (
        "词库",
        "通用规范汉字表；现代汉语常用词表（liuxilu 校对版）；THUOCL（清华大学自然语言处理实验室，MIT）；补充常用词白名单 CC-CEDICT（MDBG，CC BY-SA 4.0）与 jieba 词表（MIT）；读音取自 Unihan（Unicode License v3）。",
    ),
    (
        "语言模型",
        "中文维基百科（CC BY-SA 4.0）与 LCCC（清华大学 CoAI，MIT）语料统计。",
    ),
    ("释义表", "由大语言模型（DeepSeek）生成，微明自建。"),
    ("emoji", "Unicode CLDR annotations（Unicode License v3）。"),
    (
        "英文词表",
        "ESDB / SCOWL（© Kevin Atkinson，按其许可保留版权声明）；CSpell 词典（MIT）。",
    ),
    (
        "词汇等级",
        "CEFR-J Wordlist v1.5（Yukio Tono，cefr-j.org）；Octanove Vocabulary Profile C1/C2（CC BY-SA 4.0）；JLPT 词表（tanos.co.uk，CC BY）。",
    ),
];

const PRIVACY_NOTE: &str = "微明不上传任何数据。开着云联想时，光标附近的文字与拼音会发给你在「云服务」页填的 AI 服务商（缺省 DeepSeek）的服务器，不经过作者。检查更新只从 GitHub 下载一份版本清单，不带任何个人信息，可以关掉。「高级」页的输入日志只写在本机，可以关掉或清空。";

const FEEDBACK_NOTE: &str = "遇到问题点「打包日志到桌面」，把生成的 zip 发给作者即可（含三个进程的日志与配置文件，不含密钥）。缺省日志不含你敲的内容；排查排序问题时作者可能请你在「高级」页临时打开详细日志。";

pub(crate) fn view(settings: &Settings, context: &mut ViewContext<Settings>) -> View {
    let mut attributions: Vec<View> = Vec::with_capacity(ATTRIBUTIONS.len());
    for (name, text) in ATTRIBUTIONS {
        attributions.push(note(&format!("{name}：{text}")));
    }
    // 更新一行：「检查更新」常在，「下载并安装」有新版本才出现，后面跟状态文字
    let (status, installable) = update_line(&settings.update);
    let mut update_row: Vec<View> = vec![
        Button::new()
            .on_click(context.message(Message::CheckUpdate))
            .content("检查更新"),
    ];
    if installable {
        update_row.push(
            Button::new()
                .on_click(context.message(Message::InstallUpdate))
                .content("下载并安装"),
        );
    }
    update_row.push(
        TextBlock::new()
            .text(status)
            .text_wrapping(TextWrapping::Wrap)
            .into(),
    );
    let body = StackPanel::new().spacing(12.0).children([
        TextBlock::new()
            // GLIMMER_VERSION 由 build.rs 给：-dev 版接 git 短哈希
            .text(concat!("微明 Windows ", env!("GLIMMER_VERSION")))
            .font_size(16.0)
            .font_weight(FontWeight::SEMI_BOLD)
            .into(),
        // GLIMMER_BUILD 由 build.rs 从 git 取
        note(&format!(
            "构建 {}",
            option_env!("GLIMMER_BUILD").unwrap_or("本地构建")
        )),
        StackPanel::new()
            .orientation(Orientation::Horizontal)
            .spacing(12.0)
            .children((
                Button::new()
                    .on_click(context.message(Message::OpenWebsite))
                    .content("官网"),
                Button::new()
                    .on_click(context.message(Message::OpenRepository))
                    .content("GitHub"),
                Button::new()
                    .on_click(context.message(Message::OpenDataDir))
                    .content("打开数据目录"),
                Button::new()
                    .on_click(context.message(Message::OpenLogDir))
                    .content("打开日志目录"),
                Button::new()
                    .on_click(context.message(Message::ExportLogs))
                    .content("打包日志到桌面"),
            )),
        note(LICENSE_NOTE),
        StackPanel::new()
            .orientation(Orientation::Horizontal)
            .spacing(12.0)
            .keyed_children(
                update_row
                    .into_iter()
                    .enumerate()
                    .map(|(index, view)| KeyedView::new(index.to_string(), view)),
            ),
        field(
            "自动检查更新",
            UPDATE_NOTE,
            ToggleSwitch::new()
                .is_on(settings.config.update.check)
                .on_toggled(context.callback(Message::AutoUpdateCheck)),
        ),
        TextBlock::new()
            .text("数据来源与署名")
            .font_weight(FontWeight::SEMI_BOLD)
            .into(),
        StackPanel::new().spacing(6.0).keyed_children(
            attributions
                .into_iter()
                .enumerate()
                .map(|(index, view)| KeyedView::new(index.to_string(), view)),
        ),
        note(PRIVACY_NOTE),
        note(FEEDBACK_NOTE),
    ]);
    page("关于", body)
}
