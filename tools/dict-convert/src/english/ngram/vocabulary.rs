//! 英文语言模型的词表：`english.tsv` 里的词转小写去重，编号给计数用。

use std::collections::HashMap;
use std::path::Path;

use glimmer_dictionary::WordList;
use glimmer_lm::SENTENCE_START;

use crate::error::ConvertError;

/// 小写词 → 编号；0 号是句首标记 `<s>`。
pub(super) struct EnglishVocabulary {
    /// 小写词 → 编号。
    ids: HashMap<String, u32>,

    /// 编号 → 小写词。
    pub(super) words: Vec<String>,
}

impl EnglishVocabulary {
    /// 从一串词建表：转小写（撇号保留）后去重，`I` 与 `i`、`OR` 与 `or` 是同一个 token，`we'll` 与 `well` 不是。
    pub(super) fn from_words<'a>(words: impl IntoIterator<Item = &'a str>) -> Self {
        let mut vocabulary = Self {
            ids: HashMap::new(),
            words: vec![SENTENCE_START.to_owned()],
        };
        for word in words {
            let lowercase = word.to_ascii_lowercase();
            if vocabulary.ids.contains_key(&lowercase) {
                continue;
            }
            let id = vocabulary.words.len() as u32;
            vocabulary.ids.insert(lowercase.clone(), id);
            vocabulary.words.push(lowercase);
        }
        vocabulary
    }

    /// 读 `english.tsv` 建表。
    pub(super) fn load(path: &Path) -> Result<Self, ConvertError> {
        let list = WordList::from_path(path)?;
        Ok(Self::from_words(list.entries().map(|(_, word, _)| word)))
    }

    /// 切词给出的词（已小写）的编号；不在词表里返回 `None`，调用方当断开。
    pub(super) fn id(&self, word: &str) -> Option<u32> {
        self.ids.get(word).copied()
    }

    /// 词数（含句首标记）。
    pub(super) fn len(&self) -> usize {
        self.words.len()
    }
}
