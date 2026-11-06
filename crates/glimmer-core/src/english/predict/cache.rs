//! 预测结果的缓存：壳在轮询里反复问同一个上文，只在上文变了时重算。

use glimmer_dictionary::WordList;

use super::{NextWord, PredictParams, predict};
use crate::english::NgramHistory;
use crate::sentence::LanguageModel;

/// 最近一个上文的预测。换模型、换词表、换参数时调用方要 [`Self::clear`]。
#[derive(Debug, Default)]
pub struct PredictionCache {
    /// 缓存的是哪个上文（[`NgramHistory::cache_key`]）；`None` 是空缓存。
    key: Option<String>,

    /// 那个上文下的预测，概率从高到低。
    words: Vec<NextWord>,
}

impl PredictionCache {
    /// `history` 下的预测：和上次是同一个上文就直接给缓存，否则重算一次再记下。
    pub fn get(
        &mut self,
        model: &dyn LanguageModel,
        list: &WordList,
        history: NgramHistory<'_>,
        params: &PredictParams,
    ) -> &[NextWord] {
        let key = history.cache_key();
        if self.key.as_deref() != Some(key.as_str()) {
            self.words = predict(model, list, history, params);
            self.key = Some(key);
        }
        &self.words
    }

    /// 清空。
    pub fn clear(&mut self) {
        *self = Self::default();
    }
}
