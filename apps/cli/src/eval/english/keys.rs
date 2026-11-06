//! 计键规则（写死，改了就要重量基线）：每个字母（含撇号）1 键，选中候选 1 键，词后的空格与标点两边都不计。
//! 8 个字母的词敲 2 个字母后选中 = 3 键。

/// 一个词花了几键，两种口径各一个数。
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Keys {
    /// 文章口径：敲完整个词就算上屏了，不管要不要多按回车。
    pub article: usize,

    /// 实际按键：敲完整个词、首选又不是它时，还要按一下回车原样上屏，这一键算进去。
    pub actual: usize,
}

impl Keys {
    /// 敲了 `typed` 个字母后选中：两种口径一样。
    pub fn selected(typed: usize) -> Self {
        Self {
            article: typed + 1,
            actual: typed + 1,
        }
    }

    /// 整个词敲完上屏：`letters` 键，`extra` 为真时实际按键多 1 键（回车）。
    pub fn typed_out(letters: usize, extra: bool) -> Self {
        Self {
            article: letters,
            actual: letters + usize::from(extra),
        }
    }

    /// 补全：`ranks` 是逐字母敲目标词时它的名次（见 `Trace::ranks`），名次小于 `window` 才算看得到。
    /// 看得到、而且选它比敲完省键（敲了的字母数 + 1 < 词长）就选；一直不划算就敲完上屏。
    pub fn completion(ranks: &[Option<usize>], window: usize, extra: bool) -> Self {
        let letters = ranks.len();
        for (index, rank) in ranks.iter().enumerate() {
            let typed = index + 1;
            if typed + 1 < letters && rank.is_some_and(|rank| rank < window) {
                return Self::selected(typed);
            }
        }
        Self::typed_out(letters, extra)
    }

    /// 敲错没救回来：已敲 `typed` 键，退格退到公共前缀 `common` 处（每下 1 键），再把剩下的 `letters - common` 个字母敲完上屏。
    pub fn recovered(typed: usize, common: usize, letters: usize, extra: bool) -> Self {
        let fixed = typed + (typed - common) + (letters - common);
        Self::typed_out(fixed, extra)
    }
}
