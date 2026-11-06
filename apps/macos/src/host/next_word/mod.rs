//! 英文下一词预测的显示：什么时候画、画在哪、什么时候收。预测出不出、出什么全由 Core 定
//! （[`glimmer_core::Engine::english_predictions`]），这里只把它画到插入点旁边、定时发现过期、按键前收起。
//!
//! 窗口没有 preedit、只有候选行，与「翻译选中文字」一样借用候选窗口与 [`super::Session`]；
//! 应用里不放 marked text。

mod monitor;
mod phase;

pub(super) use monitor::NextWordMonitor;
pub use phase::NextWordPhase;

use objc2::rc::Retained;
use objc2::runtime::AnyObject;
use objc2_foundation::NSRect;

use super::{Host, with};
use crate::imk::{TextClient, modifiers, secure_input};

impl Host {
    /// 当前显示阶段；控制器据此决定 Tab 交不交给 Core、按键前要不要收起。
    pub fn next_word_phase(&self) -> NextWordPhase {
        self.next_word
    }

    /// 每个按键开头调一次：记下这一键是不是按在预测上（见 [`NextWordPhase::declined_by_key`]），返回当前阶段。
    pub fn begin_next_word_key(&mut self) -> NextWordPhase {
        self.next_word_declined = self.next_word.declined_by_key();
        self.next_word
    }

    /// 这一键按下时预测在显示或等着画：键后直通的空格不再取预测，免得同一个词马上又弹出来。
    pub fn next_word_declined(&self) -> bool {
        self.next_word_declined
    }

    /// 取一次预测（直通空格之后、Tab 接受之后）。有预测就让定时器下一拍到插入点去画；没有就收起。
    pub fn request_next_word(&mut self) {
        let before = self.next_word;
        self.next_word = before.requested(self.next_word_available());
        self.settle_next_word(before);
    }

    /// 收起预测（按键、失焦、鼠标、切中英、配置热加载……）；没在显示也没在等就什么都不做，
    /// 免得收掉别人（提示、翻译）正在用的窗口。
    pub fn dismiss_next_word(&mut self, reason: &'static str) {
        if self.next_word == NextWordPhase::Idle {
            return;
        }
        tracing::debug!(reason, "收起下一词预测");
        let before = self.next_word;
        self.next_word = NextWordPhase::Idle;
        self.settle_next_word(before);
    }

    /// 现在能不能出预测：密码框里不出；Caps Lock 切回中文 Core 不知道（它的英文模式要等下一个字符键才改），这里补上；
    /// 其余条件（开关、组句为空、上文、过期）都在 Core 里。
    fn next_word_available(&self) -> bool {
        !secure_input::enabled()
            && modifiers::english_mode()
            && !self.engine.english_predictions().items.is_empty()
    }

    /// 定时器一拍。返回要读插入点的客户端（等着画、还没画）；读插入点要等应用回话，交给调用方放在借用之外。
    fn tick_next_word(&mut self) -> Option<Retained<AnyObject>> {
        let before = self.next_word;
        let next = before.tick(self.next_word_available());
        if matches!(next, NextWordPhase::Pending { .. }) {
            if let Some(client) = self.active_client.clone() {
                return Some(client);
            }
            self.dismiss_next_word("没有当前客户端");
            return None;
        }
        self.next_word = next;
        self.settle_next_word(before);
        None
    }

    /// 把预测画到 `anchor`（插入点矩形）旁边。两次借用之间可能已经收起或条件变了，再核对一遍。
    fn show_next_word(&mut self, anchor: NSRect) {
        if !matches!(self.next_word, NextWordPhase::Pending { .. }) {
            return;
        }
        let mut predictions = self.engine.english_predictions();
        if predictions.items.is_empty() || !self.next_word_available() {
            self.dismiss_next_word("预测没了");
            return;
        }
        self.engine.annotate(&mut predictions);
        self.anchor = anchor;
        self.reset_session(None, predictions.items);
        self.render();
        self.next_word = self.next_word.shown();
    }

    /// 阶段变了之后的收尾：回到 `Idle` 停表，窗口上挂着预测的话清会话、收窗；等着画或显示着就保证定时器在跑。
    fn settle_next_word(&mut self, before: NextWordPhase) {
        if self.next_word == NextWordPhase::Idle {
            self.next_word_monitor.stop();
            if before.visible() {
                self.reset_session(None, Vec::new());
                self.window.hide();
            }
        } else {
            self.next_word_monitor.start();
        }
    }
}

/// 定时器回调：先借 Host 推进阶段；要画时放开借用去读插入点（等应用回话期间 IMK 可能重入），再借回来画。
fn poll_next_word() {
    let Some(client) = with(|h| h.tick_next_word()).flatten() else {
        return;
    };
    let anchor = TextClient::new(&client).caret_rect();
    with(|h| h.show_next_word(anchor));
}
