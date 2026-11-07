//! 焦点与生命周期：获焦开会话、失焦 / 重置清掉 Router 的组句、停用与销毁关会话。
//!
//! 失焦 / 重置 / 停用时敲了一半的拼音不由引擎上屏：daemon 在调 `FocusOut` / `Reset` 之前就按 preedit 的
//! `IBUS_ENGINE_PREEDIT_COMMIT` 模式把屏上的 preedit 落进应用（`bus/inputcontext.c` 的
//! `bus_input_context_clear_preedit_text`），随后才把引擎摘走——引擎此时再发 `CommitText` 会落空（Docker 端到端测试实测）。
//! 所以这里只让 Router 清空缓冲、丢掉它交出的原文（对应 DLL 的 `server_stale` 分支）。

use glimmer_platform::protocol::{ClientMessage, Frame};

use super::Session;
use crate::frontend::output::Output;

impl Session {
    /// 获焦（`FocusIn` / `FocusInId`）：记下客户端名、开会话、把中英模式报给 Router，并登记属性。
    pub fn focus_in(&mut self, app: Option<String>) -> Vec<Output> {
        if self.id.is_some() && self.app != app {
            self.close();
        }
        self.app = app;
        let session = self.session();
        self.notify(ClientMessage::ModeChanged {
            session,
            english: self.english,
        });
        vec![self.register_output()]
    }

    /// 失焦（`FocusOut` / `FocusOutId`）：清掉组句（preedit 已由 daemon 落进应用）、关会话，忘掉这个输入框的前文与光标。
    pub fn focus_out(&mut self) -> Vec<Output> {
        let mut out = Vec::new();
        self.discard_pending(&mut out);
        self.close();
        self.before_cursor = None;
        self.cursor = None;
        out
    }

    /// 应用要求重置（点了别处、程序改了文本）：清掉组句（preedit 已由 daemon 落进应用），会话保留。
    pub fn reset(&mut self) -> Vec<Output> {
        let mut out = Vec::new();
        self.discard_pending(&mut out);
        out
    }

    /// 启用（`Enable`）：登记属性，面板显示中 / 英。
    pub fn enable(&self) -> Vec<Output> {
        vec![self.register_output()]
    }

    /// 停用（`Disable`，切到别的输入法）：与失焦相同。
    pub fn disable(&mut self) -> Vec<Output> {
        self.focus_out()
    }

    /// 关掉当前协议会话（引擎被销毁时也调）。
    pub fn close(&mut self) {
        if let Some(session) = self.id.take() {
            self.notify(ClientMessage::CloseSession { session });
        }
        self.composing = false;
        self.predicting = false;
        self.last_frame = Frame::default();
    }

    /// 组句中就让 Router 交出缓冲区原文并清空，上屏并收起 preedit / 候选表（切中英模式时用，那时应用仍聚焦）；
    /// 显示着英文预测时同样经 `Commit` 让 Router 收起（交出的是空的）。
    pub(super) fn commit_pending(&mut self, out: &mut Vec<Output>) {
        if !self.composing && !self.predicting {
            return;
        }
        let Some(session) = self.id else {
            self.render(Frame::default(), out);
            return;
        };
        match self.call(ClientMessage::Commit { session }) {
            Some(reply) => {
                self.apply(reply, out);
            }
            None => self.render(Frame::default(), out),
        }
    }

    /// 组句中或显示着英文预测时让 Router 清空缓冲区、丢掉交出的原文，只收起 preedit / 候选表。
    /// 应用要求重置（点了别处）也走这里：预测是光标原处的，不能留到 Tab。
    fn discard_pending(&mut self, out: &mut Vec<Output>) {
        if !self.composing && !self.predicting {
            return;
        }
        if let Some(session) = self.id {
            self.notify(ClientMessage::Commit { session });
        }
        self.render(Frame::default(), out);
    }
}
