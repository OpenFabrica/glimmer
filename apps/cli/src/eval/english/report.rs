//! 英文打字模拟评测的报告与它的文本格式。
//!
//! 每组第一行的开头（`首选省键率`、`错字组`、`错前缀组`、`正常词被改错`、`每键耗时`、`个人词回归`）是给 `tools/eval/sweep.sh`
//! 抽数用的，改格式要一起改那边的正则；下一词预测组的几行开头见 [`PredictTally`] 的文档，同样被抽。
//! 下一词预测组接在全部旧输出之后，旧的各组一个字都不受它影响。

use std::collections::BTreeMap;
use std::fmt;
use std::time::Duration;

use super::{
    CaseOutcome, Group, PersonalOutcome, PredictTally, Savings, SpaceTally, Timing, TypoKind,
};

/// 一次英文评测的全部数字。
#[derive(Debug, Default)]
pub struct Report {
    /// 标题后缀：种子、词数上限。
    pub mode: String,

    /// 跑了哪几组。
    pub groups: Vec<Group>,

    /// 评了几个词。
    pub words: usize,

    /// 跳过了几段不评的原文（数字、带连字符的词……）。
    pub gaps: usize,

    /// 补全·首选组。
    pub top1: Savings,

    /// 补全·前三组。
    pub top3: Savings,

    /// 敲 1 / 2 / 3 个字母时：`[比这长的词数, 首选是它, 前三有它]`。
    pub hits: [[usize; 3]; 3],

    /// 错字组的击键。
    pub typo: Savings,

    /// 错字组按错法分（下标见 [`TypoKind::index`]）：`[词数, 首选是正确词, 前三有正确词]`。
    pub typo_kinds: [[usize; 3]; 4],

    /// 错前缀组的击键。
    pub prefix: Savings,

    /// 错前缀组：目标词第一次进前三时已敲的字母数 → 词数。
    pub prefix_seen: BTreeMap<usize, usize>,

    /// 错前缀组：敲完整个带错的词都没进前三的词数。
    pub prefix_never: usize,

    /// 错前缀固定用例。
    pub cases: Vec<CaseOutcome>,

    /// 正常词被改错·词表里有的词。
    pub space_known: SpaceTally,

    /// 正常词被改错·词表里没有的词。
    pub space_unknown: SpaceTally,

    /// 小写词被改成大写：(敲的, 上屏的) → 次数。
    pub space_upcased: BTreeMap<(String, String), usize>,

    /// 举例与清单最多列几条（`--misses`）。
    pub show_misses: usize,

    /// 原词那一遍每键（推键 + 查询）的耗时（补全、错字、错前缀任一组跑了就有）。
    pub keystrokes: Timing,

    /// 其中只敲了 1 个字母的那些键。
    pub one_letter: Timing,

    /// 12 个字母以上、已经敲错了的输入（错字组里错处之后的键 + 固定的几条长错串）。
    pub long_typos: Timing,

    /// 个人词回归用例。
    pub personal: Vec<PersonalOutcome>,

    /// 错字组没纠回来的例子。
    pub typo_misses: Vec<String>,

    /// 错前缀组目标词一直没出现的例子。
    pub prefix_misses: Vec<String>,

    /// 按空格被改掉的例子。
    pub space_misses: Vec<String>,

    /// 下一词预测组。
    pub predict: PredictTally,
}

impl Report {
    /// 这一组跑了没有。
    fn ran(&self, group: Group) -> bool {
        self.groups.contains(&group)
    }
}

/// `part / whole` 写成百分数；分母为 0 写 `-`。
pub(super) fn percent(part: usize, whole: usize) -> String {
    if whole == 0 {
        "-".to_owned()
    } else {
        format!("{:.1}%", part as f64 * 100.0 / whole as f64)
    }
}

/// 省键率写成百分数，可以为负。
pub(super) fn rate(value: Option<f64>) -> String {
    value.map_or_else(|| "-".to_owned(), |v| format!("{:.1}%", v * 100.0))
}

