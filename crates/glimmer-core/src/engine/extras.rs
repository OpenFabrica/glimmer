//! 附加候选：日期时间等快捷项、中英混输的英文词与补全、emoji。

use glimmer_dictionary::WordList;

use super::learning::Learner;
use super::{
    EMOJI_MIN_INDEX, EMOJI_PER_WORD, EMOJI_SCAN, EMOJI_TOTAL, ENGLISH_COMPLETIONS, Engine,
    MIN_COMPLETION_LETTERS,
};
use crate::candidate::{Candidate, CandidateKind};
use crate::shortcut;

impl Engine {
    /// 精确匹配自定义输入码时，数字键应选择候选。
    pub(super) fn has_custom_phrase(&self) -> bool {
        !self.english_mode
            && self
                .custom_phrases
                .iter()
                .any(|p| p.enabled && p.code == self.composition.scope())
    }

    /// 所有普通候选完成排序后按输入码固定位置。
    pub(super) fn insert_custom_phrases(&self, items: &mut Vec<Candidate>) {
        if self.english_mode {
            return;
        }
        let mut phrases: Vec<_> = self
            .custom_phrases
            .iter()
            .filter(|p| p.enabled && p.code == self.composition.scope())
            .collect();
        phrases.sort_by_key(|p| p.position);
        for phrase in phrases {
            items.insert(
                (phrase.position - 1).min(items.len()),
                Candidate {
                    text: phrase.text.clone(),
                    kind: CandidateKind::Custom(phrase.position),
                    syllables: Vec::new(),
                    reading: None,
                    translation: None,
                },
            );
        }
    }

    /// 日期 / 时间 / 星期这类快捷候选插在本地首选之后：`rq` 首选仍是词库里的词，快捷写法紧随其后。
    pub(super) fn insert_shortcuts(&self, items: &mut Vec<Candidate>, scope: &str) {
        // 只用形码时 `rq` 是编码，反查的拼音也不当快捷键；混输下拼音那条路照常给快捷候选
        if self.code_only() {
            return;
        }
        let expression_char =
            if self.zhuyin && crate::zhuyin::layout::map_key(self.modes().expression).is_some() {
                '\0'
            } else {
                self.modes().expression
            };
        let shortcuts = shortcut::candidates(scope, expression_char, &jiff::Zoned::now());
        if shortcuts.is_empty() {
            return;
        }
        let position = items.len().min(1);
        items.splice(position..position, shortcuts);
    }

    /// 中英混输：整段输入是英文词就把它加进候选。
    /// 缺省作为拼音「不像话」（切不动、或除末尾外还有声母缩写 / 残缺音节）时排第一，否则排第二；
    /// 开了中文优先（`chinese_first`）整句 / 首个中文候选已经在前，英文词排第二。没有中文候选时总在第一。
    pub(super) fn insert_english(&self, items: &mut Vec<Candidate>, unlikely_pinyin: bool) {
        // 只用形码时没有中英混输候选：编码与英文词撞形太多；用户关了中文模式英文候选也不出
        if self.code_only() || !self.mixed_english {
            return;
        }
        let lists = self.english_lists();
        if lists.is_empty() {
            return;
        }
        let text = self.composition.scope();
        if text.contains('\'') {
            return;
        }
        let english_candidate = |word: &str| Candidate {
            text: word.to_owned(),
            kind: CandidateKind::English,
            syllables: Vec::new(),
            reading: None,
            translation: None,
        };
        let word = lists.iter().find_map(|words| words.get(text));
        // 这段字母下用户选中文词（`key` → 可以）比选英文词的次数多：中文词留在第一，英文让到后面；
        // 拼音再不像话也是他自己教的
        // 这里比的是不分位置的总次数：中英取舍是语言偏好，与句首句中无关，
        // 按位置分桶数会让同一段字母的英文候选名次随上文跳
        let chosen = items
            .first()
            .filter(|c| c.kind == CandidateKind::Chinese)
            .map_or(0, |c| self.learner.choice_total(text, &c.text));
        let english_weight = word.map_or(0, |w| self.learner.weight(w));
        let english_first = !self.chinese_first && unlikely_pinyin && chosen <= english_weight;
        let mut position = if items.is_empty() || english_first {
            0
        } else {
            1
        };
        if let Some(word) = word {
            items.insert(position, english_candidate(word));
            position += 1;
        }
        // 英文补全：拼音不像话时（`compa` 切成 co'm'pa），整段多半是在打英文词的前面几个字母，补全紧跟在精确词之后；
        // 个人词表在前，两张表里都有的只出一次
        if unlikely_pinyin && text.len() >= MIN_COMPLETION_LETTERS {
            let mut budget = ENGLISH_COMPLETIONS;
            for words in &lists {
                for word in words.complete(text, budget) {
                    if items
                        .iter()
                        .any(|c| c.kind == CandidateKind::English && c.text == word)
                    {
                        continue;
                    }
                    items.insert(position, english_candidate(word));
                    position += 1;
                    budget -= 1;
                }
                if budget == 0 {
                    break;
                }
            }
        }
    }

