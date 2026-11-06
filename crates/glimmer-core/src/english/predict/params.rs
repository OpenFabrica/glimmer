//! 英文下一词预测的常数，缺省值见 [`PredictParams::DEFAULT`]；`glimmer-cli --tune en-predict-…` 扫参时换掉。

/// 出不出预测、出几个。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PredictParams {
    /// 第一名的概率下限：低于它整个不出（拿不准的预测不如不给，候选窗一闪一闪更干扰）。
    pub min_probability: f64,

    /// 最多给几个。界面只让 Tab 接受第一个，缺省只给一个；评测看「前三命中」时调成 3。
    pub count: usize,
}

impl PredictParams {
    /// 缺省值：概率下限 0.3、只给一个。
    ///
    /// 下限 0.3 是 2026-11-07 在调参集（dev 集、种子 1，原始输出在 `data/eval/english/predict-sweep/`）上扫出来的：
    /// 0.1 时聊天文本 68.9% 的词位都弹预测、精确率只有 31.8%；0.3 时出现率 24.5%、精确率 55.2%
    /// （tech 15.1% / 35.1%，writing 15.7% / 37.8%），含预测的首选省键率仍比不带预测高 5.9 / 2.0 / 1.9 个点
    /// （聊天 / tech / writing）。
    pub const DEFAULT: Self = Self {
        min_probability: 0.3,
        count: 1,
    };
}

impl Default for PredictParams {
    /// 同 [`Self::DEFAULT`]。
    fn default() -> Self {
        Self::DEFAULT
    }
}
