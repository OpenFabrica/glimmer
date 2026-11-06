//! 英文词表 `english.tsv`：`词\t编码\t词频`，一个编码可以对多个词，同码的第一个是这个编码的缺省写法
//! （`WordList::get` 取它，引擎把它当「敲这串字母原样上屏的词」）。
//!
//! 源表（`00_all_words.tsv`，ESDB / CSpell）每行 `词\t编码[\t…]`，表头行（第一列是 `word`）指出 `frequency_level` 在哪一列，`#` 为注释。
//!
//! **收什么。** ASCII 字母、中间可夹单个撇号、首尾是字母的写法（`don't`、`I'm`、`rock'n'roll`，见 [`spelling::is_accepted`]）。
//! 编码一律用 `glimmer_dictionary::english_code` 现算（小写、去撇号），源表的 key 列不用。
//! 同一个词（逐字节相同）只留第一次出现的那条；不同写法都留（`or` 与 `OR`、`may` 与 `May`、`well` 与 `we'll`）。
//!
//! **同码先后**（[`Spelling::precedence`]）：
//!
//! 1. 不带撇号的在前：敲的字母原样就是它，正常敲字不会被改成撇号词（`well` 在 `we'll` 前，`cant` 在 `can't` 前）。
//! 2. ESDB 常见度等级小的在前，没有等级的（CSpell 技术词）排后：`I`(35) 在 `i`(40) 前、`John`(35) 在 `john`(40) 前、
//!    `API`(50) 在 `api`(CSpell) 前。
//! 3. 全小写在前：同等级时普通词优先于专名 / 缩写（`or` 在 `OR` 前、`may` 在 `May` 前，两边都是 35）。
//! 4. 读入先后。
//!
//! 不按「哪个文件先读」定先后：总表按词典序排（大写在前），旧规则同码先到先得就成了 `OR` / `May`；
//! 换成「常用表在前、专名表在后」又会让 `i` 压过 `I`、`john` 压过 `John`。ESDB 等级说的正是哪个写法更基础。
//!
//! **展示写法**（`--display-forms`，`07_display_forms.tsv`）是**替换**：写法 D 放到同码第一位，同码里与 D 只差大小写的
//! 其它写法删掉（`typescript`、`Typescript` 不留），撇号词这类别的词不动。源表里没有这个编码就新加一组。
//! 它在先后规则排好之后才生效，所以 D 一定是缺省写法。
//!
//! **词频**（wordfreq 的 Zipf × 1000，频率表按小写词查，含撇号：`i'm`）：
//!
//! - 带撇号的词，词频表里没有或为 0 的**不收**：这类几乎全是 wordfreq 不认识的所有格（`aardvark's`，约 8,800 条），
//!   给底值会排到几万条真实但罕见的词前面，是噪声；wordfreq 认识的（`don't`、`it's`、`today's`）照收。
//!   这条只在给了 `--frequency` 时生效：第一遍没有词频表，全收，好让 `english_frequency.py` 给它们算词频。
//! - 不带撇号的词（技术词那一类），词频表里没有或为 0 的给底值（`--frequency-floor`，缺省 3000 即 Zipf 3）。
//! - 缺省写法：自己的词频。
//! - 与缺省写法只差大小写的（`OR`、`May`、`i`）：缺省写法的词频减 [`CASE_VARIANT_DISCOUNT`]（Zipf 少 1，概率的十分之一）。
//!   wordfreq 不分大小写，两个写法本来同分，不折扣的话 `or` 与 `OR` 谁先全看排序是否稳定。
//! - 其它写法（`we'll`、`it's`、所有格 `dog's`）：自己的词频，但不超过缺省写法（`min`）。
//!   它们是另一个词，有自己的真实词频；封顶只为保证缺省写法的词频不低于同码别的写法，先后交给引擎按上文定。
//!
//! **语料大小写**（`--case-counts`，`english-case` 的输出，可选）：上面第 2、3 条是猜的，`google` / `Google` 这类猜不准。
//! 给了它，「只差大小写」的写法按语料里原样写法的次数重排，词频也按次数比定（见 [`case`] 模块）；不给就是上面的规则。

mod case;
mod options;
mod spelling;

