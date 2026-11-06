//! 一次英文查询能用的数据来源。

use glimmer_dictionary::{Misspellings, WordList};

use crate::english::{DeadEnds, NgramHistory, SuccessorCache};
use crate::sentence::LanguageModel;

/// 英文候选的数据来源；缺哪个就少那一路，全都没有就没有候选。
#[derive(Clone, Copy)]
pub struct Sources<'a> {
    /// 随包英文词表（词频是 Zipf × 1000）。
    pub main: Option<&'a WordList>,

    /// 个人英文词表（词频是次数）。
    pub personal: Option<&'a WordList>,

    /// 开着的领域词库里的英文名称。
    pub domain: Option<&'a WordList>,

    /// 误拼表。
    pub misspellings: Option<&'a Misspellings>,

    /// 英文语言模型；没有时 `P_模型` 一律用词频换算。
    pub model: Option<&'a dyn LanguageModel>,

    /// 英文上文。
    pub history: NgramHistory<'a>,

    /// 这个上文、这个首字母下预先扫好的后继（见 [`SuccessorCache::prepare`]）；没有就不出这一路。
    pub successors: Option<&'a SuccessorCache>,

    /// 这段组句里已知走不通的编码：纠错变体已知没有时跳过一处 / 两处错误的搜索，两处错误也没结果时记下来。
    pub dead_ends: Option<&'a DeadEnds>,
}

impl<'a> Sources<'a> {
    /// 只有词表、没有模型与上文：测试与中文模式之外的简单用法。
    pub fn lists(
        main: Option<&'a WordList>,
        personal: Option<&'a WordList>,
        domain: Option<&'a WordList>,
    ) -> Self {
        Self {
            main,
            personal,
            domain,
            misspellings: None,
            model: None,
            history: NgramHistory::Unknown,
            successors: None,
            dead_ends: None,
        }
    }
}
