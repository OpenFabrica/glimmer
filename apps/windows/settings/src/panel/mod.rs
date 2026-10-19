//! 设置窗口根组件：左侧导航 + 右侧当前分节页。每改一项就原地写回 `config.toml`（保留注释）再重读，
//! 界面始终反映文件内容；Server 每秒看 mtime 热加载。
//! 状态在这里，消息在 [`message`]，生命周期在 [`component`]，表单零件在 [`controls`]，各页在 [`pages`]。

mod cloud_status;
mod component;
mod controls;
mod message;
mod pages;
mod update_status;

use std::path::{Path, PathBuf};

use glimmer_platform::Config;
use windows_reactor::*;

use self::cloud_status::CloudStatus;
pub(crate) use self::message::Message;
use self::pages::{
    about, advanced, candidates, cloud, dictionaries, fuzzy, general, shortcut, usage,
};
use self::update_status::UpdateStatus;

/// 左侧标签固定宽度，让各行控件对齐。
const LABEL_WIDTH: f64 = 140.0;

/// 设置窗口状态。
pub(crate) struct Settings {
    /// 当前配置，每次改动后从盘上重读。
    pub(super) config: Config,

    /// `config.toml` 路径。
    path: PathBuf,

    /// 当前导航分节 tag。
    page: String,

    /// 云服务「测试连接」的状态。
    cloud_status: CloudStatus,

    /// 「关于」页检查更新的状态。
    update: UpdateStatus,

    /// 最近一次词库操作的结果，显示在词库页。
    dictionary_status: String,

    /// 系统里的字族名（DirectWrite），「字体」框的提示用。
    families: Vec<String>,

    /// 「字体」框里正在敲的文字；`None` 显示配置里的值。
    font_query: Option<String>,

    /// 「背景颜色」框里还没敲完（不是合法颜色）的文字；`None` 显示配置里的值。
    background_draft: Option<String>,

    /// 「高亮颜色」框里还没敲完的文字；`None` 显示配置里的值。
    highlight_draft: Option<String>,
}

impl Settings {
    /// `%APPDATA%\Glimmer\config.toml`；取不到 `APPDATA` 退回工作目录。
    fn config_path() -> PathBuf {
        glimmer_platform::dirs::config_path().unwrap_or_else(|| PathBuf::from("config.toml"))
    }

    /// 配置文件不在就写出模板：这个账户下 Server 还没跑过时，保存与「在记事本中打开」都要有文件。
    fn ensure_config_file(path: &Path) {
        if let Err(error) = Config::write_template_if_missing(path) {
            crate::log::warn(format!("写配置模板失败: {error}"));
        }
    }

    /// 数据目录 `%APPDATA%\Glimmer`。
    fn data_dir(&self) -> &Path {
        self.path.parent().unwrap_or_else(|| Path::new("."))
    }

    /// 落盘一个配置值再重读。失败只打印。
    fn save(&mut self, section: &str, key: &str, value: impl Into<toml_edit::Value>) {
        if let Err(error) = Config::set_value(&self.path, section, key, value) {
            crate::log::warn(format!("保存 [{section}] {key} 失败: {error}"));
            return;
        }
        self.reload();
    }

    /// 落盘一个字符串数组再重读。
    fn save_array(&mut self, section: &str, key: &str, values: &[String]) {
        if let Err(error) = Config::set_array(&self.path, section, key, values) {
            crate::log::warn(format!("保存 [{section}] {key} 失败: {error}"));
            return;
        }
        self.reload();
    }

    /// 颜色框的文字变了：空或合法颜色就落盘并清掉草稿，否则只记成草稿（敲到一半不写坏配置）。
    fn save_color(&mut self, key: &str, text: &str) -> Option<String> {
        let text = text.trim();
        if text.is_empty() || glimmer_render::Color::from_hex(text).is_some() {
            self.save("general", key, text);
            None
        } else {
            Some(text.to_owned())
        }
    }

    fn reload(&mut self) {
        if let Ok(config) = Config::load(&self.path) {
            self.config = config;
        }
    }

    fn page_content(&self, context: &mut ViewContext<Self>) -> View {
        match self.page.as_str() {
            "candidates" => candidates::view(self, context),
            "shortcut" => shortcut::view(self, context),
            "cloud" => cloud::view(self, context),
            "fuzzy" => fuzzy::view(self, context),
            "dictionaries" => dictionaries::view(self, context),
            "usage" => usage::view(self, context),
            "advanced" => advanced::view(self, context),
            "about" => about::view(self, context),
            _ => general::view(self, context),
        }
    }
}
