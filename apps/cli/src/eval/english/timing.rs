//! 查询耗时的样本与分位数。

use std::time::Duration;

/// 一组查询耗时；报 p50 / p95 / 最大值。
#[derive(Debug, Clone, Default)]
pub struct Timing {
    samples: Vec<Duration>,
}

impl Timing {
    /// 记一次查询的耗时。
    pub fn push(&mut self, sample: Duration) {
        self.samples.push(sample);
    }

    /// 样本数。
    pub fn count(&self) -> usize {
        self.samples.len()
    }

    /// 第 `percent` 百分位（最近秩法：排好序取第 ⌈n·p⌉ 个）；没有样本为 `None`。
    pub fn percentile(&self, percent: f64) -> Option<Duration> {
        if self.samples.is_empty() {
            return None;
        }
        let mut sorted = self.samples.clone();
        sorted.sort_unstable();
        let rank = (sorted.len() as f64 * percent / 100.0).ceil() as usize;
        Some(sorted[rank.clamp(1, sorted.len()) - 1])
    }

    /// 最慢的一次；没有样本为 `None`。
    pub fn max(&self) -> Option<Duration> {
        self.samples.iter().max().copied()
    }
}
