mod apps;
mod candidate_renderer;
mod color_scheme;
mod dictionaries;
mod general;
mod key_combo;
mod layout_mode;
mod log_level;
mod mode_switch;
mod model;
mod modifiers;
mod preedit_mode;
mod scheme;
mod shortcut;
mod status_bar;
mod theme_mode;
mod wubi;

use std::path::Path;

use glimmer_core::FuzzyRules;
use glimmer_predict::PredictConfig;
use glimmer_update::UpdateConfig;
use serde::{Deserialize, Serialize};
use toml_edit::DocumentMut;

use crate::error::ConfigError;

pub use apps::{
    AppsConfig, DEFAULT_ENGLISH_CANDIDATES_OFF, DEFAULT_ENGLISH_CANDIDATES_OFF_LINUX,
    DEFAULT_ENGLISH_CANDIDATES_OFF_MACOS, DEFAULT_ENGLISH_CANDIDATES_OFF_WINDOWS,
};
pub use candidate_renderer::CandidateRenderer;
pub use color_scheme::ColorScheme;
pub use dictionaries::{DEFAULT_DOMAINS, DictionariesConfig};
pub use general::{
    DEFAULT_FONT_SIZE, DEFAULT_PAGE_KEYS, GeneralConfig, LEARNING_LANGUAGE_OFF, MAX_FONT_SIZE,
    MAX_PAGE_SIZE, MIN_FONT_SIZE, PAGE_KEY_OPTIONS,
};
pub use key_combo::KeyCombo;
pub use layout_mode::LayoutMode;
pub use log_level::LogLevel;
pub use mode_switch::ModeSwitch;
pub use model::LocalModelConfig;
pub use modifiers::Modifiers;
pub use preedit_mode::PreeditMode;
pub use scheme::{Scheme, scheme_label};
pub use shortcut::ShortcutConfig;
pub use status_bar::StatusBarConfig;
pub use theme_mode::ThemeMode;
pub use wubi::WubiConfig;

/// 用户配置文件（TOML）。所有平台同一份格式，缺省值全部在各分节的 `Default` 里。
///
/// 配置文件是唯一事实源：菜单、设置窗口、手改文件三个入口都只写这个文件，再由壳热加载。
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    /// 常规：学习语言、每页候选数、翻页键、外观。
    pub general: GeneralConfig,

    /// 自定义短语；保存和读取均检查位置冲突。
    #[serde(deserialize_with = "deserialize_phrases")]
    pub custom_phrases: Vec<glimmer_core::CustomPhrase>,

    /// 快捷键：前缀模式键（表达式 / 问字）与上屏译词的修饰键组合。
    pub shortcut: ShortcutConfig,

    /// 模糊音开关。
    pub fuzzy: FuzzyRules,

    /// 附加词库开关。
    pub dictionaries: DictionariesConfig,

    /// 按应用改行为（哪些应用里英文模式不给候选）。
    pub apps: AppsConfig,

    /// 云联想。
    pub predict: PredictConfig,

    /// 悬浮状态条（桌面上常驻、可拖动的中 / 英浮窗）。
    pub status_bar: StatusBarConfig,

    /// 本地整句模型。
    pub model: LocalModelConfig,

    /// 五笔的行为选项（方案在 `[general] wubi` 里选）。
    pub wubi: WubiConfig,

    /// 检查更新：启动时自动查一次的开关与清单地址。
    pub update: UpdateConfig,
}

fn deserialize_phrases<'de, D: serde::Deserializer<'de>>(
    deserializer: D,
) -> Result<Vec<glimmer_core::CustomPhrase>, D::Error> {
    let phrases = Vec::<glimmer_core::CustomPhrase>::deserialize(deserializer)?;
    glimmer_core::custom_phrase::validate_phrases(&phrases).map_err(serde::de::Error::custom)?;
    Ok(phrases)
}

