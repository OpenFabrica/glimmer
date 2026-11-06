//! 主评测那一遍：逐词模拟敲字，同时量补全、错字组、错前缀组与每键耗时。
//!
//! 每个词先在同一个上文下各敲一遍带错的串（错字组、错前缀组；敲完清掉、不上屏），再逐字母敲原词、真的上屏。
//! 三组里模拟用户最后上屏的都是原词，所以下一个词的上文三组一样，合在一遍里跑与分三遍跑结果相同，省了两遍查询。
//! 每个词的候选只取决于上文与敲的串，与「在哪一键选中」无关：首选组、前三组都从同一份逐键记录里按规则算击键。

use glimmer_core::Engine;

use super::{Group, Keys, Options, Piece, Report, Trace, Typo, pass_through, typo};

/// 错前缀组只评这么长以上的词：短词前三个字母里错了就几乎没剩下可补的。
const MIN_PREFIX_LETTERS: usize = 4;

/// 「12 个字母以上的错误输入」从这一键算起。
const LONG_INPUT: usize = 12;

/// 前三组看几个候选（数字键选当前页前三个）；下一词预测组的前三规则用同一个数。
pub(super) const WINDOW: usize = 3;

/// 固定的几条长错串：语料里 12 个字母以上的错误输入可能很少，最坏情况的耗时靠它们保底有样本。
const LONG_TYPOS: [&str; 10] = [
    "internationalizaton",
    "responsibilitiess",
    "acknowledgemnets",
    "misunderstandnig",
    "congratulatoins",
    "incomprehensable",
    "telecommunicatoins",
    "disproportionatley",
    "characteristcally",
    "qwertyuiopasdfghjkl",
];

/// 敲一遍全文（到词数上限为止），结果累计进 `report`。
pub fn run(engine: &mut Engine, pieces: &[Piece], options: &Options, report: &mut Report) {
    let mut index = 0;
    for piece in pieces {
        let (head, word, tail) = match piece {
            Piece::Gap(text) => {
                pass_through(engine, text);
                continue;
            }
            Piece::Word { head, text, tail } => (head, text.as_str(), tail),
        };
        if !options.within(index) {
            break;
        }
        pass_through(engine, head);
        let letters = word.chars().count();
        let typo = options
            .wants(Group::Typo)
            .then(|| typo::pick(word, options.seed, index, false))
            .flatten()
            .map(|typo| probe(engine, typo, word));
        let prefix = (options.wants(Group::Prefix) && letters >= MIN_PREFIX_LETTERS)
            .then(|| typo::pick(word, options.seed, index, true))
            .flatten()
            .map(|typo| probe(engine, typo, word));
        let clean = Trace::record(engine, word, word);
        commit_typed(engine, &clean);
        pass_through(engine, tail);
        let extra = clean.needs_extra_key();
        tally_timing(report, &clean);
        if options.wants(Group::Complete) {
            tally_completion(report, &clean, extra);
        }
        if let Some((typo, trace)) = typo {
            tally_typo(report, &typo, &trace, word, extra, options.show_misses);
        }
        if let Some((typo, trace)) = prefix {
            tally_prefix(report, &typo, &trace, word, extra, options.show_misses);
        }
        index += 1;
    }
    // 走到这里至少跑了补全、错字、错前缀中的一组，最坏情况的长错串也一起量；只记敲完整串那一键
    for typed in LONG_TYPOS {
        engine.clear();
        let trace = Trace::record(engine, typed, "");
        engine.clear();
        if let Some(last) = trace.durations.last() {
            report.long_typos.push(*last);
        }
    }
}

/// 敲一遍带错的串、记下过程，然后清掉缓冲区（不上屏，上文不动）。
fn probe(engine: &mut Engine, typo: Typo, word: &str) -> (Typo, Trace) {
    let trace = Trace::record(engine, &typo.typed, word);
    engine.clear();
    (typo, trace)
}

