//! 英文下一词预测在前端的走法：预测帧不算组句、只多送一个 Tab；其余键吃不吃与没显示时一样，
//! 不送 Router 的键照常归应用、只通知收起；轮询在预测显示期间开着。后端用一个按脚本答复的假 Router。

use std::sync::{Arc, Mutex};

use glimmer_core::{Candidate, CandidateKind, CandidateList};
use glimmer_linux::backend::{self, Backend};
use glimmer_linux::frontend::{Output, Session};
use glimmer_linux::key::keysym;
use glimmer_linux::key::modifiers::CONTROL_MASK;
use glimmer_platform::protocol::{
    ClientMessage, Frame, KeyOutcome, PreeditKind, PreeditSegment, ServerMessage,
};

/// 假 Router：字母进组句；组句中空格上屏「词 + 空格」并给出预测 `world`；预测显示着时 Tab 上屏 `world `、
/// 之后没有预测；其他键放行。`HideCandidates` / `Commit` 收起预测，`Poll` 当作预测过期。
#[derive(Default)]
struct Scripted {
    /// 收到的消息，按顺序。
    log: Arc<Mutex<Vec<ClientMessage>>>,

    /// 组句缓冲。
    buffer: String,

    /// 预测显示着。
    predicting: bool,
}

impl Scripted {
    /// 当前帧：组句帧、预测帧或空帧。
    fn frame(&self) -> Frame {
        if !self.buffer.is_empty() {
            return Frame {
                preedit: vec![PreeditSegment {
                    text: self.buffer.clone(),
                    kind: PreeditKind::Typed,
                }],
                cursor: self.buffer.chars().count(),
                candidates: list("hello"),
                ..Frame::default()
            };
        }
        if self.predicting {
            return Frame {
                candidates: list("world"),
                ..Frame::default()
            };
        }
        Frame::default()
    }
}

/// 一个英文候选的候选表。
fn list(text: &str) -> CandidateList {
    CandidateList {
        items: vec![Candidate {
            text: text.to_owned(),
            kind: CandidateKind::English,
            syllables: Vec::new(),
            reading: None,
            translation: None,
        }],
    }
}

impl Backend for Scripted {
    fn send(&mut self, message: ClientMessage) -> Option<ServerMessage> {
        self.log.lock().unwrap().push(message.clone());
        match message {
            ClientMessage::Key { session, event } => {
                let shown = std::mem::take(&mut self.predicting);
                let (outcome, commit) = match (event.virtual_key, event.character) {
                    (_, Some(c)) if c.is_ascii_alphabetic() && !event.modifiers.ctrl => {
                        self.buffer.push(c);
                        (KeyOutcome::Consumed, None)
                    }
                    (0x20, _) if !self.buffer.is_empty() => {
                        self.buffer.clear();
                        self.predicting = true;
                        (KeyOutcome::Consumed, Some("hello ".to_owned()))
                    }
                    (0x09, _) if shown => (KeyOutcome::Consumed, Some("world ".to_owned())),
                    _ => (KeyOutcome::Passthrough, None),
                };
                Some(ServerMessage::KeyResult {
                    session,
                    outcome,
                    commit,
                    frame: self.frame(),
                })
            }
            // 轮询时当作预测已过期（Router 的英文上文停手太久），回空帧
            ClientMessage::Poll { session } => {
                self.predicting = false;
                Some(ServerMessage::Update {
                    session,
                    frame: self.frame(),
                })
            }
            ClientMessage::Commit { session } => {
                self.predicting = false;
                self.buffer.clear();
                Some(ServerMessage::Committed {
                    session,
                    text: None,
                })
            }
            ClientMessage::HideCandidates { .. } => {
                self.predicting = false;
                None
            }
            _ => None,
        }
    }
}

/// 建会话、敲 `hello` 再按空格，停在预测显示着；返回会话、消息记录与空格那一键的指令。
fn predicting() -> (Session, Arc<Mutex<Vec<ClientMessage>>>, Vec<Output>) {
    let log = Arc::new(Mutex::new(Vec::new()));
    let backend = backend::shared(Scripted {
        log: Arc::clone(&log),
        ..Scripted::default()
    });
    let mut session = Session::new(backend);
    session.focus_in(Some("test".to_owned()));
    for c in "hello".chars() {
        session.process_key(u32::from(c), 0);
    }
    let (handled, out) = session.process_key(keysym::SPACE, 0);
    assert!(handled);
    log.lock().unwrap().clear();
    (session, log, out)
}

/// 记录里有没有发过 `Key`。
fn sent_key(log: &Arc<Mutex<Vec<ClientMessage>>>) -> bool {
    log.lock()
        .unwrap()
        .iter()
        .any(|message| matches!(message, ClientMessage::Key { .. }))
}