/// 模板的 `[apps]` 一节（Linux）：应用按 fcitx5 认到的名字（X11 是 WM_CLASS，Wayland 是 app_id）。
/// 名单要与 [`DEFAULT_ENGLISH_CANDIDATES_OFF`] 一致，测试 `template_parses_to_defaults` 会核对。
#[cfg(not(any(windows, target_os = "macos")))]
macro_rules! template_apps {
    () => {
        r#"[apps]
# 按应用改行为，条目是 fcitx5 认到的应用名（X11 是 WM_CLASS，Wayland 是 app_id；`*` 结尾按前缀匹配，不区分大小写）
# 英文模式下不给候选的应用：终端与代码编辑器里候选窗口会挡住应用自己的补全，vim 里 Tab 和方向键也另有含义。设成 [] 就处处都给
english_candidates_off = [
  "konsole", "org.kde.konsole", "yakuake", "gnome-terminal-server", "org.gnome.terminal", "xterm",
  "alacritty", "kitty", "foot", "wezterm", "org.wezfurlong.wezterm", "com.mitchellh.ghostty", "tilix", "xfce4-terminal",
  "code", "code-oss", "codium", "code-url-handler", "cursor", "jetbrains-*", "dev.zed.zed", "sublime_text", "neovide",
]
"#
    };
}

/// 模板的 `[apps]` 一节（macOS）：应用按 bundle identifier 认。名单要与 [`DEFAULT_ENGLISH_CANDIDATES_OFF`] 一致，
/// 测试 `template_parses_to_defaults` 会核对。用宏而不是常量，是因为 `concat!` 只收字面量。
#[cfg(target_os = "macos")]
macro_rules! template_apps {
    () => {
        r#"[apps]
# 按应用改行为，条目是 bundle identifier（`*` 结尾按前缀匹配）。开着「详细日志」时切到一个应用会把它的 bundle identifier 记进日志
# 英文模式（Caps Lock）下不给候选的应用：终端与代码编辑器里候选窗口会挡住应用自己的补全，vim 里 Tab 和方向键也另有含义。设成 [] 就处处都给
english_candidates_off = [
  "com.apple.Terminal", "com.googlecode.iterm2", "dev.warp.Warp-Stable", "com.mitchellh.ghostty", "io.alacritty", "net.kovidgoyal.kitty",
  "com.microsoft.VSCode", "com.todesktop.230313mzl4w4u92", "dev.zed.Zed", "com.jetbrains.*", "org.vim.MacVim", "com.sublimetext.*",
  "com.apple.dt.Xcode", "com.neovide.neovide",
]
"#
    };
}

/// 模板的 `[apps]` 一节（Windows）：应用按宿主进程的 exe 文件名认。名单要与 [`DEFAULT_ENGLISH_CANDIDATES_OFF`] 一致。
#[cfg(windows)]
macro_rules! template_apps {
    () => {
        r#"[apps]
# 按应用改行为，条目是应用进程的 exe 文件名（`*` 结尾按前缀匹配）。Server 开着 debug 日志时每开一个会话会把 exe 名记进日志
# 英文模式（Caps Lock）下不给候选的应用：终端与代码编辑器里候选窗口会挡住应用自己的补全，vim 里 Tab 和方向键也另有含义。设成 [] 就处处都给
# 经典控制台（cmd / PowerShell）的窗口属于 conhost.exe，Windows Terminal 是 WindowsTerminal.exe
english_candidates_off = [
  "conhost.exe", "WindowsTerminal.exe", "alacritty.exe", "wezterm-gui.exe", "mintty.exe",
  "Code.exe", "Code - Insiders.exe", "Cursor.exe", "zed.exe",
  "idea64.exe", "pycharm64.exe", "clion64.exe", "rustrover64.exe", "goland64.exe", "rider64.exe", "webstorm64.exe", "phpstorm64.exe", "datagrip64.exe",
  "devenv.exe", "sublime_text.exe", "notepad++.exe", "gvim.exe", "neovide.exe",
]
"#
    };
}

/// 模板 `[shortcut]` 一节里的 macOS 修饰键组合与中英切换键。
#[cfg(not(windows))]
macro_rules! template_shortcut_keys {
    () => {
        r#"# 数字键配这些修饰键上屏候选的译词：translation 第一个译词，translation_second 第二个（候选右侧有两个译词时）
# 任意修饰键组合（option / shift / control / command 用 + 连），偏好设置里点按钮录制；别用 control+数字（系统切桌面）和 command+数字（应用切标签页）
translation = "option"
translation_second = "shift+option"
# 把应用里选中的文字译成学习语言（要开着云服务）：译文先出现在候选窗口，回车替换选中的文字，Esc 保留原文
# 修饰键 + 一个字母或数字，任意组合；避开 ⌘T 这类应用常用键
translate_selection = "control+option+t"
# macOS 中英文切换：shift 为单击 Shift，也可写 control+option+m 等修饰键 + 字母组合，不能与翻译键相同
mode_switch = "shift"
# 数字键配这些修饰键删掉候选：用户词（云端选过的、自动造的）整个删掉，词库里的词清掉对它的学习记录。组句中要打感叹号先把词上屏
delete_candidate = "shift"
"#
    };
}

