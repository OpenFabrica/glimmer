//! 真后端：进程内的 [`Router`]。与 Windows 不同，没有管道，前端直接调 [`Router::handle`]。

use std::time::{Duration, Instant};

use glimmer_platform::KeyCombo;
use glimmer_platform::protocol::{ClientMessage, ServerMessage};
use glimmer_server::Router;

use super::Backend;

/// 空闲时学习数据落盘的间隔；有消息时 Router 自己按节拍落盘，这里补上长时间不打字的情况。
const IDLE_FLUSH_INTERVAL: Duration = Duration::from_secs(60);

/// 包着 Router 的后端。
pub struct RouterBackend {
    /// 协议分派与 Engine。
    router: Router,

    /// 上次空闲落盘的时间。
    last_flush: Instant,
}

impl RouterBackend {
    /// 包一个装配好的 Router。
    pub fn new(router: Router) -> Self {
        Self {
            router,
            last_flush: Instant::now(),
        }
    }
}

impl Backend for RouterBackend {
    fn send(&mut self, message: ClientMessage) -> Option<ServerMessage> {
        self.router.handle(message)
    }

    fn tick(&mut self) -> Duration {
        self.router.tick();
        if self.last_flush.elapsed() >= IDLE_FLUSH_INTERVAL {
            self.router.flush_learning();
            self.last_flush = Instant::now();
        }
        self.router.next_tick()
    }

    fn shutdown(&mut self) {
        self.router.flush_learning();
    }

    fn english_assist_keys(&self) -> Option<KeyCombo> {
        self.router.english_assist_keys()
    }

    fn english_assist(&self) -> bool {
        self.router.english_assist()
    }
}
