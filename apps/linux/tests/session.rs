//! 前端会话的集成测试：不经 D-Bus，直接用 IBus 的 keyval / state 驱动 [`Session`]，看它产出的指令与发给后端的消息。

use std::sync::{Arc, Mutex};

use glimmer_linux::EchoBackend;
use glimmer_linux::backend::{self, Backend};
use glimmer_linux::frontend::{MODE_PROPERTY, Output, Session};
use glimmer_linux::key::keysym;
use glimmer_linux::key::modifiers::RELEASE_MASK;
use glimmer_platform::protocol::{ClientMessage, ServerMessage};

/// 记下收到的每条消息，再交给回显后端答复。
struct Recording {
    /// 收到的消息，按顺序。
    log: Arc<Mutex<Vec<ClientMessage>>>,

    /// 真正答复的后端。
    inner: EchoBackend,
}

impl Backend for Recording {
    fn send(&mut self, message: ClientMessage) -> Option<ServerMessage> {
        self.log.lock().unwrap().push(message.clone());
        self.inner.send(message)
    }
}

/// 建一个带记录的会话。
fn session() -> (Session, Arc<Mutex<Vec<ClientMessage>>>) {
    let log = Arc::new(Mutex::new(Vec::new()));
    let backend = backend::shared(Recording {
        log: Arc::clone(&log),
        inner: EchoBackend::new(),
    });
    (Session::new(backend), log)
}

/// 敲下再松开一个键，返回按下时吃不吃与全部指令。
fn tap(session: &mut Session, keyval: u32) -> (bool, Vec<Output>) {
    let (handled, mut out) = session.process_key(keyval, 0);
    let (_, released) = session.process_key(keyval, RELEASE_MASK);
    out.extend(released);
    (handled, out)
}

/// 指令里所有上屏文本，按顺序。
fn commits(outputs: &[Output]) -> Vec<String> {
    outputs
        .iter()
        .filter_map(|output| match output {
            Output::Commit(text) => Some(text.clone()),
            _ => None,
        })
        .collect()
}

#[test]
fn typing_letters_then_space_commits_uppercase() {
    let (mut session, _) = session();
    session.focus_in(Some("test".to_owned()));
    let (handled, out) = tap(&mut session, u32::from('a'));
    assert!(handled);
    assert!(out.iter().any(|output| matches!(
        output,
        Output::Preedit(Some(view)) if view.text == "a" && view.cursor == 1
    )));
    assert!(session.composing());
    tap(&mut session, u32::from('b'));
    let (handled, out) = tap(&mut session, keysym::SPACE);
    assert!(handled);
    assert_eq!(commits(&out), vec!["AB".to_owned()]);
    assert!(out.contains(&Output::Preedit(None)));
    assert!(out.contains(&Output::Candidates(None)));
    assert!(!session.composing());
}

#[test]
fn candidates_carry_the_annotation() {
    let (mut session, _) = session();
    let (_, out) = tap(&mut session, u32::from('x'));
    let view = out
        .iter()
        .find_map(|output| match output {
            Output::Candidates(Some(view)) => Some(view.clone()),
            _ => None,
        })
        .expect("应有候选表");
    assert_eq!(view.rows[0].text, "X");
    assert_eq!(view.rows[0].annotation.as_deref(), Some("echo"));
}

#[test]
fn idle_keys_pass_through_without_touching_the_backend() {
    let (mut session, log) = session();
    session.focus_in(None);
    log.lock().unwrap().clear();
    let (handled, out) = tap(&mut session, keysym::RETURN);
    assert!(!handled);
    assert!(out.is_empty());
    assert!(log.lock().unwrap().is_empty());
}