use std::collections::{HashMap, HashSet};
use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::Path;

use glimmer_dictionary::english_code;

use crate::error::ConvertError;

use case::{CaseStats, read_case_counts, reorder, variant_frequency};
pub use options::WordsOptions;
use spelling::{Spelling, is_accepted};

/// 与缺省写法只差大小写的写法少给的词频（Zipf × 1000 的单位，1000 = 一个 Zipf）。
const CASE_VARIANT_DISCOUNT: u32 = 1000;

/// 同一个编码的全部写法，按读入先后。
struct Group {
    /// 检索编码。
    code: String,

    /// 写法；排序后第一个是缺省写法。
    spellings: Vec<Spelling>,
}

/// 读源表与展示写法表，排好同码先后，配上词频写出 `english.tsv`。
pub fn convert(options: &WordsOptions) -> Result<(), ConvertError> {
    let frequencies = match &options.frequency {
        Some(path) => read_frequencies(path)?,
        None => HashMap::new(),
    };
    let mut groups: Vec<Group> = Vec::new();
    let mut index_by_code: HashMap<String, usize> = HashMap::new();
    let mut seen: HashSet<String> = HashSet::new();
    let mut order = 0usize;
    for path in &options.inputs {
        let (mut added, mut unknown) = (0usize, 0usize);
        for spelling in read_source(path)? {
            // 给了词频表时，wordfreq 不认识的撇号词不收（见模块文档）；第一遍没有词频表，全收好让脚本算词频
            if options.frequency.is_some()
                && spelling.has_apostrophe()
                && frequencies
                    .get(&spelling.lowercase())
                    .is_none_or(|&count| count == 0)
            {
                unknown += 1;
                continue;
            }
            if !seen.insert(spelling.word.clone()) {
                continue;
            }
            order += 1;
            group_of(&mut groups, &mut index_by_code, &spelling.word)
                .push(Spelling { order, ..spelling });
            added += 1;
        }
        tracing::info!(
            path = %path.display(),
            entries = added,
            unknown_apostrophe_words = unknown,
            "已读取"
        );
    }
    for group in &mut groups {
        group.spellings.sort_by(Spelling::precedence);
    }
    // 给了语料大小写次数：只差大小写的写法按次数重排（见 `case` 模块）；展示写法在后面替换，仍然优先
    if let Some(path) = &options.case_counts {
        let counts = read_case_counts(path)?;
        let mut stats = CaseStats::default();
        for group in &mut groups {
            reorder(
                &mut group.spellings,
                &counts,
                options.case_min_total,
                &mut stats,
            );
        }
        tracing::info!(path = %path.display(), ?stats, "已按语料大小写重排");
    }
    for path in &options.display_forms {
        let mut replaced = 0usize;
        for spelling in read_source(path)? {
            order += 1;
            let lowercase = spelling.lowercase();
            let spellings = group_of(&mut groups, &mut index_by_code, &spelling.word);
            spellings.retain(|s| s.lowercase() != lowercase);
            spellings.insert(0, Spelling { order, ..spelling });
            replaced += 1;
        }
        tracing::info!(path = %path.display(), entries = replaced, "展示写法已替换");
    }
    write(
        &groups,
        &frequencies,
        options.frequency_floor,
        &options.output,
    )
}

/// 词所在编码的那一组，没有就新建（追加在最后，输出顺序 = 编码第一次出现的顺序）。
fn group_of<'a>(
    groups: &'a mut Vec<Group>,
    index_by_code: &mut HashMap<String, usize>,
    word: &str,
) -> &'a mut Vec<Spelling> {
    let code = english_code(word);
    let index = *index_by_code.entry(code.clone()).or_insert_with(|| {
        groups.push(Group {
            code,
            spellings: Vec::new(),
        });
        groups.len() - 1
    });
    &mut groups[index].spellings
}

