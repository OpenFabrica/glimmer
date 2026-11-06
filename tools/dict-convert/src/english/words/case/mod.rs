//! 用语料里的原样写法次数（`english-case` 的输出）定同码「只差大小写」的写法谁在前，并给这些写法配词频。
//!
//! - 只重排转小写相同的那一组（`Google` / `google`）；不带撇号的在撇号词前面这条不变，组占的位置也不变，只在组内换先后。
//! - 组内各写法的次数合计不到阈值（`--case-min-total`，缺省 20）不采信，沿用 [`super::Spelling::precedence`] 排出的先后。
//! - 采信时按次数降序，同次数保持原先后；语料里一次都没出现的写法照样保留，排在后面。
//! - 展示写法在这之后才替换，优先级最高（`I`、`TypeScript` 不受语料影响）。
//!
//! 词频：组内第一个写法照原规则；其余写法 = 第一个的词频 + 1000 × log10(它的次数 ÷ 第一个的次数)，
//! 夹在 [第一个 − 3000, 第一个 − 300]，且不低于 1（[`variant_frequency`]）。没采信的组照旧是第一个 − 1000。

mod stats;

use std::collections::HashMap;
use std::path::Path;

use crate::error::ConvertError;

use super::Spelling;
pub(super) use stats::CaseStats;

/// 组内其它写法比第一个最多少多少词频（Zipf 3）。
const MAX_GAP: f64 = 3000.0;

/// 组内其它写法比第一个至少少多少词频（Zipf 0.3），免得两个写法几乎同分、先后看排序是否稳定。
const MIN_GAP: f64 = 300.0;

/// 读 `写法\t次数`（`english-case` 的输出），`#` 为注释。
pub(super) fn read_case_counts(path: &Path) -> Result<HashMap<String, u64>, ConvertError> {
    Ok(std::fs::read_to_string(path)?
        .lines()
        .filter(|l| !l.is_empty() && !l.starts_with('#'))
        .filter_map(|l| {
            let (word, count) = l.split_once('\t')?;
            Some((word.to_owned(), count.trim().parse().ok()?))
        })
        .collect())
}

/// 一个编码已按原规则排好的写法里，逐个「只差大小写」的组按语料次数重排（规则见模块文档）。
pub(super) fn reorder(
    spellings: &mut [Spelling],
    counts: &HashMap<String, u64>,
    min_total: u64,
    stats: &mut CaseStats,
) {
    // 转小写 → 组员在 `spellings` 里的下标（按现在的先后）
    let mut members: Vec<(String, Vec<usize>)> = Vec::new();
    for (index, spelling) in spellings.iter().enumerate() {
        let lowercase = spelling.lowercase();
        match members.iter_mut().find(|(key, _)| *key == lowercase) {
            Some((_, slots)) => slots.push(index),
            None => members.push((lowercase, vec![index])),
        }
    }
    for (_, slots) in members.iter().filter(|(_, slots)| slots.len() > 1) {
        stats.groups += 1;
        let found: Vec<u64> = slots
            .iter()
            .map(|&i| counts.get(&spellings[i].word).copied().unwrap_or(0))
            .collect();
        if found.iter().sum::<u64>() < min_total {
            stats.too_few += 1;
            continue;
        }
        stats.adopted += 1;
        // 按次数降序，同次数保持原先后（`slots` 本身就是原先后）
        let mut ranked: Vec<(u64, Spelling)> = slots
            .iter()
            .zip(&found)
            .map(|(&i, &count)| (count, spellings[i].clone()))
            .collect();
        ranked.sort_by_key(|(count, _)| std::cmp::Reverse(*count));
        if ranked[0].1.word != spellings[slots[0]].word {
            stats.reordered += 1;
            stats.default_changed += usize::from(slots[0] == 0);
        }
        for (&slot, (count, spelling)) in slots.iter().zip(ranked) {
            spellings[slot] = Spelling {
                case_count: Some(count),
                ..spelling
            };
        }
    }
}

/// 组内其它写法的词频：`head` 是组内第一个写法的词频，`head_count` / `count` 是两者的语料次数。
pub(super) fn variant_frequency(head: u32, head_count: u64, count: u64) -> u32 {
    let head = f64::from(head);
    let value = if count == 0 {
        f64::NEG_INFINITY
    } else {
        head + 1000.0 * (count as f64 / head_count.max(1) as f64).log10()
    };
    // 先夹到 [head − 3000, head − 300]，再保证不低于 1（head 本身很小时下界可能是负数）
    value.clamp(head - MAX_GAP, head - MIN_GAP).max(1.0).round() as u32
}
