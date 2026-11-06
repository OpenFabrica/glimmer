//! 下一词预测组（`predict`）：和补全组完全一样地逐词敲，只多一步——开敲每个词之前先看 Engine 给不给预测。
//!
//! 规则写死，改了就要重量基线：
//! - 单独一遍：从空学习数据、不学习、没有上文开始（与别的组一样 `reset`），英文下一词预测打开。
//!   词两边的标点与空格照补全组经 `note_passthrough` 告知，所以 Core 看到的「上一个词之后敲了空格」与真实一样。
//! - 每个词开敲之前（词前标点直通之后）问一次 `Engine::english_next_words`，走真实的出预测条件（刚敲过空格、上文记着词、
//!   第一名概率不低于 `en-predict-min`……）。
//! - 只有第一个预测能被 Tab 接受。它与目标词逐字相同（判等同补全组「目标词在候选里」：`text == 原文`，大小写也要一样）
//!   就按 Tab：调 `Engine::accept_english_prediction(0)`（上屏「词 + 空格」，上文与学习走真实路径），计 1 键，这个词完成；
//!   Tab 自带的空格当作词后的空格，词后原文照常直通（词后紧跟标点时真实里要退格补标点，这里不计，只单独报次数）。
//! - 不是目标词或没出：照补全组敲（`Trace::record` + 同一个上屏函数 + 同一套计键），不额外计键（用户直接接着敲字母）。
//! - 首选规则、前三规则各算一行，两行都带预测这一步；预测是否出现、是否命中只取决于上文，与这一词按哪条规则选无关，
//!   而两条规则上屏的都是原词、留下的上文一样，所以一遍里同时算两行。
//! - 「如果给三个」只作参考、不进省键率：出了预测的词位临时把个数调成 3 再问一次、问完换回原参数，看目标词在不在前三。
//!   换参数只清掉预测缓存（主模拟接着问时重算一次，结果不变），不动上文、组句与学习。
//! - 耗时：每个能出预测的词位（刚敲过空格、上文记着词）第一次取预测的那一次，含扫后继与算分；同一上文再问走缓存，不计。
//! - 连续命中：连着几个词位都按 Tab 接受、中间只隔 Tab 自带的空格（前一个词后面就是一个空格、这个词前面没有标点）算一段，
//!   报长度 1 / 2 / 3 以上各几段与最长一段。
//! - 5 秒时效在模拟里不会触发：事件之间只隔一次查询的时间（毫秒级）。

mod streak;
mod tally;

pub use tally::PredictTally;

use std::time::Instant;

use glimmer_core::Engine;
use glimmer_core::english::PredictParams;

use super::typing::{WINDOW, commit_typed};
use super::{Keys, Options, Piece, Report, Trace, pass_through};
pub(super) use streak::Streak;

/// 「如果给三个」那一问取几个预测。
const REFERENCE_COUNT: usize = 3;

/// 敲一遍全文（到词数上限为止），结果累计进 `report.predict`。调用方先 `reset`。
pub fn run(engine: &mut Engine, pieces: &[Piece], options: &Options, report: &mut Report) {
    engine.set_english_prediction(true);
    let params = engine.english_predict_params();
    let tally = &mut report.predict;
    let mut streak = Streak::default();
    let mut index = 0;
    for piece in pieces {
        let (head, word, tail) = match piece {
            Piece::Gap(text) => {
                pass_through(engine, text);
                tally.end_streak(&mut streak);
                continue;
            }
            Piece::Word { head, text, tail } => (head, text.as_str(), tail),
        };
        if !options.within(index) {
            break;
        }
        pass_through(engine, head);
        let letters = word.chars().count();
        let hit = look(engine, word, params, tally);
        // 命中了就按 Tab；接受失败（理论上不会：同一上文、条件刚查过）就当没命中照敲
        let accepted = hit && engine.accept_english_prediction(0).is_some();
        let (top1, top3) = if accepted {
            tally.hits += 1;
            tally.punctuated += usize::from(followed_by_punctuation(tail));
            tally.accept_streak(&mut streak, head.is_empty(), tail == " ");
            (Keys::selected(0), Keys::selected(0))
        } else {
            tally.end_streak(&mut streak);
            let clean = Trace::record(engine, word, word);
            commit_typed(engine, &clean);
            let extra = clean.needs_extra_key();
            (
                Keys::completion(&clean.ranks, 1, extra),
                Keys::completion(&clean.ranks, WINDOW, extra),
            )
        };
        pass_through(engine, tail);
        tally.positions += 1;
        tally.top1.add(letters, top1);
        tally.top3.add(letters, top3);
        index += 1;
    }
    tally.end_streak(&mut streak);
}

/// 一个词位开敲之前看一眼预测：记能不能出、出没出、耗时与「如果给三个」；返回第一个预测是不是正好是目标词。
fn look(engine: &mut Engine, word: &str, params: PredictParams, tally: &mut PredictTally) -> bool {
    // 与 Core 出预测的前两条件相同（刚敲过空格、上文记着词）：只有这些词位真的扫了后继，耗时只记它们
    let eligible =
        engine.english_context().after_space() && engine.english_history().previous().is_some();
    let started = Instant::now();
    let words = engine.english_next_words();
    let elapsed = started.elapsed();
    if eligible {
        tally.eligible += 1;
        tally.timing.push(elapsed);
    }
    let Some(first) = words.first() else {
        return false;
    };
    tally.shown += 1;
    tally.top3_hits += usize::from(in_top_three(engine, word, params));
    first.text == word
}

/// 「如果给三个」：临时把个数调成 3 再问一次，问完换回原参数。
/// 换参数会清掉预测缓存，主模拟接着问时重算一次、结果不变（缓存只省时间），上文、组句与学习都不动。
fn in_top_three(engine: &mut Engine, word: &str, params: PredictParams) -> bool {
    engine.set_english_predict_params(PredictParams {
        count: REFERENCE_COUNT,
        ..params
    });
    let found = engine
        .english_next_words()
        .iter()
        .any(|next| next.text == word);
    engine.set_english_predict_params(params);
    found
}

/// 词后紧跟的是标点（不是空格或换行）：真实里 Tab 带出的空格要退格掉再补标点。
fn followed_by_punctuation(tail: &str) -> bool {
    tail.chars().next().is_some_and(|c| c != ' ' && c != '\n')
}