/// 模板 `[shortcut]` 一节里的修饰键组合（Windows 键名：alt / ctrl / win，读回来与 macOS 的 option / control / command 等价）。
#[cfg(windows)]
macro_rules! template_shortcut_keys {
    () => {
        r#"# 数字键配这些修饰键上屏候选的译词：translation 第一个译词，translation_second 第二个（候选右侧有两个译词时）
# 任意修饰键组合（alt / shift / ctrl / win 用 + 连）。Alt+数字会被 Windows 当菜单快捷键截走，缺省用 Ctrl；组句时才拦，不打字时照常放行给应用
translation = "ctrl"
translation_second = "shift+ctrl"
# 把应用里选中的文字译成学习语言（要开着云服务）：Windows 上还没接
translate_selection = "ctrl+alt+t"
# 数字键配这些修饰键删掉候选：用户词（云端选过的、自动造的）整个删掉，词库里的词清掉对它的学习记录。组句中要打感叹号先把词上屏
delete_candidate = "shift"
"#
    };
}

/// 首次运行写出的模板：默认值全部列出并注释，用户改一处即可。`[shortcut]` 的修饰键与 `[apps]` 分平台，
/// 见 [`template_shortcut_keys!`] / [`template_apps!`]。
pub const TEMPLATE: &str = concat!(
    r#"# 微明输入法配置。保存后自动生效；也可以在菜单栏的输入法菜单里改。

[general]
# 学习语言（en 英语 / ja 日语 / es 西班牙语 / off 不显示译文）：候选旁显示哪种语言的译文，要有对应的释义表才生效
learning_language = "en"
# 译词带不带读音：英语是美式音标（develop (dɪˈveləp)）、日语是假名；false 只显示译词
translation_reading = true
# 每页候选数（1–9）
page_size = 9
# 翻页键对：前一个上一页、后一个下一页。可选 "[]" 或 ",."；选 ",." 的话组句中敲逗号句号是翻页而不是上屏加标点
page_keys = "[]"
# 候选窗口外观：system 跟随系统 / light 浅色 / dark 深色
theme = "system"
# 候选窗口配色：default 默认 / blue 海蓝 / paper 暖纸 / rose 樱粉 / mono 素灰；每套有浅色、深色两版，跟着 theme 切换。只对微明渲染器生效
color_scheme = "default"
# 自定义候选窗口背景色，写成 #RRGGBB（如 #FAF6EC，要加引号）；空为用配色的。设了就不分浅色深色，字色按背景深浅自动选
background_color = ""
# 自定义当前候选的高亮底色，#RRGGBB 或带不透明度的 #RRGGBBAA；空为用配色的。高亮上的字色按底色深浅自动选
highlight_color = ""
# 候选窗口排布：vertical 竖排 / horizontal 横排（横排只给高亮候选显示译文）
layout = "vertical"
# 候选窗口由谁绘制：glimmer 微明渲染器（各平台一致，主题走它）/ system 系统原生绘制（渲染器有问题时的退路）
renderer = "glimmer"
# 候选窗口字体（字族名，如 "LXGW WenKai"）；空为系统字体。只对微明渲染器生效，没装这个字体时自动回到系统字体
font = ""
# 候选窗口文字大小（点，12–28，缺省 16）：候选词用这个字号，译文与序号按比例跟着变。只对微明渲染器生效
font_size = 16
# 组句中的拼音显示在哪：both 行内和候选窗口 / inline 只在行内 / window 只在候选窗口（应用里不放 marked text）
preedit = "both"
# 英文模式（Caps Lock 亮着）是否给英文候选：Tab 或方向键选词，空格、回车、标点仍原样上屏敲的字母；false 就是纯直通
english_candidates = true

# 繁体输出模式。开启后上屏繁体，不影响词库和个人词频的简体记录。
traditional = false
# 中文模式下整段输入是英文词时（hello / key）是否让中文候选排第一、英文词第二；缺省 false：拼音不像话的输入英文词排第一
chinese_first = false
# 中文模式下是否给英文词候选：整段输入是英文词（hello）或英文词开头（compa → company）时列出英文词；
# 与上面的 english_candidates 无关（那个管 Caps Lock 亮着的英文模式）
mixed_english_candidates = true
# 是否给 emoji 候选（kaixin → 😄）；false 不出 emoji
emoji_candidates = true
# 中文模式下（没在组句时）敲的标点转全角：, . ? ! : ; ( ) 等，数字后面的 . 保持半角。Windows 上悬浮状态条的「，。」格可以点着切；macOS 在偏好设置中选择默认中文标点模式
full_width_punctuation = true
# 英文模式下的同一件事，中英各记一份，状态条切的是当前模式那份；只有 Windows 用
english_full_width_punctuation = false
# 正在打拼音时敲 , . ? - 这些半角标点怎么办：raw 标点跟着拼音进英文直输段（hello, world 整段原样上屏）/
# commit 先把首选上屏再出标点（nihao, → 你好，）/ auto 缓冲区切不成拼音（hello）才直输，切得成（nihao）就上屏
punctuation_mode = "raw"
# 拼音方案：留空或 pinyin 全拼（缺省）/ xiaohe 小鹤双拼 / ziranma 自然码 / microsoft 微软双拼 / sogou 搜狗双拼 /
# xiaolang 小浪双拼 / abc 智能ABC双拼 / zhuyin 大千注音 / none 关（只用五笔，见下面的 wubi）。
# 双拼与注音下 v / u / i 都是按键，表达式与问字换成 Shift+V / Shift+U；微软、搜狗方案的 ; 键是 ing
scheme = ""
# 五笔：留空为关；86 / 98 / xsj（新世纪）选版本（要有对应的码表文件）。**与上面的拼音方案同时开着就是混输**——
# 两边都出候选，编码打全的五笔词在最前、其次拼音、只打了编码前缀的垫后（五笔打不出的字直接打拼音）。
# 只用五笔的话把 scheme 写成 none：那时 v / u 都是编码键，z 开头是拼音反查，敲满四码可以自动上屏；
# 混输下没有这三样（z 是声母，四个字母同样可能是拼音）。行为选项在下面的 [wubi] 里
wubi = ""
# 旧写法，改用上面的 scheme；两处都写时以 scheme 为准
# shuangpin = ""
# zhuyin = false
# 日志级别：info 缺省 / debug 详细（会记录敲的拼音与上屏的文字，配合作者排查问题时再开）。日志在 ~/Library/Logs/Glimmer/
log_level = "info"
# 输入日志：每次上屏记一行到数据目录的 input-log.jsonl（敲的键、看到的候选、选了什么），只写在这台电脑上，不上传；
# 用来离线评测排序和训练个人模型。false 不记；「高级」页可以清空
input_log = true
# 学习输入习惯：按你的选择调整候选顺序、记新词与敲错纠正。false 不再学，已学的仍参与排序；学习数据在数据目录，删掉文件即清空
learning = true
# 把系统设置「键盘 → 文本替换」里的条目当自定义短语：输入码（小写字母）敲全后短语出现在该码最靠前的空位；只有 macOS 用
system_text_replacements = true

# 自定义短语示例：取消下面各行注释后启用；同码同位置不能重复。
# [[custom_phrases]]
# code = "ww"       # 输入码：1–32 个小写英文字母
# text = "；"       # 原样上屏的文本，可包含空格与换行
# position = 1      # 固定候选位置：1–9
# enabled = true    # 是否启用；停用仍保留位置

[shortcut]
# 前缀模式键，只能是 v / u / i 之一且互不相同（这三个字母不是任何拼音音节的开头）
# 表达式模式：v1+2 出 3，v123 出中文数字
expression = "v"
# 问字模式：usangemu 问「三个木」（云端答），u4e00 出码点对应的字符（本地答）
question = "u"
# 没在组句时敲 ? 是否也进问字模式（中英文模式都行，后面跟字母才是问题，跟别的键还原成问号）；false 的话问号就是问号
question_mark = false
"#,
    template_shortcut_keys!(),
    r#"
[fuzzy]
# 模糊音：开了之后敲 zi 也出 zhi 的字、敲 lan 也出 nan 的字。默认全关，按需打开。
z_zh = false
c_ch = false
s_sh = false
n_l = false
f_h = false
l_r = false
an_ang = false
en_eng = false
in_ing = false

"#,
    template_apps!(),
    r#"
[dictionaries]
# 随包的领域词库（法律 / 医学 / 地名 / 成语 / 诗词 / IT / 财经 / 饮食 / 动物 / 汽车 / 历史人物），列在这里的才加载；
# 名字是文件名：animals automotive finance food historical_figures idioms it_computing law medicine places poetry_lines ai。
# 偏好设置「词库」页可以勾选
domains = ["idioms"]
# 自己导入的词库：放在配置同目录 dicts/ 下的 .qj 文件都会加载，这里列出要关掉的（文件名，不含扩展名）
disabled = []

[model]
# 本地整句模型：随包的小模型在本机给整句候选重新排序，全程离线；停顿后几十毫秒生效。关掉只用词库统计
enabled = true

[wubi]
# 五笔的行为（[general] wubi 开着才生效）。敲满四码且有全码命中时首选自动上屏
auto_select = true
# 逐键提示候选（编码比敲的长的）右侧显示完整编码
hint = true
# 不超过这么多码时候选只按码表顺序，不按你的用词习惯重排：一级 / 二级简码的位置固定
fixed_order_length = 2
# 整句输入：连着打编码不按空格，超过四码后首选是整句，空格上屏。开着时不再四码自动上屏，也不顶字
sentence = false

[predict]
# 云联想：把光标附近的文本发到下面的接口，让模型补全整句 / 联想下文。默认关闭。
# 开启后菜单栏的「中 / 英」旁会带一个云朵标识；Secure Input（密码框）里绝不发送。
enabled = false
# OpenAI 兼容接口地址与模型名（DeepSeek 默认值）
base_url = "https://api.deepseek.com"
model = "deepseek-v4-flash"
# 推理强度（reasoning_effort）：none 关掉模型的思考，联想要快；留空则不发这个参数
reasoning_effort = "none"
# 密钥：填在这里，或留空并设置 api_key_env 指定的环境变量（偏好设置里填的密钥写进配置同目录的 .env）
# api_key = ""
api_key_env = "GLIMMER_API_KEY"
# 单次请求超时（毫秒）、停止敲键多久后才发请求（毫秒）
timeout_ms = 5000
debounce_ms = 300
# 光标前 / 后最多发多少个字符——这是发往云端的上下文上限
lookback = 64
lookahead = 32
# 云端词到了补进候选窗口第一页末尾几格（比如 2 就是 8、9 两格），前面的本地候选不动；0 表示不要云端词
slots = 2
# 组句中除了词候选还要不要整句补全（preedit 右侧，Tab 接受）
sentence = true

[status_bar]
# 桌面上常驻、可拖动的悬浮状态条（Windows）：「中 / 英」格点一下切换模式（开着双拼时还显示方案名）、「，。」格切全角 / 半角标点、齿轮打开设置。
# 只在当前输入法是微明时显示；与任务栏的中 / 英指示器并存
# 默认关；开着时可以拖到任意位置，拖到哪下次还在哪（拖动结束时把位置写进下面的 x / y，不用手填）
enabled = false
# 记住的屏幕位置（物理像素，拖动后自动写入）；留空则首次出现在屏幕右下角
# x = 0
# y = 0

[update]
# 自动检查新版本（每 12 小时最多一次）：只从下面的地址下载一份几 KB 的版本清单，不带任何个人信息。
# 有新版本只在菜单 / 设置的「关于」页提示，下载与安装都要你自己点；false 就只有手动点「检查更新」才查
check = true
# 版本清单地址，一般不用改
feed_url = "https://github.com/OpenFabrica/glimmer/releases/latest/download/releases.json"
"#
);

