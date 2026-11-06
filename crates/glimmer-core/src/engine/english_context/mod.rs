//! 英文上文：只记前两个英文词，给英文候选的语言模型打分用。不碰中文的上屏链（那条链还管转移学习和自动造词，空格一来就清）。
//!
//! 状态转移（`now` 是事件发生的时刻）：
//!
//! | 事件 | 之后 |
//! | --- | --- |
//! | 选中英文候选、英文模式原样上屏，上屏的是随包词表里的一个词 | 记这个词（`Words`，前两个词往后挪一格） |
//! | 同上，但上屏的不是词表里的词（未登录词、带数字符号的） | 不知道（训练时未登录词 = 断开但不是句首） |
//! | 上屏中文、emoji、快捷候选 | 不知道 |
//! | 直通空格、逗号引号这类句中标点 | 不变 |
//! | 直通句末标点、回车 | 句首 |
//! | 直通其他字符（字母、数字、`-` `@` 这类） | 不知道 |
//! | 光标移动、失焦、中英切换、组句外退格、壳报告的其他直通功能键 | 不知道 |
//! | 停手超过 [`ENGLISH_CONTEXT_TIMEOUT`] 后的第一个事件 | 先变不知道，再按事件处理 |
//! | `clear()`（Esc 放弃组句） | 不变 |
//!
//! 另记一位「最后一个事件是不是空格」（[`EnglishContext::after_space`]），下一词预测只在它为真时出：
//! 每个事件（上表各行，含组句里的按键）都先把它清掉，只有直通空格把它置上；接受预测（上屏「词 + 空格」）之后也是真。
//! `clear()` 不是事件，不动它。
//!
//! 这次不读应用前文（`SurroundingText`）：各平台读前文的时机不一样，旧快照可能晚到，要按会话和请求代次判断新不新鲜。

mod earlier;
mod hooks;
mod state;

use std::time::{Duration, Instant};

pub(crate) use earlier::Earlier;
pub(crate) use state::ContextState;

use crate::english::NgramHistory;
use crate::sentence::Context;

/// 停手多久之后上文作废：收不到的事件（只用鼠标点到别处）靠它兜底。
pub const ENGLISH_CONTEXT_TIMEOUT: Duration = Duration::from_secs(5);

/// 英文上文：状态与最近一次事件的时刻。
#[derive(Debug, Clone, Default)]
pub struct EnglishContext {
    /// 当前状态。
    state: ContextState,

    /// 最近一次事件（按键、上屏、直通）的时刻；`None` 是还没有过事件。
    touched: Option<Instant>,

    /// 最近一次事件是直通空格（连敲几个空格也算）：下一词预测只在这时出。
    after_space: bool,
}

impl EnglishContext {
    /// 来了一个事件：离上一个事件超过 [`ENGLISH_CONTEXT_TIMEOUT`] 就先作废上文，再记下这一刻。
    /// 「刚敲过空格」一并清掉，是空格的由调用方再置上（[`Self::note_space`]）。
    pub(crate) fn touch(&mut self, now: Instant) {
        if self.expired(now) {
            self.state = ContextState::Unknown;
        }
        self.touched = Some(now);
        self.after_space = false;
    }

    /// 离最近一次事件已经超过 [`ENGLISH_CONTEXT_TIMEOUT`]：下一个事件一来上文就作废。
    pub(crate) fn expired(&self, now: Instant) -> bool {
        self.touched
            .is_some_and(|before| now.saturating_duration_since(before) > ENGLISH_CONTEXT_TIMEOUT)
    }

    /// 这次事件是直通空格（在 [`Self::touch`] 之后调）。
    pub(crate) fn note_space(&mut self) {
        self.after_space = true;
    }

    /// 最近一次事件是不是直通空格。
    pub fn after_space(&self) -> bool {
        self.after_space
    }

    /// 能不能出下一词预测：记着词（不是不知道、不是句首）、最近一次事件是空格、还没过期。
    pub(crate) fn predictable(&self, now: Instant) -> bool {
        self.after_space && matches!(self.state, ContextState::Words { .. }) && !self.expired(now)
    }

    /// 记一个上屏的词（模型 token），前一个词挪成再前一个。
    pub(crate) fn push_word(&mut self, token: String) {
        let earlier = match std::mem::take(&mut self.state) {
            ContextState::Unknown => Earlier::Unknown,
            ContextState::SentenceStart => Earlier::Start,
            ContextState::Words { previous, .. } => Earlier::Word(previous),
        };
        self.state = ContextState::Words {
            earlier,
            previous: token,
        };
    }

    /// 句首。
    pub(crate) fn start_sentence(&mut self) {
        self.state = ContextState::SentenceStart;
    }

    /// 不知道上文。
    pub(crate) fn lose(&mut self) {
        self.state = ContextState::Unknown;
    }

    /// 当前状态。
    #[cfg(test)]
    pub(crate) fn state(&self) -> &ContextState {
        &self.state
    }

    /// 给语言模型打分用的上文。
    pub fn history(&self) -> NgramHistory<'_> {
        match &self.state {
            ContextState::Unknown => NgramHistory::Unknown,
            ContextState::SentenceStart => NgramHistory::START,
            ContextState::Words { earlier, previous } => match earlier {
                Earlier::Unknown => NgramHistory::Previous(previous),
                Earlier::Start => NgramHistory::Context(Context::after(previous)),
                Earlier::Word(earlier) => {
                    NgramHistory::Context(Context::after_two(earlier, previous))
                }
            },
        }
    }

    /// 测试用：把最近一次事件往前挪，模拟停手。
    #[cfg(test)]
    pub(crate) fn backdate(&mut self, by: Duration) {
        if let Some(touched) = self.touched {
            self.touched = touched.checked_sub(by);
        }
    }
}
