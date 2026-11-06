//! 正在数的一段连续命中（连按 Tab）。

/// 连着几个词位都按 Tab 接受、中间只隔 Tab 自带的空格，算一段。
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Streak {
    /// 这一段已经连着接受了几个词；0 是没有在数的段。
    length: usize,

    /// 上一个接受的词后面只有一个空格：下一个词位再接受就能接上这一段。
    open: bool,
}

impl Streak {
    /// 这个词位按 Tab 接受了。`joined`：这个词前面没有标点；`open`：这个词后面只有一个空格。
    /// 接得上就加长；接不上先返回结束的那一段，再从 1 数。
    pub fn accept(&mut self, joined: bool, open: bool) -> Option<usize> {
        let finished = if self.open && joined {
            None
        } else {
            self.end()
        };
        self.length += 1;
        self.open = open;
        finished
    }

    /// 这个词位没接受（或上文断开）：返回正在数的一段（没有为 `None`），清零。
    pub fn end(&mut self) -> Option<usize> {
        let length = std::mem::take(&mut self.length);
        self.open = false;
        (length > 0).then_some(length)
    }
}
