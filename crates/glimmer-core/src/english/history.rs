//! 给英文语言模型打分用的上文：分得出「不知道上文」「句首」「只知道前一个词」「知道前两个词」。

use crate::sentence::Context;

/// 英文语言模型的上文。
///
/// 为什么不直接用 [`Context`]：`Context.previous = None` 在 n-gram 模型里是句首，`earlier = None` 会补句首标记，
/// 都说不出「不知道」。英文上文常常是不知道的（刚聚焦、刚动过光标），这时只该用一元概率，
/// 按句首算会把句首常见词（`I`、`the`）错误地抬上去。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NgramHistory<'a> {
    /// 不知道上文：只用一元概率。
    Unknown,

    /// [`Context`] 的原语义：`previous = None` 是句首；`previous` 有值、`earlier = None` 是「前一个词在句首」（补句首标记走三元）。
    Context(Context<'a>),

    /// 只知道前一个词，再前面不知道：只走二元，不补句首标记。
    Previous(&'a str),
}

impl<'a> NgramHistory<'a> {
    /// 句首。
    pub const START: Self = Self::Context(Context::START);

    /// 紧挨着的前一个词；不知道或在句首为 `None`。
    pub fn previous(&self) -> Option<&'a str> {
        match self {
            Self::Unknown => None,
            Self::Context(context) => context.previous,
            Self::Previous(previous) => Some(previous),
        }
    }

    /// 当缓存键用的字符串：不同上文给不同的串。token 里没有空格与 `?`，拼起来不会撞。
    pub fn cache_key(&self) -> String {
        match *self {
            Self::Unknown => String::new(),
            Self::Previous(previous) => format!("? {previous}"),
            Self::Context(context) => match (context.earlier, context.previous) {
                (_, None) => "<s>".to_owned(),
                (None, Some(previous)) => format!("<s> {previous}"),
                (Some(earlier), Some(previous)) => format!("{earlier} {previous}"),
            },
        }
    }

    /// 最接近的 [`Context`]：给只认 `Context` 的缺省实现用。不知道上文时退成句首，这是近似，实现方应覆盖。
    pub fn approximate_context(&self) -> Context<'a> {
        match *self {
            Self::Unknown => Context::START,
            Self::Context(context) => context,
            Self::Previous(previous) => Context::after(previous),
        }
    }
}
