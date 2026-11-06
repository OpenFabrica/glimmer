//! 下一词预测组的计数与它在报告里的那一段。

use std::collections::BTreeMap;
use std::fmt;

use super::Streak;
use crate::eval::english::report::{percent, savings_line, timing_line};
use crate::eval::english::savings::Savings;
use crate::eval::english::timing::Timing;

/// 下一词预测组的全部数字。
///
/// 报告里这几行的开头（`预测出现`、`第一名命中`、`含预测首选省键率`、`含预测前三省键率`、`连续命中`、`如果给三个`、`取预测耗时`）
/// 是给 `tools/eval/sweep.sh` 抽数用的，改格式要一起改那边的正则。
#[derive(Debug, Default)]
pub struct PredictTally {
    /// 评了几个词位。
    pub positions: usize,

    /// 能出预测的词位：刚敲过空格、上文记着词（出不出再看第一名概率够不够下限）。
    pub eligible: usize,

    /// 真的出了预测的词位。
    pub shown: usize,

    /// 第一个预测正好是目标词、按 Tab 接受了的词位。
    pub hits: usize,

    /// 其中词后紧跟标点的（真实里要退格掉 Tab 带出的空格再补标点，这里不计键）。
    pub punctuated: usize,

    /// 「如果给三个」：出了预测的词位里，个数调成 3 时目标词在前三的。
    pub top3_hits: usize,

    /// 含预测的首选规则击键。
    pub top1: Savings,

    /// 含预测的前三规则击键。
    pub top3: Savings,

    /// 连续命中：段长 → 段数。
    pub streaks: BTreeMap<usize, usize>,

    /// 每个能出预测的词位第一次取预测的耗时。
    pub timing: Timing,
}

impl PredictTally {
    /// 这个词位按 Tab 接受了：记进连续命中（接不上的旧段在这里落账）。
    pub(super) fn accept_streak(&mut self, streak: &mut Streak, joined: bool, open: bool) {
        if let Some(length) = streak.accept(joined, open) {
            *self.streaks.entry(length).or_default() += 1;
        }
    }

    /// 这个词位没接受或上文断开：正在数的一段落账。
    pub(super) fn end_streak(&mut self, streak: &mut Streak) {
        if let Some(length) = streak.end() {
            *self.streaks.entry(length).or_default() += 1;
        }
    }

    /// 段长至少 `min`、至多 `max` 的段数。
    fn streaks_between(&self, min: usize, max: usize) -> usize {
        self.streaks.range(min..=max).map(|(_, count)| count).sum()
    }
}

impl fmt::Display for PredictTally {
    /// 报告里的「下一词预测」一段，接在全部旧输出之后。
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(
            f,
            "\n下一词预测（每个词开敲前先看预测，第一个正好是目标词就按 Tab 接受，1 键；不是就照补全组敲，不加键）"
        )?;
        writeln!(
            f,
            "预测出现  {} / {} 词位（{}）  能出的词位 {}（刚敲过空格、上文记着词；能出没出的是第一名概率不到下限）",
            self.shown,
            self.positions,
            percent(self.shown, self.positions),
            self.eligible,
        )?;
        writeln!(
            f,
            "第一名命中  {} / {}（精确率 {}）  占全部词位 {}  其中词后紧跟标点 {} 次（真实里要退格补标点，这里不计）",
            self.hits,
            self.shown,
            percent(self.hits, self.shown),
            percent(self.hits, self.positions),
            self.punctuated,
        )?;
        writeln!(f, "含预测首选省键率  {}", savings_line(&self.top1))?;
        writeln!(f, "含预测前三省键率  {}", savings_line(&self.top3))?;
        writeln!(
            f,
            "连续命中（连按 Tab，中间只隔 Tab 带出的空格）  1 次 {}  2 次 {}  3 次以上 {}  最长 {}",
            self.streaks_between(1, 1),
            self.streaks_between(2, 2),
            self.streaks_between(3, usize::MAX),
            self.streaks.keys().next_back().copied().unwrap_or(0),
        )?;
        writeln!(
            f,
            "如果给三个（en-predict-count=3，只作参考，不进省键率）  前三有目标词 {} / {}（占出现 {}）  占全部词位 {}",
            self.top3_hits,
            self.shown,
            percent(self.top3_hits, self.shown),
            percent(self.top3_hits, self.positions),
        )?;
        writeln!(f, "取预测耗时  {}", timing_line(&self.timing))?;
        writeln!(
            f,
            "  （每个能出的词位第一次取预测，含扫后继与算分；同一上文再问走缓存，不计）"
        )
    }
}
