//! 个人词回归组（`tools/eval/english/personal.tsv`）：在进程内经 Engine 的公开接口造出固定的学习状态，
//! 看个人词排得对不对。不读用户机器上的学习文件，每条用例从一个空的内存学习器开始。
//!
//! 学习状态全走真实路径：背景记录与「选个人词」都是英文模式下 `set_input` → `query` → `commit`，
//! 新词第一次没有候选时整词敲完 `take_raw`（回车原样上屏），与壳里一样把它学进个人英文词表。

mod case;
mod outcome;
mod rule;

pub use case::PersonalCase;
pub use outcome::PersonalOutcome;
pub use rule::Rule;

use glimmer_core::{CandidateKind, Engine};
use glimmer_learning::FrequencyLearner;

/// 逐条跑，返回每条的结果。跑完学习器留在最后一条的状态，调用方负责换回空的。
pub fn run(engine: &mut Engine, cases: &[PersonalCase]) -> Vec<PersonalOutcome> {
    cases.iter().map(|case| run_case(engine, case)).collect()
}

/// 一条用例：空学习器 → 背景记录 → 选个人词 → 关掉学习后逐个前缀量排序。
fn run_case(engine: &mut Engine, case: &PersonalCase) -> PersonalOutcome {
    engine.set_learner(Box::new(FrequencyLearner::default()));
    engine.set_learning(true);
    engine.clear();
    engine.break_chain();
    let background = seed_background(engine, case);
    let selected = select_word(engine, case);
    // 量的时候不再学，免得查询前缀本身改了状态
    engine.set_learning(false);
    let mut ranks = Vec::new();
    for prefix in &case.prefixes {
        engine.clear();
        engine.break_chain();
        engine.set_input(prefix);
        let items = engine
            .query()
            .map(|query| query.candidates.items)
            .unwrap_or_default();
        engine.clear();
        let own = items.iter().position(|c| c.text == case.word);
        let rival = case
            .rule
            .rival()
            .and_then(|rival| items.iter().position(|c| c.text == rival));
        ranks.push((prefix.clone(), own, rival));
    }
    let passed = !ranks.is_empty()
        && ranks
            .iter()
            .all(|(_, own, rival)| case.rule.passes(*own, *rival));
    PersonalOutcome {
        case: case.clone(),
        background,
        selected,
        ranks,
        passed,
    }
}

/// 造 `case.background` 条无关的学习记录：按两个字母的前缀 `aa`…`zz` 轮着敲，第 r 轮选每个前缀下第 r + 1 个英文候选
/// （避开 [`PersonalCase::touches`] 的词）上屏一次。词表固定时造出来的状态固定。返回实际造了几条。
fn seed_background(engine: &mut Engine, case: &PersonalCase) -> usize {
    let prefixes: Vec<String> = ('a'..='z')
        .flat_map(|first| ('a'..='z').map(move |second| format!("{first}{second}")))
        .collect();
    let mut count = 0;
    let mut round = 0;
    while count < case.background {
        let mut added = false;
        for prefix in &prefixes {
            if count >= case.background {
                break;
            }
            engine.clear();
            engine.set_input(prefix);
            let Ok(query) = engine.query() else {
                continue;
            };
            let Some(candidate) = query
                .candidates
                .items
                .into_iter()
                .filter(|c| c.kind == CandidateKind::English && !case.touches(&c.text))
                .nth(round)
            else {
                continue;
            };
            engine.commit(&candidate);
            engine.clear();
            engine.note_passthrough(' ');
            count += 1;
            added = true;
        }
        // 一整轮都没有可选的了：词表太小，造不到要的数
        if !added {
            break;
        }
        round += 1;
    }
    engine.clear();
    count
}

/// 选个人词 `case.times` 次：敲 `select_input`，候选里有它就选；没有就整词敲完回车原样上屏。返回从候选里选中的次数。
fn select_word(engine: &mut Engine, case: &PersonalCase) -> usize {
    let mut selected = 0;
    for _ in 0..case.times {
        engine.clear();
        engine.set_input(&case.select_input);
        let found = engine.query().ok().and_then(|query| {
            query
                .candidates
                .items
                .into_iter()
                .find(|c| c.text == case.word)
        });
        match found {
            Some(candidate) => {
                engine.commit(&candidate);
                selected += 1;
            }
            None => {
                engine.clear();
                engine.set_input(&case.word);
                engine.take_raw();
            }
        }
        engine.clear();
        engine.note_passthrough(' ');
    }
    selected
}
