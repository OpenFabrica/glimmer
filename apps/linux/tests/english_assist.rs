//! 开关英文辅助的组合键（缺省 Ctrl+Shift+E）在前端的走法：只有这一个带修饰键的字母先送 Router 判断，
//! 字母统一成小写；Router 吃了就刷属性（「英」↔「英+」），放行就照常归应用。其他 Ctrl 组合照旧不送。
//! 后端用一个按 Router 契约答复的假 Router。

use std::sync::{Arc, Mutex};

use glimmer_linux::backend::{self, Backend};
use glimmer_linux::frontend::{ASSIST_PROPERTY, Output, Session};
use glimmer_linux::key::keysym;
use glimmer_linux::key::modifiers::{CONTROL_MASK, MOD1_MASK, RELEASE_MASK, SHIFT_MASK};
use glimmer_platform::KeyCombo;
use glimmer_platform::protocol::{
    ClientMessage, Frame, KeyEvent, KeyModifiers, KeyOutcome, PreeditKind, PreeditSegment,
    ServerMessage,
};

/// 假 Router：认得配置的组合键（字符小写 + 物理修饰键）就翻转开关、吃掉；不带修饰键的字母进组句；其余放行。
struct Scripted {
    /// 收到的消息，按顺序。
    log: Arc<Mutex<Vec<ClientMessage>>>,

    /// 英文辅助开着。
    assist: bool,

    /// 配置的组合键；`None` 是禁用。
    keys: Option<KeyCombo>,

    /// 认不认组合键：`false` 模拟配置刚热加载掉、前端还按旧组合送来的情形。
    recognizes: bool,

    /// 组句缓冲。
    buffer: String,
}

impl Scripted {
    /// 缺省组合、开关开着、认组合键。
    fn new(log: Arc<Mutex<Vec<ClientMessage>>>) -> Self {
        Self {
            log,
            assist: true,
            keys: Some(KeyCombo::ENGLISH_ASSIST_DEFAULT),
            recognizes: true,
            buffer: String::new(),
        }
    }

    /// 按 Router 的判定比组合键。
    fn matches(&self, event: &KeyEvent) -> bool {
        self.recognizes
            && self.keys.is_some_and(|combo| {
                event.character == Some(combo.key)
                    && event.modifiers.chord() == KeyModifiers::from(combo.modifiers)
            })
    }

    /// 当前帧：有缓冲是组句帧，否则空帧。
    fn frame(&self) -> Frame {
        if self.buffer.is_empty() {
            return Frame::default();
        }
        Frame {
            preedit: vec![PreeditSegment {
                text: self.buffer.clone(),
                kind: PreeditKind::Typed,
            }],
            cursor: self.buffer.chars().count(),
            ..Frame::default()
        }
    }
}

impl Backend for Scripted {
    fn send(&mut self, message: ClientMessage) -> Option<ServerMessage> {
        self.log.lock().unwrap().push(message.clone());
        let ClientMessage::Key { session, event } = message else {
            return None;
        };
        let outcome = if self.matches(&event) {
            self.assist = !self.assist;
            KeyOutcome::Consumed
        } else if event.character.is_some_and(|c| c.is_ascii_alphabetic())
            && event.modifiers.chord() == KeyModifiers::default()
        {
            self.buffer.extend(event.character);
            KeyOutcome::Consumed
        } else {
            KeyOutcome::Passthrough
        };
        Some(ServerMessage::KeyResult {
            session,
            outcome,
            commit: None,
            frame: self.frame(),
        })
    }

    fn english_assist_keys(&self) -> Option<KeyCombo> {
        self.keys
    }

    fn english_assist(&self) -> bool {
        self.assist
    }
}

/// 建会话并获焦；`setup` 改假 Router 的初始状态。返回会话与消息记录（获焦的消息已清掉）。
fn session(setup: impl FnOnce(&mut Scripted)) -> (Session, Arc<Mutex<Vec<ClientMessage>>>) {
    let log = Arc::new(Mutex::new(Vec::new()));
    let mut scripted = Scripted::new(Arc::clone(&log));
    setup(&mut scripted);
    let mut session = Session::new(backend::shared(scripted));
    session.focus_in(Some("test".to_owned()));
    log.lock().unwrap().clear();
    (session, log)
}

/// 记录里发过的按键。
fn sent_keys(log: &Arc<Mutex<Vec<ClientMessage>>>) -> Vec<KeyEvent> {
    log.lock()
        .unwrap()
        .iter()
        .filter_map(|message| match message {
            ClientMessage::Key { event, .. } => Some(*event),
            _ => None,
        })
        .collect()
}

/// X 按着 Ctrl+Shift 时 `E` 键送的是大写 keysym。
const CTRL_SHIFT: u32 = CONTROL_MASK | SHIFT_MASK;

