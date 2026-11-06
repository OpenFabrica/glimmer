//! 各模块共用的零件：造 Router、造按键、拆回话、假的状态条、打分器与英文语言模型。

pub use std::path::PathBuf;
pub use std::sync::{Arc, Mutex};

pub use glimmer_core::english::NgramHistory;
pub use glimmer_core::sentence::{Context, LanguageModel, SentenceScorer};
pub use glimmer_core::{Language, ModeKeys, ShuangpinScheme};
pub use glimmer_platform::protocol::{
    ClientMessage, Frame, KeyEvent, KeyModifiers, KeyOutcome, PROTOCOL_VERSION, ServerMessage,
    SessionId,
};
pub use glimmer_platform::{
    AppsConfig, DEFAULT_ENGLISH_CANDIDATES_OFF_WINDOWS, PreeditMode, Scheme,
};
pub use glimmer_server::dispatch::{StatusEvent, StatusSink, StatusView};
pub use glimmer_server::{AssemblySpec, Router, RouterConfig, assembly};

pub const SESSION: SessionId = SessionId(1);

/// Caps Lock 亮着。
pub const CAPS: KeyModifiers = KeyModifiers {
    ctrl: false,
    shift: false,
    alt: false,
    win: false,
    caps: true,
    english_mode: false,
};

/// 持久英文模式（Caps 灭）。
pub const ENGLISH: KeyModifiers = KeyModifiers {
    ctrl: false,
    shift: false,
    alt: false,
    win: false,
    caps: false,
    english_mode: true,
};

/// 样例词库装一个 Router，开好一个会话。
pub fn router() -> Router {
    router_with(RouterConfig::default())
}

pub fn router_with(config: RouterConfig) -> Router {
    router_in(config, None)
}

/// 在某个应用（宿主 exe 名）里开会话，名单用 Windows 缺省那份。
pub fn router_in_app(app: &str) -> Router {
    let config = RouterConfig {
        apps: AppsConfig::with_english_candidates_off(DEFAULT_ENGLISH_CANDIDATES_OFF_WINDOWS),
        ..RouterConfig::default()
    };
    router_in(config, Some(app.to_owned()))
}

/// `?` 开着当问字入口的 Router（配置 `[shortcut] question_mark`，缺省关）。
pub fn router_asking() -> Router {
    router_asking_with(RouterConfig::default())
}

pub fn router_asking_with(config: RouterConfig) -> Router {
    let mut router = router_with(config);
    router.engine_mut().set_mode_keys(ModeKeys {
        question_mark: true,
        ..ModeKeys::default()
    });
    router
}

pub fn router_in(config: RouterConfig, app: Option<String>) -> Router {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let dict = root.join("assets/sample/dict.tsv");
    let glossary = root.join("assets/sample/glossary-en.tsv");
    let mut engine = assembly::assemble(&AssemblySpec {
        glossary: Some((Language::English, glossary)),
        english: Some(root.join("assets/sample/english.tsv")),
        ..AssemblySpec::new(dict)
    })
    .expect("assemble engine from sample data");
    // 与 main.rs 一样，双拼方案是启动时直接设给 Engine 的。
    engine.set_shuangpin(config.scheme.shuangpin());
    let mut router = Router::new(engine, config);
    assert_eq!(
        router.handle(ClientMessage::OpenSession {
            session: SESSION,
            app,
            protocol: PROTOCOL_VERSION,
        }),
        None
    );
    router
}

pub fn letter(c: char) -> KeyEvent {
    letter_with(c, Default::default())
}

/// `c` 的大小写就是 DLL 按 Shift 解析出的字符。
pub fn letter_with(c: char, modifiers: KeyModifiers) -> KeyEvent {
    KeyEvent::new(c.to_ascii_uppercase() as u32, Some(c), modifiers)
}

pub fn press(router: &mut Router, event: KeyEvent) -> (KeyOutcome, Option<String>, Frame) {
    key_result(router.handle(ClientMessage::Key {
        session: SESSION,
        event,
    }))
}

/// 英文模式下敲一串字母，返回最后一次的处理结果。
pub fn type_english(router: &mut Router, text: &str) -> (KeyOutcome, Option<String>, Frame) {
    let mut last = None;
    for c in text.chars() {
        last = Some(press(router, letter_with(c, ENGLISH)));
    }
    last.expect("typed at least one letter")
}

pub fn candidate_texts(frame: &Frame) -> Vec<&str> {
    frame
        .candidates
        .items
        .iter()
        .map(|c| c.text.as_str())
        .collect()
}

pub fn digit(n: u32) -> KeyEvent {
    digit_with(n, Default::default())
}

