//! 各张表的路径、加载、保存与重建：用户词、按输入串的选择、个人英文词、个人敲错表、个人 n-gram。

use std::io::Write;
use std::path::{Path, PathBuf};

use foldhash::HashMap;
use glimmer_core::storage::write_atomic;
use glimmer_dictionary::{Dictionary, WordList, english_code};

use super::{
    ANY_TOKEN, FrequencyLearner, RAW_MARK, USER_CHOICES_FILE, USER_ENGLISH_FILE, USER_NGRAM_FILE,
    USER_TYPOS_FILE, USER_WORD_FREQUENCY, USER_WORDS_FILE, data_lines, parse_counted_pair,
};
use crate::error::LearningError;

impl FrequencyLearner {
    /// 个人英文词表文件与词频文件同目录。
    pub(super) fn english_path(frequency_path: &Path) -> PathBuf {
        frequency_path.with_file_name(USER_ENGLISH_FILE)
    }

    /// 从 `词\t次数` 读个人英文词，返回跳过的坏行数。
    pub(super) fn load_english(&mut self, source: &str) -> usize {
        let mut skipped = 0;
        for line in data_lines(source) {
            let Some((word, count)) = line
                .split_once('\t')
                .and_then(|(word, count)| Some((word, count.trim().parse::<u32>().ok()?)))
            else {
                skipped += 1;
                continue;
            };
            if count > 0 {
                self.english
                    .insert(word.to_ascii_lowercase(), (word.to_owned(), count));
            }
        }
        self.rebuild_english();
        skipped
    }

    /// 个人英文词很少，每次变化整个重建词表即可。次数当词频，补全时常用的在前。
    ///
    /// 词表编码与随包词表一样用 [`english_code`]（小写、去撇号），敲 `dont` 能查到学过的 `don't`；
    /// 内存里的键仍是小写写法，`dont` 与 `don't` 各记各的次数。去掉撇号后什么都不剩的不进词表。
    pub(super) fn rebuild_english(&mut self) {
        if self.english.is_empty() {
            self.english_list = None;
            return;
        }
        let tsv: String = self
            .english
            .values()
            .filter_map(|(word, count)| {
                let code = english_code(word);
                (!code.is_empty()).then(|| format!("{word}\t{code}\t{count}\n"))
            })
            .collect();
        match WordList::parse(&tsv) {
            Ok(list) => self.english_list = Some(list),
            Err(error) => tracing::warn!(%error, "个人英文词表重建失败"),
        }
    }

    /// 个人英文词条数。
    pub fn english_count(&self) -> usize {
        self.english.len()
    }

    pub(super) fn save_english_to(&mut self, path: &Path) -> Result<(), LearningError> {
        let mut rows: Vec<&(String, u32)> = self.english.values().collect();
        rows.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
        write_atomic(path, |file| {
            writeln!(file, "# 微明个人英文词：词\\t次数")?;
            for (word, count) in rows {
                writeln!(file, "{word}\t{count}")?;
            }
            Ok(())
        })?;
        self.english_dirty = false;
        Ok(())
    }

    /// 按输入串记的表的路径：有方案子目录就放那里，否则与词频文件同目录。
    fn scheme_scoped_path(&self, frequency_path: &Path, file: &str) -> PathBuf {
        match &self.scheme_dir {
            Some(dir) => dir.join(file),
            None => frequency_path.with_file_name(file),
        }
    }

    /// 按输入串记的选择文件：与词频文件同目录，或方案子目录里。
    pub(super) fn choices_path(&self, frequency_path: &Path) -> PathBuf {
        self.scheme_scoped_path(frequency_path, USER_CHOICES_FILE)
    }

    /// 从 `输入串\t词\t次数\t位置` 读按输入串记的选择，返回跳过的坏行数。
    /// 位置列缺省（老文件的三列行）算不分位置，读进 `any` 桶等迁移。
    pub(super) fn load_choices(&mut self, source: &str) -> usize {
        let mut skipped = 0;
        for line in data_lines(source) {
            let Some((input, text, count, token)) = parse_choice_line(line) else {
                skipped += 1;
                continue;
            };
            if count == 0 {
                continue;
            }
            let counts = self
                .choices
                .entry(input.to_owned())
                .or_default()
                .entry(text.to_owned())
                .or_default();
            if !counts.add_token(token, count) {
                skipped += 1;
            }
        }
        // 坏行可能留下全空的条目（认不出位置列），清掉免得占着条数上限
        self.choices.retain(|_, texts| {
            texts.retain(|_, counts| !counts.is_empty());
            !texts.is_empty()
        });
        skipped
    }

