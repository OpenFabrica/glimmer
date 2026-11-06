//! 按语料重排大小写写法的统计，只用来打日志。

/// 「只差大小写」的写法组（同码、转小写相同、至少两个写法）的处理结果。
#[derive(Debug, Default)]
pub(in crate::english::words) struct CaseStats {
    /// 这样的组有几个。
    pub(in crate::english::words) groups: usize,

    /// 按语料定了先后的组。
    pub(in crate::english::words) adopted: usize,

    /// 其中组内第一个写法变了的。
    pub(in crate::english::words) reordered: usize,

    /// 其中整个编码的缺省写法（同码第一个）变了的。
    pub(in crate::english::words) default_changed: usize,

    /// 语料次数合计不到阈值、沿用原规则的组。
    pub(in crate::english::words) too_few: usize,
}
