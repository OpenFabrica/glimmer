//! 英文打字模拟评测（`--eval-english`）：模拟一个人在英文模式下逐字母敲一段英文，每敲一键向 `Engine` 要一次候选，
//! 量补全省了多少键、敲错了救不救得回来、正常词会不会被空格改掉、每键多慢，再跑错前缀固定用例与个人词回归用例，
//! 最后单独一遍量下一词预测（按 Tab 接受）能再省多少键（规则见 [`predict`]）。
//!
//! 规则写死，改了就要重量基线：
//! - 计键：每个字母（含撇号）1 键，选中候选 1 键，词后的空格与标点两边都不计（见 [`Keys`]）。
//! - 模拟用户每敲一个字母看一次候选；目标词在窗口里（首选组看第 1 个，前三组看前 3 个）、而且选它比接着敲省键才选。
//!   敲到最后一个字母才出现或一直不出现，就敲完上屏：「文章口径」不再加键；「实际按键」在首选不是它、又有候选时
//!   多算一下回车（空格会上屏别的词），所以可以为负。
//! - 只算候选列表里真的有的候选（窗口内的按空格或数字键就能选到）。
//! - 下一个词的上文是这一遍里真的经 Engine 上屏的字（`commit` / `take_raw`），词两边的标点与空格经 `note_passthrough` 告知，
//!   不拿原文充数。引擎按英文上文给候选排序，所以下一个词的上文必须是模拟里真的上屏的字：上一个词被换掉了，
//!   下一个词看到的就是换掉后的那个。
//! - 主评测从空的学习数据开始、评测中不学习（学习器换成空的内存学习器、再关掉学习），不写任何文件；
//!   个人词回归组每条用例单独造学习状态，跑完换回空的。
//! - 注错按固定种子（`--eval-english-seed`），同样的文本与种子两次跑的数字一样（耗时除外）。

mod cases;
mod error;
mod group;
mod keys;
mod options;
mod personal;
mod piece;
mod predict;
mod report;
mod savings;
mod space;
mod timing;
mod token;
mod trace;
mod typing;
mod typo;

#[cfg(test)]
mod tests;

use std::collections::HashSet;
use std::path::Path;

use glimmer_core::Engine;
use glimmer_dictionary::WordList;
use glimmer_learning::FrequencyLearner;

pub use error::EnglishEvalError;
pub use group::Group;
pub use options::Options;
pub use report::Report;

use cases::{CaseOutcome, PrefixCase};
use keys::Keys;
use personal::{PersonalCase, PersonalOutcome};
use piece::Piece;
use predict::PredictTally;
use savings::Savings;
use space::SpaceTally;
use timing::Timing;
use trace::Trace;
use typo::{Typo, TypoKind};

/// 跑一遍英文评测。`words` 是引擎里装的那份英文词表（只用来判断「词表里有没有这个词」），没有就报错。
/// 进来时引擎的学习器会被换成空的内存学习器、英文模式打开；`--user-dict` 给的学习文件不读也不写。
pub fn run(
    engine: &mut Engine,
    words: Option<&WordList>,
    options: &Options,
) -> Result<Report, EnglishEvalError> {
    let words = words.ok_or(EnglishEvalError::NoWordList)?;
    let mut text = String::new();
    for path in &options.paths {
        text.push_str(&read(path)?);
        text.push('\n');
    }
    // 用例表先读，格式错了在跑大半天之前就报
    let prefix_cases = if options.wants(Group::Cases) {
        PrefixCase::parse(&read(&options.prefix_cases)?, &options.prefix_cases)?
    } else {
        Vec::new()
    };
    let personal_cases = if options.wants(Group::Personal) {
        PersonalCase::parse(&read(&options.personal_cases)?, &options.personal_cases)?
    } else {
        Vec::new()
    };
    let pieces = token::split(&text);
    let mut report = Report {
        mode: header(options),
        show_misses: options.show_misses,
        groups: Group::ALL
            .into_iter()
            .filter(|&g| options.wants(g))
            .collect(),
        ..Report::default()
    };
    count_pieces(&pieces, options, &mut report);
    if options.wants(Group::Complete) || options.wants(Group::Typo) || options.wants(Group::Prefix)
    {
        reset(engine);
        typing::run(engine, &pieces, options, &mut report);
    }
    if options.wants(Group::Space) {
        reset(engine);
        let vocabulary: HashSet<String> = words
            .entries()
            .map(|(_, word, _)| word.to_ascii_lowercase())
            .collect();
        space::run(engine, &pieces, &vocabulary, options, &mut report);
    }
    if options.wants(Group::Cases) {
        reset(engine);
        report.cases = cases::run(engine, &prefix_cases);
    }
    if options.wants(Group::Personal) {
        reset(engine);
        report.personal = personal::run(engine, &personal_cases);
        reset(engine);
    }
    // 放在最后、单独一遍：前面各组的跑法与加这一组之前完全一样
    if options.wants(Group::Predict) {
        reset(engine);
        predict::run(engine, &pieces, options, &mut report);
        reset(engine);
    }
    Ok(report)
}

/// 回到「空学习数据、不学习、英文模式、没有上文（中文上屏链与英文上文都清掉）」的起点。
fn reset(engine: &mut Engine) {
    engine.set_learner(Box::new(FrequencyLearner::default()));
    engine.set_learning(false);
    engine.set_english_mode(true);
    engine.clear();
    engine.break_chain();
    // break_chain 已经连带作废英文上文，这里再明说一次：每组评测都从「不知道上文」开始
    engine.break_english_context();
    engine.history_mut().clear();
}

/// 不进缓冲区、原样交给应用的字符（标点、空格、跳过的原文）逐个告知引擎，与壳里直通时一样。
fn pass_through(engine: &mut Engine, text: &str) {
    for c in text.chars() {
        engine.note_passthrough(c);
    }
}

/// 数一数词数上限以内评了几个词、跳过几段原文。
fn count_pieces(pieces: &[Piece], options: &Options, report: &mut Report) {
    for piece in pieces {
        match piece {
            Piece::Word { .. } if !options.within(report.words) => break,
            Piece::Word { .. } => report.words += 1,
            Piece::Gap(_) => report.gaps += 1,
        }
    }
}

/// 报告标题后缀：种子与词数上限。
fn header(options: &Options) -> String {
    let mut mode = format!("，种子 {}", options.seed);
    if let Some(limit) = options.words {
        mode.push_str(&format!("，前 {limit} 词"));
    }
    mode
}

/// 读一个文本文件，出错时带上路径。
fn read(path: &Path) -> Result<String, EnglishEvalError> {
    std::fs::read_to_string(path).map_err(|source| EnglishEvalError::Read {
        path: path.to_owned(),
        source,
    })
}