    /// 老格式（不分位置）的选择次数一次性拆进句首 / 句中两桶：按个人 n-gram 里这个词的句首占比
    /// `c(<s>, w) / c(w)` 分配（四舍五入），n-gram 不认识这个词就留在 `any`。
    ///
    /// 拆完 `any` 清零，所以对已经是新格式的数据是空操作，重复调用结果一样。
    pub(super) fn migrate_choices(&mut self) {
        let mut migrated = 0usize;
        for texts in self.choices.values_mut() {
            for (text, counts) in texts.iter_mut() {
                let any = counts.any();
                // 原样上屏标记本来就不分位置，不拆
                if any == 0 || text == RAW_MARK {
                    continue;
                }
                let total = self.ngram.count(text);
                if total == 0 {
                    continue;
                }
                let start = self.ngram.pair(None, text);
                let share = split_share(any, start, total);
                counts.split_any(share);
                migrated += 1;
            }
        }
        if migrated > 0 {
            self.choices_dirty = true;
            tracing::info!(entries = migrated, "按输入串记的选择已按句首 / 句中拆开");
        }
    }

    pub(super) fn save_choices_to(&mut self, path: &Path) -> Result<(), LearningError> {
        let mut rows: Vec<(&String, &String, u32, &'static str)> = self
            .choices
            .iter()
            .flat_map(|(input, texts)| {
                texts.iter().flat_map(move |(text, counts)| {
                    counts
                        .rows()
                        .map(move |(count, token)| (input, text, count, token))
                })
            })
            .collect();
        rows.sort_by(|a, b| {
            a.0.cmp(b.0)
                .then_with(|| b.2.cmp(&a.2))
                .then_with(|| a.1.cmp(b.1))
                .then_with(|| a.3.cmp(b.3))
        });
        write_atomic(path, |file| {
            writeln!(file, "# 微明按输入串记的选择：输入串\t词\t次数\t位置")?;
            for (input, text, count, token) in rows {
                writeln!(file, "{input}\t{text}\t{count}\t{token}")?;
            }
            Ok(())
        })?;
        self.choices_dirty = false;
        Ok(())
    }

    /// 个人敲错表：与词频文件同目录，或方案子目录里。
    pub(super) fn typos_path(&self, frequency_path: &Path) -> PathBuf {
        self.scheme_scoped_path(frequency_path, USER_TYPOS_FILE)
    }

    /// 从 `敲的\t要的\t次数` 读个人敲错表，返回跳过的坏行数。
    pub(super) fn load_typos(&mut self, source: &str) -> usize {
        let mut skipped = 0;
        for line in data_lines(source) {
            let Some((typed, intended, count)) = parse_counted_pair(line) else {
                skipped += 1;
                continue;
            };
            if count > 0 {
                self.typos
                    .entry(typed.to_owned())
                    .or_default()
                    .insert(intended.to_owned(), count);
            }
        }
        skipped
    }

    pub(super) fn save_typos_to(&mut self, path: &Path) -> Result<(), LearningError> {
        let mut rows: Vec<(&String, &String, &u32)> = self
            .typos
            .iter()
            .flat_map(|(typed, intended)| {
                intended
                    .iter()
                    .map(move |(syllable, count)| (typed, syllable, count))
            })
            .collect();
        rows.sort_by(|a, b| {
            a.0.cmp(b.0)
                .then_with(|| b.2.cmp(a.2))
                .then_with(|| a.1.cmp(b.1))
        });
        write_atomic(path, |file| {
            writeln!(file, "# 微明个人敲错表：敲的\t要的\t次数")?;
            for (typed, intended, count) in rows {
                writeln!(file, "{typed}\t{intended}\t{count}")?;
            }
            Ok(())
        })?;
        self.typos_dirty = false;
        Ok(())
    }

    /// 个人敲错表条数。
    pub fn typo_count_total(&self) -> usize {
        self.typos.values().map(HashMap::len).sum()
    }

    /// 按输入串记的选择条数。
    pub fn choice_count(&self) -> usize {
        self.choices.values().map(HashMap::len).sum()
    }

    /// 所有按输入串记的计数减半（三个桶一起），去掉减到零的。
    pub(super) fn decay_choices(&mut self) {
        for texts in self.choices.values_mut() {
            texts.retain(|_, counts| {
                counts.halve();
                !counts.is_empty()
            });
        }
        self.choices.retain(|_, texts| !texts.is_empty());
    }

    /// 用户词文件：与词频文件同目录，或方案子目录里（五笔的用户词记的是编码）。
    pub(super) fn words_path(&self, frequency_path: &Path) -> PathBuf {
        self.scheme_scoped_path(frequency_path, USER_WORDS_FILE)
    }

    /// 个人 n-gram 文件与词频文件同目录。
    pub(super) fn ngram_path(frequency_path: &Path) -> PathBuf {
        frequency_path.with_file_name(USER_NGRAM_FILE)
    }

    /// 个人 n-gram 里不同的转移条数（二元对 + 三元条）。
    pub fn ngram_transition_count(&self) -> usize {
        self.ngram.transition_count()
    }

    pub(super) fn save_ngram_to(&mut self, path: &Path) -> Result<(), LearningError> {
        let tsv = self.ngram.to_tsv();
        write_atomic(path, |file| {
            writeln!(file, "# 微明个人 n-gram：前词\t后词\t次数，句首用 <s>")?;
            file.write_all(tsv.as_bytes())
        })?;
        self.ngram_dirty = false;
        Ok(())
    }

    /// 从主词库同款 TSV 读用户词，返回跳过的坏行数。
    pub(super) fn load_words(&mut self, source: &str) -> usize {
        let mut skipped = 0;
        for line in data_lines(source) {
            let mut fields = line.split('\t');
            let (Some(text), Some(pinyin)) = (fields.next(), fields.next()) else {
                skipped += 1;
                continue;
            };
            if text.is_empty() || pinyin.trim().is_empty() {
                skipped += 1;
                continue;
            }
            self.words.insert(text.to_owned(), pinyin.trim().to_owned());
        }
        self.rebuild_words();
        skipped
    }

    /// 用户词很少，每次变化整个重建小词库即可。
    pub(super) fn rebuild_words(&mut self) {
        if self.words.is_empty() {
            self.user_dictionary = None;
            return;
        }
        let tsv: String = self
            .words
            .iter()
            .map(|(text, pinyin)| format!("{text}\t{pinyin}\t{USER_WORD_FREQUENCY}\n"))
            .collect();
        match Dictionary::parse(&tsv) {
            Ok(dictionary) => self.user_dictionary = Some(dictionary),
            Err(error) => tracing::warn!(%error, "用户词库重建失败"),
        }
    }

    pub fn word_count(&self) -> usize {
        self.words.len()
    }

    pub(super) fn save_words_to(&mut self, path: &Path) -> Result<(), LearningError> {
        write_atomic(path, |file| {
            writeln!(file, "# 微明用户词：词\\t拼音\\t词频，与主词库同格式")?;
            for (text, pinyin) in &self.words {
                writeln!(file, "{text}\t{pinyin}\t{USER_WORD_FREQUENCY}")?;
            }
            Ok(())
        })?;
        self.words_dirty = false;
        Ok(())
    }
}

/// 解析选择表的一行：四列是 `输入串\t词\t次数\t位置`，三列是老格式（位置算 `any`）。
fn parse_choice_line(line: &str) -> Option<(&str, &str, u32, &str)> {
    let mut fields = line.split('\t');
    let (Some(input), Some(text), Some(count)) = (fields.next(), fields.next(), fields.next())
    else {
        return None;
    };
    let count = count.trim().parse::<u32>().ok()?;
    let token = fields.next().map_or(ANY_TOKEN, str::trim);
    Some((input, text, count, token))
}

/// 老计数里该分给句首的份额：`any × start / total` 四舍五入，不超过 `any`。
fn split_share(any: u32, start: u32, total: u32) -> u32 {
    if total == 0 {
        return 0;
    }
    let total = u64::from(total);
    let share = (u64::from(any) * u64::from(start) + total / 2) / total;
    u32::try_from(share).unwrap_or(any).min(any)
}
