//! 正常词被改错率：干净文本里每个词都整个敲完、按空格（上屏第一个候选），看上屏的是不是敲的那个词。
//!
//! 单独一遍：这里上屏的可能不是原词（被补全或纠错换掉），下一个词的上文就是被换掉之后的字，不能和主评测共用一遍。
//! 分「词表里有」「词表里没有」两种报，每种再拆「只改大小写」与「换成别的词」：词表里有的词换成别的词靠精确词排第一保证为 0，
//! 只改大小写（词表只收了 `OR` 这种写法）单独看；词表里没有的词在现在的按键规则（空格选高亮）下会被换掉，要靠回车。
//! 另把「小写词被改成大写」按「敲的 → 上屏的」聚合计数，`or → OR` 这类一眼能看到。

mod change;
mod tally;

pub use change::Change;
pub use tally::SpaceTally;

use std::collections::HashSet;

use glimmer_core::Engine;

use super::{Options, Piece, Report, pass_through};

/// 敲一遍全文，结果写进 `report.space_*`。`vocabulary` 是词表里全部词的小写。
pub fn run(
    engine: &mut Engine,
    pieces: &[Piece],
    vocabulary: &HashSet<String>,
    options: &Options,
    report: &mut Report,
) {
    let mut index = 0;
    for piece in pieces {
        let (head, word, tail) = match piece {
            Piece::Gap(text) => {
                pass_through(engine, text);
                continue;
            }
            Piece::Word { head, text, tail } => (head, text, tail),
        };
        if !options.within(index) {
            break;
        }
        pass_through(engine, head);
        for c in word.chars() {
            engine.push(c);
        }
        let first = engine
            .query()
            .ok()
            .and_then(|query| query.candidates.items.into_iter().next());
        // 空格：有候选上屏第一个，没有候选原样上屏
        let committed = match first {
            Some(candidate) => engine.commit(&candidate),
            None => engine.take_raw(),
        };
        engine.clear();
        pass_through(engine, tail);
        let known = vocabulary.contains(&word.to_ascii_lowercase());
        let change = Change::of(word, &committed);
        if known {
            report.space_known.add(change);
        } else {
            report.space_unknown.add(change);
        }
        if Change::upcased(word, &committed) {
            *report
                .space_upcased
                .entry((word.clone(), committed.clone()))
                .or_default() += 1;
        }
        if change != Change::Same && report.space_misses.len() < options.show_misses {
            report.space_misses.push(format!(
                "[{}·{}] {:<16} → {}",
                if known {
                    "词表里有"
                } else {
                    "词表里没有"
                },
                if change == Change::CaseOnly {
                    "只改大小写"
                } else {
                    "换成别的词"
                },
                word,
                committed,
            ));
        }
        index += 1;
    }
}
