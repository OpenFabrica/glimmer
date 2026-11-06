//! 进程级单例：一个 Engine + 一个候选窗口，所有输入会话共用。
//!
//! IMK 的所有回调都在主线程，所以用 `thread_local` 而不是锁；从别的线程访问会拿到 `None`。
//!
//! 配置只有一条通路：[`Host::apply_config`] 把当前 `Config` 推给 Engine 与界面。启动、菜单开关、
//! 设置窗口、手改文件被监视到，全都走它；三个入口都只写 `config.toml`，不各存一套状态。

mod cloud;
mod config;
mod diagnostics;
mod dictionaries;
mod init;
mod learn_text;
mod model;
mod next_word;
mod presenting;
mod session;
mod settings;
mod update;
mod wubi;

use std::cell::RefCell;
use std::path::PathBuf;

use glimmer_core::{
    Candidate, CandidateKind, CloudWord, EmojiTable, Engine, FuzzyRules, Language, ModeKeys,
    NoGlossFiller, NoInputLogger, NoPredictor, NoTranslator, Prediction,
};
use glimmer_dictionary::{Dictionary, Misspellings, WordList};
use glimmer_learning::{FrequencyLearner, InputLog, UsageStats, VocabularyBook};
use glimmer_lm::NgramModel;
use glimmer_platform::extra_dictionaries;
use glimmer_platform::{
    AppsConfig, CandidateRenderer, DEFAULT_ENGLISH_CANDIDATES_OFF, DictionariesConfig,
    GeneralConfig, KeyCombo, LEARNING_LANGUAGE_OFF, LayoutMode, LocalModelConfig, LogLevel,
    ModeSwitch, Modifiers, PAGE_KEY_OPTIONS, PreeditMode, Scheme, ShortcutConfig, ThemeMode,
};
use glimmer_predict::{
    CloudGlossFiller, CloudPredictor, ConnectionTest, PredictConfig, PredictError,
};
use glimmer_translate::{Glossary, LayeredTranslator, LevelTable, PersonalGlossary};
use objc2::MainThreadMarker;
use objc2::rc::Retained;
use objc2::runtime::AnyObject;
use objc2_app_kit::{NSPasteboard, NSPasteboardTypeString};
use objc2_foundation::{NSProcessInfo, NSRect, NSString};

use crate::app::BundleInfo;
use crate::app::{Settings, logging, paths};
use crate::candidates::{CandidateWindow, Preedit};
use crate::error::HostError;
use crate::menubar::{InputMenu, MenuAction, ModeIndicator};
use crate::preferences::{PreferencesWindow, Setting, SettingValue, WUBI_VARIANTS};

use cloud::{CloudTestMonitor, PredictMonitor};
use config::{ConfigWatch, TextReplacement};
pub use dictionaries::DictionaryInfo;
pub use init::init;
use model::RescoreMonitor;
use next_word::NextWordMonitor;
pub use next_word::NextWordPhase;
use presenting::Notice;
pub use presenting::TranslationJob;
pub use session::Session;
use update::UpdateState;
use wubi::load_wubi_scheme;

pub struct Host {
    /// 输入内核。平台层只能通过它的公开 API 拿候选，不允许碰词库或排序。
    pub engine: Engine,

    /// 候选窗口。
    pub window: CandidateWindow,

    /// 菜单栏的中 / 英状态项。
    pub indicator: ModeIndicator,

    /// 输入法菜单，挂在状态项和系统输入源菜单上。
    pub menu: InputMenu,

    /// 偏好设置窗口。
    pub preferences: PreferencesWindow,

    /// 配置文件的当前值与修改时间。
    pub settings: Settings,

    /// 配置文件监视定时器，激活期间跑。
    pub watch: ConfigWatch,

    /// 上次把学习数据落盘的时间；激活期间的定时器按 [`LEARNING_FLUSH_INTERVAL`] 再刷一次。
    pub last_flush: std::time::Instant,

    /// 当前 Predictor 是按哪份 `[predict]` 建的；配置没变就不重建（重建会起新线程、丢缓存）。
    applied_predict: PredictConfig,

    /// 附加词库是按哪份 `[dictionaries]` 装的；开关变了才重新加载。
    applied_dictionaries: DictionariesConfig,

    /// 偏好设置「词库」页显示的列表，勾选框 / 移除按钮的下标对着它。
    dictionary_list: Vec<DictionaryInfo>,

    /// 当前学习语言；`None` 为关（不显示译文）。
    learning_language: Option<Language>,

    /// 打进包里的释义表语言，设置窗口按这个顺序列。
    languages: Vec<Language>,

    /// 版本号与构建标识，诊断信息里用。
    version: String,

    /// 见 [`BundleInfo::build`]。
    build: String,

    /// 每页候选数（配置 `[general] page_size`，已夹到 1–9）。
    pub page_size: usize,

    /// 候选窗口第一页末尾留给云端词的格数（配置 `[predict] slots`）。
    pub cloud_slots: usize,

    /// 翻页键对（上一页、下一页）。
    pub page_keys: (char, char),

    /// 配数字键上屏第一 / 第二个译词的修饰键组合（配置 `[shortcut] translation` / `translation_second`）。
    pub translation_keys: (Modifiers, Modifiers),

    /// 配数字键删候选的修饰键（配置 `[shortcut] delete_candidate`）。
    pub delete_keys: Modifiers,

    /// 候选窗口顶行显示的一句临时状态（删了什么词），下一次查询就没了。
    pub status: Option<String>,