/// 原词敲完之后真的上屏：首选就是它按空格上屏这个候选，否则回车原样上屏敲的字母。两条路上屏的都是原词。
/// 下一词预测组没命中时也走这里，保证它与补全组上屏的字、留下的上文一样。
pub(super) fn commit_typed(engine: &mut Engine, clean: &Trace) {
    match (&clean.top, clean.final_rank()) {
        (Some(top), Some(0)) => {
            engine.commit(top);
        }
        _ => {
            engine.take_raw();
        }
    }
    engine.clear();
}

/// 补全：两种窗口的击键、敲 1–3 个字母时的命中。
fn tally_completion(report: &mut Report, clean: &Trace, extra: bool) {
    let letters = clean.ranks.len();
    report
        .top1
        .add(letters, Keys::completion(&clean.ranks, 1, extra));
    report
        .top3
        .add(letters, Keys::completion(&clean.ranks, WINDOW, extra));
    for (typed, hit) in report.hits.iter_mut().enumerate() {
        if letters <= typed + 1 {
            continue;
        }
        let rank = clean.ranks[typed];
        hit[0] += 1;
        hit[1] += usize::from(rank == Some(0));
        hit[2] += usize::from(rank.is_some_and(|r| r < WINDOW));
    }
}

/// 原词那一遍每键的耗时；第一键单独再记一份（只敲 1 个字母是最坏情况之一：前缀区间最大）。
fn tally_timing(report: &mut Report, clean: &Trace) {
    for duration in &clean.durations {
        report.keystrokes.push(*duration);
    }
    if let Some(first) = clean.durations.first() {
        report.one_letter.push(*first);
    }
}

/// 错字组：带错的词整个敲完再看候选，前三有原词就选（1 键），没有就退格到错处重敲。
fn tally_typo(
    report: &mut Report,
    typo: &Typo,
    trace: &Trace,
    word: &str,
    extra: bool,
    show_misses: usize,
) {
    let letters = word.chars().count();
    let typed = trace.ranks.len();
    let rank = trace.final_rank();
    let found = rank.is_some_and(|r| r < WINDOW);
    let kind = &mut report.typo_kinds[typo.kind.index()];
    kind[0] += 1;
    kind[1] += usize::from(rank == Some(0));
    kind[2] += usize::from(found);
    let keys = if found {
        Keys::selected(typed)
    } else {
        Keys::recovered(typed, typo.common, letters, extra)
    };
    report.typo.add(letters, keys);
    // 已经敲错了（过了公共前缀）、又有 12 个字母以上的键，算进最坏情况
    for (index, duration) in trace.durations.iter().enumerate() {
        if index + 1 >= LONG_INPUT && index >= typo.common {
            report.long_typos.push(*duration);
        }
    }
    if !found && report.typo_misses.len() < show_misses {
        report.typo_misses.push(format!(
            "[{}] {:<16} → {:<14} 现在前三 {}",
            typo.kind.label(),
            typo.typed,
            word,
            trace.top3.join(" / "),
        ));
    }
}

/// 错前缀组：错处之后每一键看前三，原词一出现就选；一直没出现就和错字组一样退格重敲。
fn tally_prefix(
    report: &mut Report,
    typo: &Typo,
    trace: &Trace,
    word: &str,
    extra: bool,
    show_misses: usize,
) {
    let letters = word.chars().count();
    let typed = trace.ranks.len();
    // 错处之前出现不算：那时还没敲错，量的是补全不是纠错
    let seen = trace
        .ranks
        .iter()
        .enumerate()
        .skip(typo.common)
        .find(|(_, rank)| rank.is_some_and(|r| r < WINDOW))
        .map(|(index, _)| index + 1);
    let keys = match seen {
        Some(at) => {
            *report.prefix_seen.entry(at).or_default() += 1;
            Keys::selected(at)
        }
        None => {
            report.prefix_never += 1;
            if report.prefix_misses.len() < show_misses {
                report.prefix_misses.push(format!(
                    "[{}] {:<16} → {:<14} 敲完时前三 {}",
                    typo.kind.label(),
                    typo.typed,
                    word,
                    trace.top3.join(" / "),
                ));
            }
            Keys::recovered(typed, typo.common, letters, extra)
        }
    };
    report.prefix.add(letters, keys);
}