#[test]
fn combo_is_forwarded_lowercased_consumed_and_refreshes_the_mode() {
    let (mut session, log) = session(|_| {});
    let (handled, out) = session.process_key(u32::from('E'), CTRL_SHIFT);
    assert!(handled, "Router 吃了，应用收不到");
    let keys = sent_keys(&log);
    assert_eq!(keys.len(), 1);
    assert_eq!(keys[0].character, Some('e'), "字母统一成小写再送");
    assert_eq!(keys[0].virtual_key, 0x45);
    assert!(keys[0].modifiers.ctrl && keys[0].modifiers.shift);
    assert!(out.contains(&Output::Mode {
        english: false,
        assist: false
    }));
    assert!(!session.english_assist());
    // 再按一次翻回来
    let (handled, out) = session.process_key(u32::from('E'), CTRL_SHIFT);
    assert!(handled);
    assert!(out.contains(&Output::Mode {
        english: false,
        assist: true
    }));
}

#[test]
fn english_mode_shows_the_assist_state_and_shift_chord_keeps_the_mode() {
    let (mut session, _) = session(|_| {});
    session.process_key(keysym::SHIFT_L, 0);
    session.process_key(keysym::SHIFT_L, RELEASE_MASK);
    assert!(session.english());
    // 按住 Ctrl、Shift 再按 E：Shift 不算单击，不切中英
    session.process_key(keysym::CONTROL_L, 0);
    session.process_key(keysym::SHIFT_L, CONTROL_MASK);
    let (handled, out) = session.process_key(u32::from('E'), CTRL_SHIFT);
    session.process_key(u32::from('E'), CTRL_SHIFT | RELEASE_MASK);
    let (_, released) = session.process_key(keysym::SHIFT_L, CTRL_SHIFT | RELEASE_MASK);
    assert!(handled);
    assert!(out.contains(&Output::Mode {
        english: true,
        assist: false
    }));
    assert!(released.is_empty());
    assert!(session.english(), "仍在英文模式");
}

#[test]
fn combo_is_forwarded_while_composing() {
    let (mut session, log) = session(|_| {});
    session.process_key(u32::from('n'), 0);
    assert!(session.composing());
    log.lock().unwrap().clear();
    let (handled, _) = session.process_key(u32::from('E'), CTRL_SHIFT);
    assert!(handled);
    assert_eq!(
        sent_keys(&log).len(),
        1,
        "组句中带 Ctrl 的字母平时不送，这一个送"
    );
}

#[test]
fn other_command_chords_are_not_forwarded() {
    for (keyval, state, what) in [
        (u32::from('e'), CONTROL_MASK, "Ctrl+E"),
        (u32::from('R'), CTRL_SHIFT, "Ctrl+Shift+R"),
        (u32::from('e'), CONTROL_MASK | MOD1_MASK, "Ctrl+Alt+E"),
        (u32::from('E'), CTRL_SHIFT | MOD1_MASK, "Ctrl+Alt+Shift+E"),
        (u32::from('c'), CONTROL_MASK, "Ctrl+C"),
    ] {
        let (mut session, log) = session(|_| {});
        let (handled, out) = session.process_key(keyval, state);
        assert!(!handled, "{what} 归应用");
        assert!(sent_keys(&log).is_empty(), "{what} 不送 Router");
        assert!(out.is_empty(), "{what}：{out:?}");
        assert!(session.english_assist());
    }
}

#[test]
fn passthrough_from_the_router_goes_to_the_app() {
    let (mut session, log) = session(|scripted| scripted.recognizes = false);
    let (handled, out) = session.process_key(u32::from('E'), CTRL_SHIFT);
    assert!(!handled, "Router 不认就交回应用");
    assert_eq!(sent_keys(&log).len(), 1);
    assert!(!out.iter().any(|o| matches!(o, Output::Mode { .. })));
}

#[test]
fn disabled_combo_is_not_forwarded() {
    let (mut session, log) = session(|scripted| scripted.keys = None);
    let (handled, _) = session.process_key(u32::from('E'), CTRL_SHIFT);
    assert!(!handled);
    assert!(sent_keys(&log).is_empty());
    assert!(session.property_activate(ASSIST_PROPERTY).is_empty());
}

#[test]
fn custom_combo_follows_the_backend() {
    let combo: KeyCombo = "ctrl+alt+g".parse().unwrap();
    let (mut session, log) = session(|scripted| scripted.keys = Some(combo));
    let (handled, _) = session.process_key(u32::from('E'), CTRL_SHIFT);
    assert!(!handled, "缺省组合不再拦");
    let (handled, _) = session.process_key(u32::from('g'), CONTROL_MASK | MOD1_MASK);
    assert!(handled);
    assert_eq!(sent_keys(&log).len(), 1);
    assert!(!session.english_assist());
}

#[test]
fn property_click_sends_the_combo() {
    let (mut session, log) = session(|_| {});
    let out = session.property_activate(ASSIST_PROPERTY);
    assert!(out.contains(&Output::Mode {
        english: false,
        assist: false
    }));
    let keys = sent_keys(&log);
    assert_eq!(keys.len(), 1);
    assert_eq!(keys[0].character, Some('e'));
    assert!(!keys[0].modifiers.english_mode);
}