    /// 输入日志是否在记（配置 `[general] input_log`），换了才重开文件。
    input_log_enabled: Option<bool>,

    /// 翻译选中文字的快捷键（配置 `[shortcut] translate_selection`）。
    pub translate_keys: KeyCombo,

    /// 中英文切换键；配置冲突时禁用。
    pub mode_switch_keys: Option<ModeSwitch>,

    /// 进行中的「翻译选中文字」；有它时候选窗口显示的是译文（或「翻译中…」），按键先归它处理。
    pub translation: Option<TranslationJob>,

    /// 正在显示的提示（候选窗口里一行字，几秒后自动收）。
    pub notice: Option<Notice>,

    /// 组句中的拼音显示在行内、候选窗口还是两处。
    pub preedit_mode: PreeditMode,

    /// 英文模式是否给英文候选（配置 `[general] english_candidates`）。
    pub english_candidates: bool,

    /// 上次从系统读到的文本替换（激活输入法时重读），`[general] system_text_replacements` 开着时并进自定义短语。
    text_replacements: Vec<TextReplacement>,

    /// 按应用的行为（配置 `[apps]`）：哪些应用里英文模式不给候选。
    pub apps: AppsConfig,

    /// 联想结果轮询定时器。
    pub monitor: PredictMonitor,

    /// 英文下一词预测的显示阶段（预测本身在 Core）；不是 `Idle` 时候选窗口归它用。
    next_word: NextWordPhase,

    /// 当前这一键按下时预测在显示或等着画（用户不要它）：这一键后的空格不再取预测。每个按键开头重设。
    next_word_declined: bool,

    /// 下一词预测的轮询定时器：等一拍再画、发现过期。
    next_word_monitor: NextWordMonitor,

    /// 进行中的云服务连通性测试（「云服务」页「测试连接」按钮）；没在测为 `None`。
    cloud_test: Option<ConnectionTest>,

    /// 连通性测试的轮询定时器。
    cloud_test_monitor: CloudTestMonitor,

    /// 本地整句模型的防抖与轮询定时器。
    rescore: RescoreMonitor,

    /// 正在后台加载的模型；加载完接到 Engine 上就清掉。
    model_loader: Option<
        std::sync::mpsc::Receiver<Result<glimmer_neural::CharScorer, glimmer_neural::NeuralError>>,
    >,

    /// 上次套用的 `[model]`，变了才重载 / 卸载。
    applied_model: Option<LocalModelConfig>,

    /// 检查更新与下载安装包的状态。
    update: UpdateState,

    /// 当前会话的候选、高亮、页码、preedit。
    pub session: Session,

    /// 组句中到达的整句补全，Tab 接受。
    pub sentence: Option<String>,

    /// 最近一次绘制时的光标矩形，联想结果到达后在同一位置重画。
    pub anchor: NSRect,

    /// 当前激活会话的客户端（IMK 传给 activateServer 的 sender），给不在按键回调里的路径（配置热加载清组句）
    /// 清 marked text 用；停用时清掉。对它的调用要放在 [`with`] 的借用之外（IPC 期间 IMK 可能重入）。
    pub active_client: Option<Retained<AnyObject>>,

    /// [`Host::reset_composition`] 清掉了正在组句的内容、而应用里的 marked text 还没清：等有客户端的路径来收。
    pub marked_stale: bool,

    /// 当前激活的输入控制器（对象地址，只用来比对）。IMK 在同一应用里换焦点时先 activate 新会话、
    /// 再 deactivate 旧会话（Chromium 系浏览器里能隔两百多毫秒），旧会话的 deactivate 不能把全局状态拆掉。
    pub active_controller: Option<usize>,
}

thread_local! {
    static HOST: RefCell<Option<Host>> = const { RefCell::new(None) };
}

/// 激活期间学习数据最多隔这么久落一次盘。
const LEARNING_FLUSH_INTERVAL: std::time::Duration = std::time::Duration::from_secs(60);

/// 输入统计文件名，与学习数据同目录（按天一行，见 `glimmer-learning::UsageStats`）。
const USAGE_FILE: &str = "usage.tsv";

/// 词汇记录文件名，与学习数据同目录（一个译词一行，见 `glimmer-learning::VocabularyBook`）。
const VOCABULARY_FILE: &str = "user-vocab.tsv";

/// 可能打进包里的释义表语言，按这个顺序在设置里列出；文件不存在的不列。
const GLOSSARY_LANGUAGES: [Language; 3] =
    [Language::English, Language::Japanese, Language::Spanish];

/// 在单例上执行操作。未初始化、不在主线程、或正处在另一次 `with` 之内（重入）时返回 `None`。
///
/// 重入是真会发生的：闭包里若碰了应用那边的东西（读上下文、取光标位置、插入文字），IMK 会在等应用回话时
/// 跑一轮 run loop，`deactivateServer:` 这类回调就可能在这时进来；这时再 `borrow_mut` 就是 panic 加进程退出。
/// 所以借不到就记一条日志放过这次调用，调用方本来就都按 `None` 处理；根治靠把 IPC 放在借用之外。
pub fn with<R>(f: impl FnOnce(&mut Host) -> R) -> Option<R> {
    HOST.with(|host| match host.try_borrow_mut() {
        Ok(mut guard) => guard.as_mut().map(f),
        Err(_) => {
            tracing::warn!("Host 正在被借用（IMK 回调重入），本次调用跳过");
            None
        }
    })
}
