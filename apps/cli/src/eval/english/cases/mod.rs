//! 错前缀固定用例表（`tools/eval/english/prefix-typos.tsv`）：每条从空上文敲一遍，报期望词在不在候选里、排第几。

mod case;
mod outcome;

pub use case::PrefixCase;
pub use outcome::CaseOutcome;

use glimmer_core::Engine;

/// 逐条跑：清掉上文、逐键敲、敲完查一次。不上屏，学习由调用方关掉。
pub fn run(engine: &mut Engine, cases: &[PrefixCase]) -> Vec<CaseOutcome> {
    cases
        .iter()
        .map(|case| {
            engine.clear();
            engine.break_chain();
            for c in case.typed.chars() {
                engine.push(c);
            }
            let items = engine
                .query()
                .map(|query| query.candidates.items)
                .unwrap_or_default();
            engine.clear();
            CaseOutcome {
                case: case.clone(),
                rank: items.iter().position(|c| c.text == case.expected),
                top3: items.iter().take(3).map(|c| c.text.clone()).collect(),
            }
        })
        .collect()
}
