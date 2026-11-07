//! 按消息类型分派：会话开关、按键、轮询、失焦上屏、选区 / 光标矩形 / 中英模式的通知。

use glimmer_platform::protocol::{
    ClientMessage, Frame, KeyEvent, KeyOutcome, PROTOCOL_VERSION, ServerMessage, SessionId,
};

use super::Router;
use super::key::Effect;
use super::session::SessionInfo;

impl Router {
    pub(super) fn dispatch(&mut self, message: ClientMessage) -> Option<ServerMessage> {
        match message {
            ClientMessage::OpenSession {
                session,
                app,
                protocol,
            } => {
                tracing::debug!(?session, app, protocol, "会话打开");
                if protocol != PROTOCOL_VERSION {
                    tracing::warn!(
                        ?session,
                        app,
                        dll = protocol,
                        server = PROTOCOL_VERSION,
                        "DLL 与 Server 的协议版本不同（应用还没重启、用着旧 DLL？），照常服务"
                    );
                }
                // 同一会话重开（DLL 断线重连）：从干净状态起。
                if self.focused == Some(session) {
                    self.reset_composition();
                    self.focused = None;
                }
                self.sessions.insert(
                    session,
                    SessionInfo {
                        app,
                        private: false,
                    },
                );
                None
            }
            ClientMessage::Key { session, event } => Some(self.handle_key(session, event)),
            ClientMessage::Poll { session } => Some(self.handle_poll(session)),
            ClientMessage::Commit { session } => {
                let text = self.commit_raw_for(session);
                tracing::debug!(?session, ?text, "焦点离开，结束组句");
                Some(ServerMessage::Committed { session, text })
            }
            ClientMessage::Surrounding { session, text } => {
                tracing::trace!(?session, chars = text.chars().count(), "收到光标前文");
                self.set_surrounding(session, text);
                None
            }
            ClientMessage::Privacy { session, private } => {
                tracing::debug!(?session, private, "输入框私密状态");
                self.set_privacy(session, private);
                None
            }
            ClientMessage::Selection {
                session,
                request,
                text,
                rect,
            } => Some(self.handle_selection(session, request, text, rect)),
            ClientMessage::PositionCandidates { session, rect } => {
                self.position_candidates(session, rect);
                None
            }
            ClientMessage::HideCandidates { session } => {
                // 组句在 DLL 侧结束（应用终止组句 / 翻译评审失焦）：只收窗口；缓冲留给下一键的 Commit 清。
                // 显示着英文预测时壳也发它：预测时按了壳不送来的键（空格、退格、方向键、Ctrl 组合……），
                // 那个键已经原样交给应用、Core 没看到，照这些键本来的规则把英文上文作废。
                if self.focused == Some(session) {
                    self.end_translation();
                    if self.end_english_prediction() {
                        self.engine.break_english_context();
                    }
                    self.hide_candidate_window();
                }
                None
            }
            ClientMessage::ModeChanged { session, english } => {
                tracing::debug!(?session, english, "中英模式");
                // 切中英 / 获焦时报模式：预测不跨模式留着
                if self.focused == Some(session) && self.end_english_prediction() {
                    self.hide_candidate_window();
                }
                self.handle_mode_changed(english);
                None
            }
            ClientMessage::SyncMode { session } => Some(ServerMessage::ModeSync {
                session,
                english: self.take_pending_mode(),
            }),
            ClientMessage::ImeSwitched { session } => {
                tracing::debug!(?session, "切成了别的输入法");
                if self.end_english_prediction() {
                    self.hide_candidate_window();
                }
                self.handle_ime_switched();
                None
            }
            ClientMessage::CloseSession { session } => {
                self.sessions.remove(&session);
                if self.focused == Some(session) {
                    self.reset_composition();
                    self.focused = None;
                }
                self.flush_learning();
                tracing::debug!(?session, "会话关闭");
                None
            }
        }
    }

    /// 一次按键：翻译评审优先；显示着英文预测时 Tab 接受、其余键先收起预测再照原来的路走；
    /// 翻译选中文字、英文辅助两个组合键在普通分流之前认；最后看要不要出新的预测。
    fn handle_key(&mut self, session: SessionId, event: KeyEvent) -> ServerMessage {
        self.ensure_focus(session);
        self.notice = None;
        if self.translation.is_some() {
            return self.handle_translation_review(session, &event);
        }
        let visible = self.english_prediction_visible();
        let predicted = self.end_english_prediction();
        if let Some(text) = self.accept_english_prediction_key(predicted && visible, &event) {
            // 接受后上文前移一个词、又是「刚敲过空格」，马上取再下一个
            self.refresh_english_prediction();
            let frame = self.current_frame();
            self.reconcile_candidates(&frame);
            return ServerMessage::KeyResult {
                session,
                outcome: KeyOutcome::Consumed,
                commit: Some(text),
                frame,
            };
        }
        if self.engine.composition().is_empty()
            && self.engine.prediction_enabled()
            && self.matches_translate_combo(&event)
        {
            if predicted {
                self.hide_candidate_window();
            }
            self.selection_seq += 1;
            self.pending_selection = Some(self.selection_seq);
            tracing::debug!(
                ?session,
                request = self.selection_seq,
                "翻译选中文字：请 DLL 读选区"
            );
            return ServerMessage::RequestSelection {
                session,
                request: self.selection_seq,
            };
        }
        // 英文辅助开关要在 apply_key 之前认：那里带 Ctrl / Alt / Win 的组合键一律直通给应用
        let effect = if self.matches_english_assist_combo(&event) {
            self.toggle_english_assist()
        } else {
            self.apply_key(&event)
        };
        let (commit, outcome) = match effect {
            Effect::Changed(commit) => {
                self.recompose();
                (commit, KeyOutcome::Consumed)
            }
            Effect::Navigated => (None, KeyOutcome::Consumed),
            Effect::Passthrough => (None, KeyOutcome::Passthrough),
        };
        // 预测显示着时按的别的键只收起预测：连敲第二个空格 Core 仍算「刚敲过空格」，但那不该把预测再弹出来
        if !predicted {
            self.refresh_english_prediction();
        }
        self.poll_prediction();
        let frame = self.current_frame();
        self.reconcile_candidates(&frame);
        ServerMessage::KeyResult {
            session,
            outcome,
            commit,
            frame,
        }
    }

    /// 云联想轮询：聚焦会话拉一次异步结果回最新一帧，否则回空帧。释义兜底与本地整句模型也借这个节拍收。
    fn handle_poll(&mut self, session: SessionId) -> ServerMessage {
        self.tick();
        let learned = self.engine.poll_glosses();
        if learned > 0 {
            tracing::info!(learned, "释义兜底写入个人释义表");
        }
        let frame = if self.focused == Some(session) {
            self.expire_english_prediction();
            self.poll_prediction();
            let frame = self.current_frame();
            self.reconcile_candidates(&frame);
            frame
        } else {
            Frame::default()
        };
        ServerMessage::Update { session, frame }
    }
}