/// 数字键 1–9；`character` 按 DLL 的解析：按着 Shift 是上档字符。
pub fn digit_with(n: u32, modifiers: KeyModifiers) -> KeyEvent {
    let c = if modifiers.shift {
        b")!@#$%^&*("[n as usize] as char
    } else {
        char::from_digit(n, 10).unwrap()
    };
    KeyEvent::new(0x30 + n, Some(c), modifiers)
}

pub const SHIFT: KeyModifiers = KeyModifiers {
    shift: true,
    ..ALT_OFF
};

pub const CTRL: KeyModifiers = KeyModifiers {
    ctrl: true,
    ..ALT_OFF
};

pub const ALT_OFF: KeyModifiers = KeyModifiers {
    ctrl: false,
    shift: false,
    alt: false,
    win: false,
    caps: false,
    english_mode: false,
};

/// 两个平台的缺省快捷键都没用 Win，拿来测「没配到的修饰键归应用」。
pub const WIN: KeyModifiers = KeyModifiers {
    win: true,
    ..ALT_OFF
};

/// 平台缺省的译词键：macOS 是 Alt，Windows 是 Ctrl（Alt 被系统菜单截走）。
#[cfg(not(windows))]
pub const TRANSLATE: KeyModifiers = KeyModifiers {
    alt: true,
    ..ALT_OFF
};
#[cfg(windows)]
pub const TRANSLATE: KeyModifiers = KeyModifiers {
    ctrl: true,
    ..ALT_OFF
};

pub const TRANSLATE_SECOND: KeyModifiers = KeyModifiers {
    shift: true,
    ..TRANSLATE
};

/// 当前页里 `text` 排第几（1 起）。
pub fn slot_of(frame: &Frame, text: &str) -> u32 {
    let position = candidate_texts(frame)
        .iter()
        .position(|t| *t == text)
        .unwrap_or_else(|| panic!("{text} 应在当前页：{:?}", candidate_texts(frame)));
    position as u32 + 1
}

/// 带字符的按键（标点等），虚拟键码随便给一个 OEM 键。
pub fn punct(c: char) -> KeyEvent {
    KeyEvent::new(0xBE, Some(c), Default::default())
}

pub fn key_result(message: Option<ServerMessage>) -> (KeyOutcome, Option<String>, Frame) {
    match message {
        Some(ServerMessage::KeyResult {
            outcome,
            commit,
            frame,
            ..
        }) => (outcome, commit, frame),
        other => panic!("expected KeyResult, got {other:?}"),
    }
}

/// 中文模式下敲一串字母，返回最后一次的处理结果。
pub fn type_letters(router: &mut Router, text: &str) -> (KeyOutcome, Option<String>, Frame) {
    let mut last = None;
    for c in text.chars() {
        last = Some(key_result(router.handle(ClientMessage::Key {
            session: SESSION,
            event: letter(c),
        })));
    }
    last.expect("typed at least one letter")
}

pub fn preedit(frame: &Frame) -> String {
    frame.preedit.iter().map(|s| s.text.as_str()).collect()
}

/// 记录状态条调用：`Some(模式格文字)` 是显示、`None` 是收起。
#[derive(Clone, Default)]
pub struct RecordingStatus(pub Arc<Mutex<Vec<Option<String>>>>);

impl RecordingStatus {
    pub fn calls(&self) -> Vec<Option<String>> {
        self.0.lock().unwrap().clone()
    }
}

impl StatusSink for RecordingStatus {
    fn show_status(&self, view: StatusView) {
        let label = match (view.english, view.scheme) {
            (true, _) => "英".to_owned(),
            (false, Some(scheme)) => format!("中 · {scheme}"),
            (false, None) => "中".to_owned(),
        };
        self.0.lock().unwrap().push(Some(label));
    }

    fn hide_status(&self) {
        self.0.lock().unwrap().push(None);
    }
}

pub fn function_key(virtual_key: u32) -> KeyEvent {
    KeyEvent::new(virtual_key, None, Default::default())
}

