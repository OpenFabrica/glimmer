//! 回显后端：不带词库的假 Router，只为跑通 IBus 链路（bin 的缺省后端、Docker 端到端测试用）。
//! 字母追加进 preedit，唯一的候选是大写形式（带译文 `echo`），空格 / 数字 1 上屏大写，退格删一个，Esc 清空。
//! 缺省的英文辅助组合键（Ctrl+Shift+E）照 Router 的契约翻转开关、吃掉，不进缓冲。

use std::collections::HashMap;

use glimmer_core::{Candidate, CandidateKind, CandidateList, Language, Sense, Translation};
use glimmer_platform::KeyCombo;
use glimmer_platform::protocol::{
    ClientMessage, Frame, KeyEvent, KeyModifiers, KeyOutcome, PreeditKind, PreeditSegment,
    ServerMessage, SessionId,
};

use super::Backend;

/// 按会话记一段缓冲。
#[derive(Debug)]
pub struct EchoBackend {
    /// 会话 → 已敲的字母。
    buffers: HashMap<SessionId, String>,

    /// 英文辅助开着（全局一份，与 Router 一样）。
    assist: bool,
}

impl Default for EchoBackend {
    /// 空缓冲、英文辅助开着（与配置缺省一致）。
    fn default() -> Self {
        Self {
            buffers: HashMap::new(),
            assist: true,
        }
    }
}

impl EchoBackend {
    /// 空后端。
    pub fn new() -> Self {
        Self::default()
    }

    /// 是不是缺省的英文辅助组合：字符小写 + 物理修饰键 Ctrl+Shift。
    fn is_english_assist(event: &KeyEvent) -> bool {
        let combo = KeyCombo::ENGLISH_ASSIST_DEFAULT;
        event.character == Some(combo.key)
            && event.modifiers.chord() == KeyModifiers::from(combo.modifiers)
    }

    /// 处理一次按键，返回 `(处置, 上屏文本)`；缓冲区就地改。
    fn key(buffer: &mut String, event: KeyEvent) -> (KeyOutcome, Option<String>) {
        let composing = !buffer.is_empty();
        match (event.virtual_key, event.character) {
            (_, Some(c)) if c.is_ascii_alphabetic() && !event.modifiers.english_mode => {
                buffer.push(c.to_ascii_lowercase());
                (KeyOutcome::Consumed, None)
            }
            (0x20, _) | (0x31, _) if composing => {
                let text = buffer.to_ascii_uppercase();
                buffer.clear();
                (KeyOutcome::Consumed, Some(text))
            }
            (0x08, _) if composing => {
                buffer.pop();
                (KeyOutcome::Consumed, None)
            }
            (0x1B, _) if composing => {
                buffer.clear();
                (KeyOutcome::Consumed, None)
            }
            _ if composing => (KeyOutcome::Consumed, None),
            _ => (KeyOutcome::Passthrough, None),
        }
    }

    /// 按缓冲区画一帧；空缓冲区是空帧。
    fn frame(buffer: &str) -> Frame {
        if buffer.is_empty() {
            return Frame::default();
        }
        let candidate = Candidate {
            text: buffer.to_ascii_uppercase(),
            kind: CandidateKind::English,
            syllables: Vec::new(),
            reading: None,
            translation: Some(Translation::new(
                Language::English,
                vec![Sense {
                    part_of_speech: None,
                    text: "echo".to_owned(),
                    reading: None,
                    fresh: false,
                }],
            )),
        };
        Frame {
            preedit: vec![PreeditSegment {
                text: buffer.to_owned(),
                kind: PreeditKind::Typed,
            }],
            cursor: buffer.chars().count(),
            candidates: CandidateList {
                items: vec![candidate],
            },
            page_count: 1,
            ..Frame::default()
        }
    }
}

impl Backend for EchoBackend {
    fn send(&mut self, message: ClientMessage) -> Option<ServerMessage> {
        match message {
            ClientMessage::OpenSession { session, .. } => {
                self.buffers.entry(session).or_default();
                None
            }
            ClientMessage::CloseSession { session } => {
                self.buffers.remove(&session);
                None
            }
            ClientMessage::Key { session, event } => {
                let toggles = Self::is_english_assist(&event);
                if toggles {
                    self.assist = !self.assist;
                }
                let buffer = self.buffers.entry(session).or_default();
                let (outcome, commit) = if toggles {
                    (KeyOutcome::Consumed, None)
                } else {
                    Self::key(buffer, event)
                };
                Some(ServerMessage::KeyResult {
                    session,
                    outcome,
                    commit,
                    frame: Self::frame(buffer),
                })
            }
            ClientMessage::Commit { session } => {
                let text = self.buffers.get_mut(&session).map(std::mem::take);
                Some(ServerMessage::Committed {
                    session,
                    text: text.filter(|text| !text.is_empty()),
                })
            }
            ClientMessage::Poll { session } => {
                let buffer = self.buffers.get(&session).map_or("", String::as_str);
                Some(ServerMessage::Update {
                    session,
                    frame: Self::frame(buffer),
                })
            }
            ClientMessage::Selection { session, .. } => Some(ServerMessage::Update {
                session,
                frame: Frame::default(),
            }),
            ClientMessage::SyncMode { session } => Some(ServerMessage::ModeSync {
                session,
                english: None,
            }),
            _ => None,
        }
    }

    fn english_assist(&self) -> bool {
        self.assist
    }
}
