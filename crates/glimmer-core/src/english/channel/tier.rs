//! 敲错变体的代价档。

/// 一个变体（或纠错候选）有多「贵」，从便宜到贵排：`Ord` 小的先。下一步排序给每档一个先验概率。
///
/// 分档的依据：手机和键盘上的真实打字错误里，绝大多数是手指落到旁边的键、两个键按反、
/// 双写字母多按或少按一下；离得远的字母互换、凭空多出一个不相干的字母少得多。
/// 所以「一处编辑」拆成两档：物理上容易犯的归 [`CostTier::Near`]，其余归 [`CostTier::Edit`]。
/// 误拼表收的是人写错的常见词形（`recieve`），比任何猜测都可靠，单独一档放最前。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum CostTier {
    /// 误拼表命中（[`crate::english::misspelling_hits`]）；变体生成不会给出这一档。
    Misspelling,

    /// 便宜的一处错误：邻键替换（`hwllo` → hello）、相邻对调（`teh` → the）、
    /// 多敲了与左右相同或相邻的键（`helllo` → hello，`hwello` → hello）、
    /// 漏敲了双写字母的一半（`helo` → hello，`tomorow` → tomorrow）。
    Near,

    /// 其他一处错误：不相邻的替换、多出或漏掉一个不相干的字母。
    Edit,

    /// 两处错误；只在长词且一处错误没结果时才试（[`super::should_try_two_edits`]）。
    TwoEdits,
}