#[test]
fn lone_shift_toggles_mode_and_commits_pending_text() {
    let (mut session, log) = session();
    session.focus_in(None);
    tap(&mut session, u32::from('n'));
    let (_, out) = tap(&mut session, keysym::SHIFT_L);
    assert_eq!(commits(&out), vec!["n".to_owned()]);
    assert!(out.contains(&Output::Mode {
        english: true,
        assist: true
    }));
    assert!(session.english());
    assert!(
        log.lock()
            .unwrap()
            .iter()
            .any(|message| matches!(message, ClientMessage::ModeChanged { english: true, .. }))
    );
    // 英文模式下回显后端放行字母。
    let (handled, _) = tap(&mut session, u32::from('a'));
    assert!(!handled);
}

#[test]
fn shift_chord_does_not_toggle_mode() {
    let (mut session, _) = session();
    session.process_key(keysym::SHIFT_L, 0);
    session.process_key(u32::from('A'), 1);
    session.process_key(u32::from('A'), 1 | RELEASE_MASK);
    let (_, out) = session.process_key(keysym::SHIFT_L, 1 | RELEASE_MASK);
    assert!(out.is_empty());
    assert!(!session.english());
}

#[test]
fn property_click_toggles_mode() {
    let (mut session, _) = session();
    assert_eq!(
        session.property_activate(MODE_PROPERTY),
        vec![Output::Mode {
            english: true,
            assist: true
        }]
    );
    assert!(session.property_activate("Other").is_empty());
}

#[test]
fn focus_out_clears_the_router_buffer_and_closes_the_session() {
    let (mut session, log) = session();
    session.focus_in(None);
    tap(&mut session, u32::from('h'));
    tap(&mut session, u32::from('i'));
    let out = session.focus_out();
    // preedit 由 daemon 按 PREEDIT_COMMIT 落进应用，引擎自己不再上屏。
    assert!(commits(&out).is_empty());
    assert!(out.contains(&Output::Preedit(None)));
    assert!(!session.composing());
    let log = log.lock().unwrap();
    let tail: Vec<&ClientMessage> = log.iter().rev().take(2).collect();
    assert!(matches!(tail[0], ClientMessage::CloseSession { .. }));
    assert!(matches!(tail[1], ClientMessage::Commit { .. }));
}

#[test]
fn surrounding_text_and_cursor_are_sent_when_composition_starts() {
    let (mut session, log) = session();
    session.focus_in(None);
    session.set_surrounding_text("今天天气", 4);
    session.set_cursor_location(10, 20, 2, 16);
    log.lock().unwrap().clear();
    tap(&mut session, u32::from('h'));
    tap(&mut session, u32::from('a'));
    let log = log.lock().unwrap();
    let surrounding: Vec<&str> = log
        .iter()
        .filter_map(|message| match message {
            ClientMessage::Surrounding { text, .. } => Some(text.as_str()),
            _ => None,
        })
        .collect();
    assert_eq!(surrounding, vec!["今天天气"]);
    assert!(log.iter().any(|message| matches!(
        message,
        ClientMessage::PositionCandidates { rect, .. } if rect.right == 12 && rect.bottom == 36
    )));
}

#[test]
fn password_fields_are_reported_private() {
    let (mut session, log) = session();
    session.focus_in(None);
    session.set_content_type(8, 0);
    session.set_content_type(8, 0);
    let private: Vec<bool> = log
        .lock()
        .unwrap()
        .iter()
        .filter_map(|message| match message {
            ClientMessage::Privacy { private, .. } => Some(*private),
            _ => None,
        })
        .collect();
    assert_eq!(private, vec![true]);
}

#[test]
fn candidate_click_selects_like_a_digit_key() {
    let (mut session, _) = session();
    tap(&mut session, u32::from('o'));
    let out = session.candidate_clicked(0);
    assert_eq!(commits(&out), vec!["O".to_owned()]);
}

#[test]
fn poll_skips_unchanged_frames() {
    let (mut session, _) = session();
    assert!(session.poll().is_empty());
    tap(&mut session, u32::from('k'));
    assert!(session.poll().is_empty());
}