impl Config {
    /// 随包五笔码表的文件名（`wubi86.qj` / `wubi98.qj` / `wubixsj.qj`），壳与 CLI 按它找文件；五笔没开为 `None`。
    pub fn wubi_table_file(&self) -> Option<&'static str> {
        self.general
            .wubi()
            .map(glimmer_core::WubiVariant::data_file)
    }

    /// 保存自定义短语列表，冲突时不修改文件。
    pub fn set_custom_phrases(
        path: &Path,
        phrases: &[glimmer_core::CustomPhrase],
    ) -> Result<(), String> {
        glimmer_core::custom_phrase::validate_phrases(phrases)?;
        let source = match std::fs::read_to_string(path) {
            Ok(s) => s,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => TEMPLATE.to_owned(),
            Err(e) => return Err(e.to_string()),
        };
        let mut document: DocumentMut = source.parse::<DocumentMut>().map_err(|e| e.to_string())?;
        let old = document
            .get("custom_phrases")
            .and_then(toml_edit::Item::as_array_of_tables)
            .cloned()
            .unwrap_or_default();
        let mut used = std::collections::BTreeSet::new();
        // 先按输入码和位置匹配，重排或删除时注释跟随原规则。
        let mut matches: Vec<_> = phrases
            .iter()
            .map(|p| {
                let found = old
                    .iter()
                    .enumerate()
                    .find(|(_, t)| {
                        t.get("code").and_then(toml_edit::Item::as_str) == Some(p.code.as_str())
                            && t.get("position").and_then(toml_edit::Item::as_integer)
                                == Some(p.position as i64)
                    })
                    .map(|(i, _)| i);
                if let Some(i) = found {
                    used.insert(i);
                }
                found
            })
            .collect();
        // 等长列表中修改了输入码或位置的条目，沿用其未被占用的原表。
        if old.len() == phrases.len() {
            for (i, matched) in matches.iter_mut().enumerate() {
                if matched.is_none() && used.insert(i) {
                    *matched = Some(i);
                }
            }
        }
        let mut tables = toml_edit::ArrayOfTables::new();
        for (p, matched) in phrases.iter().zip(matches) {
            let mut t = matched
                .and_then(|i| old.get(i))
                .cloned()
                .unwrap_or_default();
            for (key, mut value) in [
                ("code", toml_edit::Value::from(p.code.as_str())),
                ("text", toml_edit::Value::from(p.text.as_str())),
                ("position", toml_edit::Value::from(p.position as i64)),
                ("enabled", toml_edit::Value::from(p.enabled)),
            ] {
                if let Some(previous) = t.get(key).and_then(toml_edit::Item::as_value) {
                    if previous.to_string() == value.to_string() {
                        continue;
                    }
                    *value.decor_mut() = previous.decor().clone();
                }
                t[key] = toml_edit::Item::Value(value);
            }
            // 序列化按文档位置排序；统一锚点后，同组条目使用本次列表顺序。
            t.set_position(old.iter().filter_map(toml_edit::Table::position).min());
            tables.push(t);
        }
        document["custom_phrases"] = toml_edit::Item::ArrayOfTables(tables);
        write_file(path, &document.to_string()).map_err(|e| e.to_string())
    }

    /// 读配置。文件不存在按默认值；存在但解析失败报错，不要静默吞掉用户的笔误。
    pub fn load(path: &Path) -> Result<Self, ConfigError> {
        let source = match std::fs::read_to_string(path) {
            Ok(source) => source,
            Err(source) if source.kind() == std::io::ErrorKind::NotFound => {
                return Ok(Self::default());
            }
            Err(source) => {
                return Err(ConfigError::Read {
                    path: path.to_owned(),
                    source,
                });
            }
        };
        toml::from_str(&source).map_err(|source| ConfigError::Parse {
            path: path.to_owned(),
            source: Box::new(source),
        })
    }

    /// 原地改一个布尔键，见 [`Self::set_value`]。
    pub fn set_bool(path: &Path, section: &str, key: &str, value: bool) -> Result<(), ConfigError> {
        Self::set_value(path, section, key, value)
    }

    /// 原地改一个键（`[section] key = value`），其余内容、注释与顺序原样保留：
    /// 菜单和设置窗口落盘都走这里。文件不存在时从模板起步；文件有语法错误就报错不写，
    /// 不能替用户「修复」成丢了注释的文件。
    pub fn set_value(
        path: &Path,
        section: &str,
        key: &str,
        value: impl Into<toml_edit::Value>,
    ) -> Result<(), ConfigError> {
        let source = match std::fs::read_to_string(path) {
            Ok(source) => source,
            Err(source) if source.kind() == std::io::ErrorKind::NotFound => TEMPLATE.to_owned(),
            Err(source) => {
                return Err(ConfigError::Read {
                    path: path.to_owned(),
                    source,
                });
            }
        };
        let mut document: DocumentMut = source.parse().map_err(|source| ConfigError::Edit {
            path: path.to_owned(),
            source,
        })?;
        // 分节不存在时先建成标准表，否则 toml_edit 会写成顶层的行内表 `predict = { enabled = true }`
        if !document.get(section).is_some_and(|item| item.is_table()) {
            document[section] = toml_edit::table();
        }
        document[section][key] = toml_edit::value(value);
        // 写临时文件再改名：输入法进程随时可能被杀，不能留半个配置文件
        write_file(path, &document.to_string())
    }

    /// 原地把一个键改成字符串数组（`[section] key = ["a", "b"]`），其余内容、注释与顺序原样保留。
    /// 设置界面改词库列表（`[dictionaries] domains` / `disabled`）走这里，[`Self::set_value`] 只能写标量。
    pub fn set_array<S: AsRef<str>>(
        path: &Path,
        section: &str,
        key: &str,
        values: &[S],
    ) -> Result<(), ConfigError> {
        let source = match std::fs::read_to_string(path) {
            Ok(source) => source,
            Err(source) if source.kind() == std::io::ErrorKind::NotFound => TEMPLATE.to_owned(),
            Err(source) => {
                return Err(ConfigError::Read {
                    path: path.to_owned(),
                    source,
                });
            }
        };
        let mut document: DocumentMut = source.parse().map_err(|source| ConfigError::Edit {
            path: path.to_owned(),
            source,
        })?;
        if !document.get(section).is_some_and(|item| item.is_table()) {
            document[section] = toml_edit::table();
        }
        let mut array = toml_edit::Array::new();
        for value in values {
            array.push(value.as_ref());
        }
        document[section][key] = toml_edit::value(array);
        write_file(path, &document.to_string())
    }

    /// 文件不存在时写出模板（目录一并建），返回是否写了。
    pub fn write_template_if_missing(path: &Path) -> Result<bool, ConfigError> {
        if path.exists() {
            return Ok(false);
        }
        write_file(path, TEMPLATE)?;
        Ok(true)
    }
}

