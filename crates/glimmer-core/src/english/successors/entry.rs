//! 后继缓存里的一条。

/// 上文之后见过的一个后继：模型 token、它的检索编码、这条接续的次数。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SuccessorEntry {
    /// 检索编码（token 去撇号），缓存按它排序，前缀查询两次二分。
    pub(super) code: String,

    /// 模型里的 token（小写、留撇号）。
    pub(super) token: String,

    /// 这条接续在语料里的次数。
    pub(super) count: u32,
}

impl SuccessorEntry {
    /// 检索编码。
    pub fn code(&self) -> &str {
        &self.code
    }

    /// 模型 token。
    pub fn token(&self) -> &str {
        &self.token
    }

    /// 接续次数。
    pub fn count(&self) -> u32 {
        self.count
    }
}