/// 假打分器：偏爱某个文本，其余都给低分（与 Core 的重打分测试同款）。
pub struct Prefers(pub &'static str);

impl SentenceScorer for Prefers {
    fn score(&self, _context: &str, texts: &[&str]) -> Vec<f64> {
        texts
            .iter()
            .map(|t| if *t == self.0 { -1.0 } else { -20.0 })
            .collect()
    }
}

/// 接了假模型的 Router：本地整句模型在壳里是异步接法，按键先按词级出候选，停顿后 tick 才换。
pub fn router_with_scorer(preferred: &'static str) -> Router {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let mut engine = assembly::assemble(&AssemblySpec::new(root.join("assets/sample/dict.tsv")))
        .expect("assemble engine from sample data");
    engine.set_async_sentence_scorer(Some(Box::new(Prefers(preferred))));
    let mut router = Router::new(engine, RouterConfig::default());
    router.handle(ClientMessage::OpenSession {
        session: SESSION,
        app: None,
        protocol: PROTOCOL_VERSION,
    });
    router
}

/// 一直 tick 到首选变成 `text` 或等满 `timeout`；返回最后一帧。
pub fn tick_until_first(router: &mut Router, text: &str, timeout: std::time::Duration) -> Frame {
    let started = std::time::Instant::now();
    loop {
        std::thread::sleep(router.next_tick().min(std::time::Duration::from_millis(20)));
        router.tick();
        let frame = match router.handle(ClientMessage::Poll { session: SESSION }) {
            Some(ServerMessage::Update { frame, .. }) => frame,
            other => panic!("expected Update, got {other:?}"),
        };
        if candidate_texts(&frame).first() == Some(&text) || started.elapsed() > timeout {
            return frame;
        }
    }
}

pub fn press_in(router: &mut Router, session: SessionId, event: KeyEvent) {
    let _ = router.handle(ClientMessage::Key { session, event });
}

/// 假英文语言模型：只有手写的几条二元（概率, 次数），够下一词预测一路接下去：hello → world → the → 没了。
pub struct ChainModel(pub &'static [(&'static str, &'static str, f64, u32)]);

impl ChainModel {
    /// `previous` 之后的二元后继。
    fn after(
        &self,
        previous: &str,
    ) -> impl Iterator<Item = &(&'static str, &'static str, f64, u32)> {
        self.0.iter().filter(move |(p, ..)| *p == previous)
    }

    /// `previous` 之后 `word` 的概率；没有这条二元为 `None`。
    fn probability(&self, previous: Option<&str>, word: &str) -> Option<f64> {
        let previous = previous?;
        self.after(previous)
            .find(|(_, w, ..)| *w == word)
            .map(|(_, _, p, _)| *p)
    }
}

impl LanguageModel for ChainModel {
    /// 只看前一个词的二元概率。
    fn log_prob(&self, context: Context<'_>, word: &str) -> Option<f64> {
        self.probability(context.previous, word).map(f64::ln)
    }

    /// 逐词查前一个词的二元概率；不知道上文时全是 `None`。
    fn log_probs(&self, history: NgramHistory<'_>, words: &[&str]) -> Vec<Option<f64>> {
        words
            .iter()
            .map(|word| self.probability(history.previous(), word).map(f64::ln))
            .collect()
    }

    /// 前一个词的二元后继（预测从这里取候选）。
    fn successors(
        &self,
        history: NgramHistory<'_>,
        keep: &mut dyn FnMut(&str) -> bool,
    ) -> Vec<(String, u32)> {
        // 只有二元：知道前两个词时三元那一路给空，Core 再并上前一个词的二元
        let previous = match history {
            NgramHistory::Previous(previous) => previous,
            NgramHistory::Context(context) if context.earlier.is_none() => {
                let Some(previous) = context.previous else {
                    return Vec::new();
                };
                previous
            }
            _ => return Vec::new(),
        };
        self.after(previous)
            .filter(|(_, w, ..)| keep(w))
            .map(|(_, w, _, count)| ((*w).to_owned(), *count))
            .collect()
    }
}

/// 样例词表里的一条预测链：hello → world → the，the 之后没有后继。
pub const CHAIN: &[(&str, &str, f64, u32)] =
    &[("hello", "world", 0.6, 600), ("world", "the", 0.5, 500)];

/// 装了假英文语言模型、能出下一词预测的 Router；`prediction` 是 `[general] english_prediction`。
pub fn router_predicting(prediction: bool) -> Router {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let mut engine = assembly::assemble(&AssemblySpec {
        english: Some(root.join("assets/sample/english.tsv")),
        ..AssemblySpec::new(root.join("assets/sample/dict.tsv"))
    })
    .expect("assemble engine from sample data")
    .with_english_lm(Box::new(ChainModel(CHAIN)));
    engine.set_english_prediction(prediction);
    let mut router = Router::new(engine, RouterConfig::default());
    router.handle(ClientMessage::OpenSession {
        session: SESSION,
        app: None,
        protocol: PROTOCOL_VERSION,
    });
    router
}

/// 英文模式的空格。
pub fn english_space() -> KeyEvent {
    KeyEvent::new(0x20, Some(' '), ENGLISH)
}

/// 英文模式的功能键（无字符）。
pub fn english_key(virtual_key: u32) -> KeyEvent {
    KeyEvent::new(virtual_key, None, ENGLISH)
}

/// 拉一次轮询，返回帧。
pub fn poll(router: &mut Router) -> Frame {
    match router.handle(ClientMessage::Poll { session: SESSION }) {
        Some(ServerMessage::Update { frame, .. }) => frame,
        other => panic!("expected Update, got {other:?}"),
    }
}
