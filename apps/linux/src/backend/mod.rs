//! 前端与引擎之间的接缝：IBus 层只认 [`Backend`]，不关心背后是进程内的 Router 还是测试用的假实现（[`EchoBackend`]）。

mod echo;
mod router;

use std::sync::{Arc, Mutex};
use std::time::Duration;

use glimmer_platform::KeyCombo;
use glimmer_platform::protocol::{ClientMessage, ServerMessage};

pub use echo::EchoBackend;
pub use router::RouterBackend;

/// 协议的一端：收 [`ClientMessage`]，需要回话的消息回 [`ServerMessage`]。语义与 Windows DLL ↔ Server 的管道一致，
/// 只是换成了进程内调用。
pub trait Backend {
    /// 发一条消息。`Key` / `Commit` / `Poll` / `Selection` 这类一问一答的消息返回答复；
    /// `OpenSession` / `ModeChanged` / `PositionCandidates` 这类通知返回 `None`。
    fn send(&mut self, message: ClientMessage) -> Option<ServerMessage>;

    /// 空闲节拍：没有消息进来时也要推进的事（接上加载好的模型、看配置文件改没改、落盘学习数据）。
    /// 返回下一次最晚什么时候再调。缺省什么都不做、一秒后再来。
    fn tick(&mut self) -> Duration {
        Duration::from_secs(1)
    }

    /// 进程退出前调一次：把还没落盘的东西写下去。缺省什么都不做。
    fn shutdown(&mut self) {}

    /// 开关英文辅助的组合键（`[shortcut] english_assist`），前端只把这一个带修饰键的字母先送来判断；
    /// `None` 是禁用。缺省用产品缺省组合 Ctrl+Shift+E。
    fn english_assist_keys(&self) -> Option<KeyCombo> {
        Some(KeyCombo::ENGLISH_ASSIST_DEFAULT)
    }

    /// 英文模式这时带不带补全、纠错、预测（`[general] english_candidates`），属性面板据此显示「英+」。缺省开着。
    fn english_assist(&self) -> bool {
        true
    }
}

/// 所有引擎实例共用的一个后端（Router 本来就按会话分派状态）。
pub type SharedBackend = Arc<Mutex<dyn Backend + Send>>;

/// 把一个后端包成 [`SharedBackend`]。
pub fn shared(backend: impl Backend + Send + 'static) -> SharedBackend {
    Arc::new(Mutex::new(backend))
}