/// 读一个源表：每行 `词\t编码[\t…]`，表头行给出 `frequency_level` 的列号；没有表头的文件等级全为 `None`。
/// 只交出 [`is_accepted`] 的写法，`order` 由调用方填。
fn read_source(path: &Path) -> Result<Vec<Spelling>, ConvertError> {
    let mut level_column: Option<usize> = None;
    let mut spellings = Vec::new();
    let mut first = true;
    for raw in std::fs::read_to_string(path)?.lines() {
        let line = raw.trim_end();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let fields: Vec<&str> = line.split('\t').map(str::trim).collect();
        // 至少要有 `词\t编码` 两列：YAML 头这类没有制表符的行自然跳过
        if fields.len() < 2 {
            continue;
        }
        // 表头只认第一行正文、且前两列是 `word\tkey`：总表里有个正常词 `word`（`word\tword\t35`），不能当表头
        let header = first && fields[0] == "word" && fields[1] == "key";
        first = false;
        if header {
            level_column = fields.iter().position(|f| *f == "frequency_level");
            continue;
        }
        if !is_accepted(fields[0]) {
            continue;
        }
        let level = level_column
            .and_then(|column| fields.get(column))
            .and_then(|value| value.parse().ok());
        spellings.push(Spelling {
            word: fields[0].to_owned(),
            level,
            order: 0,
            case_count: None,
        });
    }
    Ok(spellings)
}

/// 读频率表 `小写词\t词频`（`tools/corpus/english_frequency.py` 的输出；纯字母词的小写词就是编码，旧文件照用）。
fn read_frequencies(path: &Path) -> Result<HashMap<String, u32>, ConvertError> {
    Ok(std::fs::read_to_string(path)?
        .lines()
        .filter(|l| !l.is_empty() && !l.starts_with('#'))
        .filter_map(|l| {
            let (word, count) = l.split_once('\t')?;
            Some((word.to_ascii_lowercase(), count.trim().parse().ok()?))
        })
        .collect())
}

/// 按模块文档的词频规则配词频，写出 `词\t编码\t词频`，并打出统计。
fn write(
    groups: &[Group],
    frequencies: &HashMap<String, u32>,
    floor: u32,
    output: &Path,
) -> Result<(), ConvertError> {
    let mut floored = 0usize;
    // 查自己的词频，没有或为 0 就给底值并计数
    let mut own = |spelling: &Spelling| match frequencies.get(&spelling.lowercase()) {
        Some(&count) if count > 0 => count,
        _ => {
            floored += 1;
            floor
        }
    };
    let mut file = BufWriter::new(File::create(output)?);
    writeln!(
        file,
        "# 由 glimmer-dict-convert english 生成。词\\t编码\\t词频（wordfreq 的 Zipf 频率 ×1000，没有的给底值 {floor}）；同码第一个是缺省写法"
    )?;
    let (mut entries, mut codes, mut shared, mut apostrophes) = (0usize, 0usize, 0usize, 0usize);
    for group in groups {
        let Some(default) = group.spellings.first() else {
            continue;
        };
        let default_lowercase = default.lowercase();
        let default_count = own(default);
        let mut values: Vec<u32> = Vec::with_capacity(group.spellings.len());
        for (index, spelling) in group.spellings.iter().enumerate() {
            // 同组（只差大小写）里排在它前面的第一个写法：按语料重排过的组，词频跟着它与语料次数走
            let head = group.spellings[..index]
                .iter()
                .position(|s| s.lowercase() == spelling.lowercase());
            let count = match (head, spelling.case_count) {
                _ if index == 0 => default_count,
                (Some(head), Some(count)) if group.spellings[head].case_count.is_some() => {
                    let head_count = group.spellings[head].case_count.unwrap_or(0);
                    variant_frequency(values[head], head_count, count)
                }
                _ if spelling.lowercase() == default_lowercase => {
                    default_count.saturating_sub(CASE_VARIANT_DISCOUNT)
                }
                _ => own(spelling).min(default_count),
            };
            values.push(count);
            writeln!(file, "{}\t{}\t{count}", spelling.word, group.code)?;
            entries += 1;
            apostrophes += usize::from(spelling.has_apostrophe());
        }
        codes += 1;
        shared += usize::from(group.spellings.len() > 1);
    }
    file.flush()?;
    tracing::info!(
        path = %output.display(),
        entries,
        codes,
        shared_codes = shared,
        apostrophe_words = apostrophes,
        floored,
        "写出完成"
    );
    Ok(())
}

#[cfg(test)]
mod tests;
