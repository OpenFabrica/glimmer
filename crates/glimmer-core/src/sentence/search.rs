//! 一次整句转换的搜索参数。

use super::{Context, ReadingShare};

/// 替代写法的代价到这个数才算「猜用户敲错了」：模糊音（ln 2）是用户自己开的读法，不算猜，不受原样成词保护。
pub const PROTECTED_MIN_COST: f64 = 1.0;

/// 一次整句转换的搜索参数：打包成一个值往 Viterbi 里传，免得签名再长。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Search<'a> {
    /// 全拼句子末尾的单字母也当一个音节读（见 `convert_whole`）。
    pub keep_partial: bool,

    /// 要前几条路径。
    pub paths: usize,

    /// 这段拼音之前的上文（上屏链上的前两个词）：第一个词按它打分，不再一律当句首。
    pub initial: Context<'a>,

    /// 原样成词保护：一条猜敲错的边整个落在「按敲的原样读出的多音节词」里面时多扣这么多；0 为不保护。
    pub protected_extra: f64,

    /// 多音字的读音份额表（见 [`ReadingShare`]）；`None` 为不扣读音份额。五笔编码没有读音，那条路一律 `None`。
    pub reading: Option<&'a ReadingShare>,

    /// 终点上「整段只是一个词、按敲的原样读」的最优走法也带回来，不在前 `paths` 条里就补在最后；
    /// 同一段文字按几个词切、分更高的走法也补上。整句与整词比分用（见 [`WHOLE_WORD_MARGIN`](super::WHOLE_WORD_MARGIN)）。
    pub whole_word: bool,
}

impl Search<'_> {
    /// 只要最优的一条、句首、不保护：单元测试与不经过 Engine 的调用用。
    pub const BEST: Search<'static> = Search {
        keep_partial: false,
        paths: 1,
        initial: Context::START,
        protected_extra: 0.0,
        reading: None,
        whole_word: false,
    };
}