/// 记录里有没有发过 `HideCandidates`。
fn sent_hide(log: &Arc<Mutex<Vec<ClientMessage>>>) -> bool {
    log.lock()
        .unwrap()
        .iter()
        .any(|message| matches!(message, ClientMessage::HideCandidates { .. }))
}

#[test]
fn prediction_frame_is_not_a_composition_and_keeps_polling() {
    let (session, _, out) = predicting();
    assert!(!session.composing(), "预测不算组句");
    assert!(session.predicting());
    assert!(session.polling(), "过期要靠轮询收起");
    assert!(out.contains(&Output::Commit("hello ".to_owned())));
    assert!(
        out.iter()
            .any(|o| matches!(o, Output::Candidates(Some(view)) if view.rows[0].text == "world"))
    );
    // 组句收起那一下发了空 preedit，之后的预测帧不再碰 preedit
    let preedits = out
        .iter()
        .filter(|o| matches!(o, Output::Preedit(_)))
        .count();
    assert_eq!(preedits, 1, "{out:?}");
}

#[test]
fn tab_is_forwarded_and_accepts() {
    let (mut session, log, _) = predicting();
    let (handled, out) = session.process_key(keysym::TAB, 0);
    assert!(handled);
    assert!(sent_key(&log));
    assert_eq!(
        out.iter()
            .filter_map(|o| match o {
                Output::Commit(text) => Some(text.as_str()),
                _ => None,
            })
            .collect::<Vec<_>>(),
        ["world "]
    );
    assert!(out.contains(&Output::Candidates(None)));
    assert!(!session.predicting());
    assert!(
        !out.iter().any(|o| matches!(o, Output::Preedit(_))),
        "预测进出不碰 preedit：{out:?}"
    );
}

#[test]
fn keys_not_forwarded_while_idle_go_to_the_app_and_hide_the_prediction() {
    for (keyval, state, what) in [
        (keysym::BACKSPACE, 0, "退格"),
        (keysym::SPACE, 0, "空格"),
        (keysym::RETURN, 0, "回车"),
        (keysym::ESCAPE, 0, "Esc"),
        (keysym::LEFT, 0, "左"),
        (u32::from('c'), CONTROL_MASK, "Ctrl+C"),
    ] {
        let (mut session, log, _) = predicting();
        let (handled, out) = session.process_key(keyval, state);
        assert!(!handled, "{what} 交给应用");
        assert!(!sent_key(&log), "{what} 不送 Router（与没显示预测时一样）");
        assert!(sent_hide(&log), "{what} 通知 Router 收起预测");
        assert!(out.contains(&Output::Candidates(None)), "{what}：{out:?}");
        assert!(!out.iter().any(|o| matches!(o, Output::Preedit(_))));
        assert!(!session.predicting() && !session.polling());
    }
}

#[test]
fn modifier_press_alone_keeps_the_prediction() {
    let (mut session, log, _) = predicting();
    let (handled, out) = session.process_key(keysym::SHIFT_L, 0);
    assert!(!handled);
    assert!(out.is_empty());
    assert!(!sent_hide(&log));
    assert!(session.predicting());
}

#[test]
fn digit_is_forwarded_as_before_and_its_passthrough_hides_the_prediction() {
    let (mut session, log, _) = predicting();
    let (handled, out) = session.process_key(u32::from('1'), 0);
    assert!(!handled, "Router 放行，应用自己插数字");
    assert!(sent_key(&log), "空组句时数字本来就送 Router（转全角）");
    assert!(out.contains(&Output::Candidates(None)));
    assert!(!session.predicting());
}

#[test]
fn reset_drops_the_prediction() {
    let (mut session, log, _) = predicting();
    let out = session.reset();
    assert!(out.contains(&Output::Candidates(None)));
    assert!(
        log.lock()
            .unwrap()
            .iter()
            .any(|m| matches!(m, ClientMessage::Commit { .. }))
    );
    assert!(!session.predicting());
}

#[test]
fn poll_while_predicting_hides_an_expired_prediction() {
    let (mut session, log, _) = predicting();
    let out = session.poll();
    assert!(
        log.lock()
            .unwrap()
            .iter()
            .any(|m| matches!(m, ClientMessage::Poll { .. })),
        "预测显示着时轮询照开"
    );
    assert!(out.contains(&Output::Candidates(None)), "{out:?}");
    assert!(!out.iter().any(|o| matches!(o, Output::Preedit(_))));
    assert!(!session.polling(), "收起后轮询停下");
}