/// 原子写配置文件；数据目录还没有就先建（新账户第一次打开设置时输入法可能还没跑过）。
fn write_file(path: &Path, text: &str) -> Result<(), ConfigError> {
    let write = || {
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir)?;
        }
        glimmer_core::storage::write_atomic_str(path, text)
    };
    write().map_err(|source| ConfigError::Write {
        path: path.to_owned(),
        source,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn template_parses_to_defaults() {
        let config: Config = toml::from_str(TEMPLATE).unwrap();
        assert_eq!(config, Config::default());
    }

    #[test]
    fn partial_file_keeps_other_defaults() {
        let config: Config = toml::from_str("[predict]\nenabled = true\nlookback = 10\n").unwrap();
        assert!(config.predict.enabled);
        assert_eq!(config.predict.lookback, 10);
        assert_eq!(config.predict.model, "deepseek-v4-flash");
        assert_eq!(config.predict.reasoning_effort, "none");
        assert_eq!(config.predict.api_key_env, "GLIMMER_API_KEY");
    }

    #[test]
    fn wubi_section_and_table_file_parse() {
        let config: Config =
            toml::from_str("[general]\nwubi = \"86\"\n[wubi]\nauto_select = false\n").unwrap();
        assert_eq!(
            config.general.wubi(),
            Some(glimmer_core::WubiVariant::Wubi86)
        );
        assert_eq!(config.wubi_table_file(), Some("wubi86.qj"));
        assert!(!config.wubi.auto_select && config.wubi.hint);
        assert_eq!(config.wubi.fixed_order_length, 2);
    }

    #[test]
    fn update_section_parses() {
        let config: Config = toml::from_str("[update]\ncheck = false\n").unwrap();
        assert!(!config.update.check);
        assert_eq!(config.update.feed_url, glimmer_update::DEFAULT_FEED_URL);
    }

    #[test]
    fn fuzzy_section_parses() {
        let config: Config = toml::from_str("[fuzzy]\nz_zh = true\nan_ang = true\n").unwrap();
        assert!(config.fuzzy.z_zh && config.fuzzy.an_ang && !config.fuzzy.n_l);
        assert!(config.fuzzy.any());
    }

    #[test]
    fn general_and_shortcut_sections_parse() {
        let config: Config = toml::from_str(
            "[general]\npage_size = 5\npage_keys = \"[]\"\ntheme = \"dark\"\nlayout = \"horizontal\"\npreedit = \"window\"\n[shortcut]\nexpression = \"i\"\n",
        )
        .unwrap();
        assert_eq!(config.general.page_size(), 5);
        assert_eq!(config.general.page_keys(), ('[', ']'));
        assert_eq!(config.general.theme, ThemeMode::Dark);
        assert_eq!(config.general.layout, LayoutMode::Horizontal);
        assert_eq!(config.general.preedit, PreeditMode::Window);
        assert_eq!(config.general.learning_language, "en");
        assert!(config.general.translation_reading);
        assert!(config.general.english_candidates);
        assert!(!config.general.traditional);
        assert!(config.general.mixed_english_candidates && config.general.emoji_candidates);
        assert_eq!(config.general.shuangpin(), None);
        assert_eq!(config.general.wubi(), None);
        assert_eq!(config.wubi_table_file(), None);
        assert_eq!(config.general.log_level, LogLevel::Info);
        assert_eq!(config.shortcut.mode.expression, 'i');
        assert_eq!(config.shortcut.mode.question, 'u');
        // 译词修饰键缺省分平台（Windows 是 Ctrl 系，其余 Option 系，见 shortcut.rs），断言跟着 Default 走
        assert_eq!(
            config.shortcut.translation,
            Config::default().shortcut.translation
        );
    }

    #[test]
    fn set_value_writes_strings_and_integers() {
        let path = std::env::temp_dir().join("glimmer-config-set-value-test.toml");
        let _ = std::fs::remove_file(&path);
        Config::set_value(&path, "general", "page_size", 5i64).unwrap();
        Config::set_value(&path, "general", "theme", "dark").unwrap();
        Config::set_value(&path, "shortcut", "question", "i").unwrap();
        let config = Config::load(&path).unwrap();
        assert_eq!(config.general.page_size, 5);
        assert_eq!(config.general.theme, ThemeMode::Dark);
        assert_eq!(config.shortcut.mode.question, 'i');
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn set_bool_keeps_comments_and_flips_only_that_key() {
        let path = std::env::temp_dir().join("glimmer-config-set-bool-test.toml");
        std::fs::write(
            &path,
            "# 头注释\n[fuzzy]\n# 说明\nz_zh = false\nn_l = true\n",
        )
        .unwrap();
        Config::set_bool(&path, "fuzzy", "z_zh", true).unwrap();
        Config::set_bool(&path, "predict", "enabled", true).unwrap();
        let text = std::fs::read_to_string(&path).unwrap();
        assert!(
            text.starts_with("# 头注释\n[fuzzy]\n# 说明\nz_zh = true\nn_l = true\n"),
            "{text}"
        );
        let config = Config::load(&path).unwrap();
        assert!(config.fuzzy.z_zh && config.fuzzy.n_l && config.predict.enabled);
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn writes_create_the_data_directory_for_a_fresh_account() {
        let dir = std::env::temp_dir().join("glimmer-config-fresh-account-test");
        let _ = std::fs::remove_dir_all(&dir);
        let path = dir.join("Glimmer").join("config.toml");
        assert!(Config::write_template_if_missing(&path).unwrap());
        assert!(!Config::write_template_if_missing(&path).unwrap());
        assert_eq!(std::fs::read_to_string(&path).unwrap(), TEMPLATE);
        // 没有模板直接保存也行
        std::fs::remove_dir_all(&dir).unwrap();
        Config::set_bool(&path, "predict", "enabled", true).unwrap();
        assert!(Config::load(&path).unwrap().predict.enabled);
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn set_bool_starts_from_template_when_missing() {
        let path = std::env::temp_dir().join("glimmer-config-set-bool-missing-test.toml");
        let _ = std::fs::remove_file(&path);
        Config::set_bool(&path, "predict", "enabled", true).unwrap();
        let text = std::fs::read_to_string(&path).unwrap();
        assert!(text.contains("# 微明输入法配置"));
        assert!(Config::load(&path).unwrap().predict.enabled);
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn set_bool_refuses_broken_file() {
        let path = std::env::temp_dir().join("glimmer-config-set-bool-broken-test.toml");
        std::fs::write(&path, "[fuzzy\nz_zh = false\n").unwrap();
        assert!(matches!(
            Config::set_bool(&path, "fuzzy", "z_zh", true),
            Err(ConfigError::Edit { .. })
        ));
        assert_eq!(
            std::fs::read_to_string(&path).unwrap(),
            "[fuzzy\nz_zh = false\n"
        );
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn missing_file_is_default() {
        let path = std::env::temp_dir().join("glimmer-config-missing-test.toml");
        let _ = std::fs::remove_file(&path);
        assert_eq!(Config::load(&path).unwrap(), Config::default());
    }
}