/// 毫秒，两位小数。
pub(super) fn ms(value: Option<Duration>) -> String {
    value.map_or_else(
        || "-".to_owned(),
        |d| format!("{:.2} ms", d.as_secs_f64() * 1000.0),
    )
}

/// 一行耗时：次数、p50、p95、最大。
pub(super) fn timing_line(timing: &Timing) -> String {
    format!(
        "{} 次  p50 {}  p95 {}  最大 {}",
        timing.count(),
        ms(timing.percentile(50.0)),
        ms(timing.percentile(95.0)),
        ms(timing.max()),
    )
}

/// 名次写成「第 N 位」，不在候选里写明。
fn place(rank: Option<usize>) -> String {
    rank.map_or_else(|| "不在候选里".to_owned(), |i| format!("第 {} 位", i + 1))
}

/// 正常词被改错的一类：`被改 / 总数（百分比）只改大小写 N 换成别的词 M`。
/// 前半截的格式 `tools/eval/sweep.sh` 在抽，后面的拆分是追加的。
fn space_part(tally: &SpaceTally) -> String {
    format!(
        "{} / {}（{}）只改大小写 {} 换成别的词 {}",
        tally.changed(),
        tally.total,
        percent(tally.changed(), tally.total),
        tally.case_only,
        tally.replaced,
    )
}

/// 一组击键：两种口径的省键率与击键数。
pub(super) fn savings_line(savings: &Savings) -> String {
    format!(
        "文章口径 {}  实际按键 {}  （击键 {} / {}，字母 {}）",
        rate(savings.article_rate()),
        rate(savings.actual_rate()),
        savings.article,
        savings.actual,
        savings.letters,
    )
}