    /// 给英文候选用的词表，个人的在前、领域的其次、随包的在后；一张都没有就是空。
    pub(super) fn english_lists(&self) -> Vec<&WordList> {
        self.learner
            .user_english()
            .into_iter()
            .chain((!self.extra_english.is_empty()).then_some(&self.extra_english))
            .chain(self.english.as_ref())
            .collect()
    }

    /// emoji 候选：前几个中文候选里有配 emoji 的，emoji 跟在那个词后面，右侧标注它对应的词；
    /// 但最早排在下标 [`EMOJI_MIN_INDEX`]，前三位留给字词（首选配了 emoji 时 emoji 从第四位起），
    /// 也不插进一串快捷写法或英文词中间（见 [`emoji_slot`]）。
    /// 词后面紧挨着的英文词候选（中文优先时 `key` → 可以、key）不被 emoji 挤开，emoji 排在它之后。
    /// 用户关了 emoji 候选（`emoji_candidates`）时什么都不插。
    pub(super) fn insert_emoji(&self, items: &mut Vec<Candidate>) {
        if !self.emoji_candidates {
            return;
        }
        let Some(table) = &self.emoji else { return };
        // 先在原列表上定好每个 emoji 跟在哪里（插入下标），扫完再统一插：插的位置要往后挪到前几位之外，
        // 边扫边插会让后挪的 emoji 夹进还没扫到的词中间
        let mut planned: Vec<(usize, Candidate)> = Vec::new();
        let mut index = 0;
        let mut scanned = 0;
        while index < items.len() && scanned < EMOJI_SCAN && planned.len() < EMOJI_TOTAL {
            let item = &items[index];
            index += 1;
            if !matches!(
                item.kind,
                CandidateKind::Chinese | CandidateKind::Sentence | CandidateKind::English
            ) {
                continue;
            }
            scanned += 1;
            // 英文词按小写查英文表（smile → 😀）
            let key = if item.kind == CandidateKind::English {
                item.text.to_lowercase()
            } else {
                item.text.clone()
            };
            let emojis = table.lookup(&key);
            if emojis.is_empty() {
                continue;
            }
            let word = item.text.clone();
            let syllables = item.syllables.clone();
            if item.kind != CandidateKind::English {
                while items
                    .get(index)
                    .is_some_and(|next| next.kind == CandidateKind::English)
                {
                    index += 1;
                }
            }
            for emoji in emojis.iter().take(EMOJI_PER_WORD) {
                if planned.len() >= EMOJI_TOTAL {
                    break;
                }
                planned.push((
                    index,
                    Candidate {
                        text: emoji.clone(),
                        kind: CandidateKind::Emoji,
                        syllables: syllables.clone(),
                        reading: Some(word.clone()),
                        translation: None,
                    },
                ));
            }
        }
        // 插入下标都按插之前的列表算好再从后往前插：前面的下标不受影响；同一个位置上先插的会被后插的推到后面，
        // 所以倒着插顺序不乱。挪位只会往后、且不越过后面的计划位置，先后关系不变
        let slots: Vec<usize> = planned
            .iter()
            .map(|(index, _)| emoji_slot(items, *index))
            .collect();
        for (slot, (_, emoji)) in slots.into_iter().zip(planned).rev() {
            items.insert(slot, emoji);
        }
    }
}

/// emoji 的插入下标：`index` 是紧跟对应词的位置，最早挪到 [`EMOJI_MIN_INDEX`]（前三位留给字词）；
/// 落在一串快捷写法或英文词中间时挪到这一串之后（`sj` 的三种时间写法、英文词与它的补全不被拆开）。列表不够长时排在末尾。
fn emoji_slot(items: &[Candidate], index: usize) -> usize {
    let mut slot = index.max(EMOJI_MIN_INDEX).min(items.len());
    while slot > 0
        && slot < items.len()
        && items[slot].kind == items[slot - 1].kind
        && matches!(
            items[slot].kind,
            CandidateKind::Shortcut | CandidateKind::English
        )
    {
        slot += 1;
    }
    slot
}