impl fmt::Display for Report {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(
            f,
            "英文打字模拟评测（冷启动，不学习，不写文件）{}",
            self.mode
        )?;
        writeln!(f, "词 {} 个，跳过 {} 段非词原文", self.words, self.gaps)?;
        if self.ran(Group::Complete) {
            writeln!(
                f,
                "\n补全（逐字母敲，目标词进窗口、选它又省键才选；选中 1 键，空格标点不计）"
            )?;
            writeln!(f, "首选省键率  {}", savings_line(&self.top1))?;
            writeln!(f, "前三省键率  {}", savings_line(&self.top3))?;
            writeln!(f, "  命中率（分母是比这更长的词）")?;
            for (index, [total, top1, top3]) in self.hits.iter().enumerate() {
                writeln!(
                    f,
                    "    敲 {} 个字母  {:>6} 词  首选 {:>6}  前三 {:>6}",
                    index + 1,
                    total,
                    percent(*top1, *total),
                    percent(*top3, *total),
                )?;
            }
        }
        if self.ran(Group::Typo) {
            let [total, top1, top3] = self.typo_kinds.iter().fold([0; 3], |sum, kind| {
                [sum[0] + kind[0], sum[1] + kind[1], sum[2] + kind[2]]
            });
            writeln!(
                f,
                "\n错字组  {} 词  前三有正确词 {}  首选 {}  省键率 {}",
                total,
                percent(top3, total),
                percent(top1, total),
                savings_line(&self.typo),
            )?;
            writeln!(
                f,
                "  （每词注一处错，整个敲完不选词，敲完看候选；前三没有就退格到错处重敲）"
            )?;
            for kind in TypoKind::ALL {
                let [total, top1, top3] = self.typo_kinds[kind.index()];
                writeln!(
                    f,
                    "    {:<6} {:>6} 词  前三 {:>6}  首选 {:>6}",
                    kind.label(),
                    total,
                    percent(top3, total),
                    percent(top1, total),
                )?;
            }
        }
        if self.ran(Group::Prefix) {
            writeln!(
                f,
                "\n错前缀组  {} 词  省键率 {}  目标词没出现 {} 个",
                self.prefix.words,
                savings_line(&self.prefix),
                self.prefix_never,
            )?;
            writeln!(
                f,
                "  （4 个字母以上的词，前三个字母里注一处错接着敲，错处之后目标词一进前三就选）"
            )?;
            write!(f, "  目标词第一次进前三时已敲：")?;
            for (letters, count) in &self.prefix_seen {
                write!(f, " {letters} 个字母 {count}；")?;
            }
            writeln!(f, " 没出现 {}", self.prefix_never)?;
        }
        if self.ran(Group::Cases) {
            let found = self.cases.iter().filter(|c| c.rank.is_some()).count();
            let top3 = self
                .cases
                .iter()
                .filter(|c| c.rank.is_some_and(|r| r < 3))
                .count();
            writeln!(
                f,
                "\n错前缀固定用例  {} 条  在候选里 {}  前三 {}",
                self.cases.len(),
                found,
                top3
            )?;
            for outcome in &self.cases {
                writeln!(
                    f,
                    "  {:<12} → {:<14} {:<10} 现在前三 {}",
                    outcome.case.typed,
                    outcome.case.expected,
                    place(outcome.rank),
                    outcome.top3.join(" / "),
                )?;
            }
        }
        if self.ran(Group::Space) {
            writeln!(
                f,
                "\n正常词被改错  词表里有 {}  词表里没有 {}",
                space_part(&self.space_known),
                space_part(&self.space_unknown),
            )?;
            writeln!(f, "  （每个词整个敲完按空格，空格上屏第一个候选）")?;
            let total: usize = self.space_upcased.values().sum();
            write!(
                f,
                "  小写词被改成大写 {} 次（{} 种）",
                total,
                self.space_upcased.len()
            )?;
            // 次数多的在前，同次数按敲的词排，保证两次跑列出来一样
            let mut upcased: Vec<(&(String, String), &usize)> = self.space_upcased.iter().collect();
            upcased.sort_by(|a, b| b.1.cmp(a.1).then_with(|| a.0.cmp(b.0)));
            let shown: Vec<String> = upcased
                .iter()
                .take(self.show_misses)
                .map(|((typed, committed), count)| format!("{typed} → {committed} {count}"))
                .collect();
            if shown.is_empty() {
                writeln!(f)?;
            } else {
                writeln!(f, "：{}", shown.join("；"))?;
            }
        }
        if self.keystrokes.count() > 0 {
            writeln!(f, "\n每键耗时  {}", timing_line(&self.keystrokes))?;
            writeln!(
                f,
                "  只敲 1 个字母         {}",
                timing_line(&self.one_letter)
            )?;
            writeln!(
                f,
                "  12 个字母以上的错误输入 {}",
                timing_line(&self.long_typos)
            )?;
        }
        if self.ran(Group::Personal) {
            let passed = self.personal.iter().filter(|o| o.passed).count();
            writeln!(f, "\n个人词回归  过 {} / {}", passed, self.personal.len())?;
            for outcome in &self.personal {
                let case = &outcome.case;
                let ranks: Vec<String> = outcome
                    .ranks
                    .iter()
                    .map(|(prefix, own, rival)| match case.rule.rival() {
                        Some(word) => {
                            format!(
                                "{prefix}：{} {}，{word} {}",
                                case.word,
                                place(*own),
                                place(*rival)
                            )
                        }
                        None => format!("{prefix}：{} {}", case.word, place(*own)),
                    })
                    .collect();
                writeln!(
                    f,
                    "  {}  {}  （背景记录 {} 条，选 {} 次其中从候选里选中 {} 次）  {}",
                    if outcome.passed { "过  " } else { "不过" },
                    case.name,
                    outcome.background,
                    case.times,
                    outcome.selected,
                    ranks.join("；"),
                )?;
            }
        }
        for (title, misses) in [
            ("错字组没纠回来的例子（敲的 → 原词）", &self.typo_misses),
            (
                "错前缀组目标词没出现的例子（敲的 → 原词）",
                &self.prefix_misses,
            ),
            ("按空格被改掉的例子（敲的 → 上屏的）", &self.space_misses),
        ] {
            if misses.is_empty() {
                continue;
            }
            writeln!(f, "\n{title}：")?;
            for miss in misses {
                writeln!(f, "  {miss}")?;
            }
        }
        // 放在最后：只选旧分组时输出与加这一组之前逐字相同
        if self.ran(Group::Predict) {
            write!(f, "{}", self.predict)?;
        }
        Ok(())
    }
}
