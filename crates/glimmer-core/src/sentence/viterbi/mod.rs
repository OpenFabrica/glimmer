//! 词图上的最优路径：Viterbi + 束搜索。
//!
//! 状态只按前一个词分（束宽内），三元（静态模型与个人 n-gram 都看前二词）要的前二词取前驱节点的回指
//! （它那条最优路径上的前一个词）：不扩状态，代价是三元上下文是近似的。真按前二词扩状态要多一个数量级的节点，
//! 而回指给出的前二词与最优路径上的一致，只有束内次优走法会差。

mod arrival;
mod node;

use glimmer_dictionary::{Dictionary, SyllablePattern};

use arrival::Arrival;
use node::Node;

use super::{
    BEAM_WIDTH, CodeLattice, Context, Conversion, LanguageModel, Lattice, MIN_PARTIAL_LETTERS,
    Personal, Search, SentenceWord, SpanCache, SyllableLattice, fallback_log_prob,
    transition_log_prob,
};
use crate::ranking::weight_bonus;

/// 词库里没有的孤立音节（罕见音节没有单字）按这个 log 概率兜底，让路径总能走通。
const UNKNOWN_LOG_PROB: f64 = -30.0;

/// 把音节序列转成最可能的词序列。`positions` 每个位置是若干写法（第一种是敲的，其余是模糊音 / 敲错变体），
/// `cost(位置, 命中的音节)` 是那个位置命中这种写法要扣的分（敲的原样 0），
/// `weight` 是用户选择次数，`personal` 是个人 n-gram 与插值参数（没有个人数据就传 [`Personal::NONE`]），`cache` 是格子候选的缓存
/// （调用方保证它与词库、`weight`、`personal`、`cost` 一致，这些一变就清）。
///
/// 简拼位置（`w x q`）按前缀取词：每个格子的候选会多得多，由语言模型在路径上分辨。
/// 全拼句子末尾的前缀太短时不算它（多半是没打完的音节）；前面已有简拼的句子里末尾单字母就是一个音节。
pub fn convert(
    dictionaries: &[&Dictionary],
    positions: &[Vec<SyllablePattern<'_>>],
    model: &dyn LanguageModel,
    personal: Personal<'_>,
    weight: impl Fn(&str) -> u32,
    cost: impl Fn(usize, &str) -> f64,
    cache: &mut SpanCache,
) -> Option<Conversion> {
    convert_paths(
        dictionaries,
        positions,
        Search::BEST,
        model,
        personal,
        weight,
        cost,
        cache,
    )
    .into_iter()
    .next()
}

/// 得分最高的前 `search.paths` 条路径（最多束宽条，按得分降序，文本相同的只留一条）：给重打分用。
/// `search.whole_word` 为真时末尾再补整词的读法，可能与前面某条同文本（见 [`Search::whole_word`]）。
///
/// `search.keep_partial` 为真时全拼句子末尾的单字母也当一个音节读（`huo z…` → 或者）：给「整段拼音读法」与别的读法比分用，
/// 比分要两边覆盖同样多的字母。`search.initial` 是这段拼音之前的上文，第一个词的转移按它算；
/// `search.protected_extra` 是原样成词保护多扣的分（见 [`SyllableLattice`]）。
#[allow(clippy::too_many_arguments)]
pub fn convert_paths(
    dictionaries: &[&Dictionary],
    positions: &[Vec<SyllablePattern<'_>>],
    search: Search<'_>,
    model: &dyn LanguageModel,
    personal: Personal<'_>,
    weight: impl Fn(&str) -> u32,
    cost: impl Fn(usize, &str) -> f64,
    cache: &mut SpanCache,
) -> Vec<Conversion> {
    let Some((last, head)) = positions.split_last() else {
        return Vec::new();
    };
    let Some(&last) = last.first() else {
        return Vec::new();
    };
    let abbreviated_head = head.iter().any(|p| p.first().is_none_or(|t| !t.complete));
    let positions = if search.keep_partial
        || last.complete
        || abbreviated_head
        || last.text.len() >= MIN_PARTIAL_LETTERS
    {
        positions
    } else {
        head
    };
    if positions.is_empty() || search.paths == 0 {
        return Vec::new();
    }
    let mut lattice = SyllableLattice::new(
        dictionaries,
        positions,
        personal,
        &weight,
        &cost,
        cache,
        search,
    );
    search_lattice(&mut lattice, search, model, personal, &weight)
}

/// 连着打的五笔编码（`wqvbkhlg`，全是编码键）转成最可能的词序列，前 `search.paths` 条。
/// `dictionaries` 是码表与用户词；`search.keep_partial` 与 `search.protected_extra` 是拼音的事，这里不看。
/// `keys` 里有不是编码键的字符时为空。
pub fn convert_codes(
    dictionaries: &[&Dictionary],
    keys: &str,
    search: Search<'_>,
    model: &dyn LanguageModel,
    personal: Personal<'_>,
    weight: impl Fn(&str) -> u32,
    cache: &mut SpanCache,
) -> Vec<Conversion> {
    let Some(mut lattice) = CodeLattice::new(dictionaries, keys, personal, &weight, cache) else {
        return Vec::new();
    };
    search_lattice(&mut lattice, search, model, personal, &weight)
}

/// 词图上得分最高的前 `search.paths` 条路径（最多束宽条，按得分降序，文本相同的只留一条；整词读法另补，见 [`add_whole_word`]）。
/// 位置是什么由 `lattice` 定（拼音是音节、五笔是编码字母），这里只管找路。
fn search_lattice(
    lattice: &mut impl Lattice,
    search: Search<'_>,
    model: &dyn LanguageModel,
    personal: Personal<'_>,
    weight: &impl Fn(&str) -> u32,
) -> Vec<Conversion> {
    let n = lattice.positions();
    let k = search.paths;
    if n == 0 || k == 0 {
        return Vec::new();
    }
    let log_total = lattice.log_total();

    // nodes[i]：覆盖前 i 个位置、以某个词结尾的部分路径；nodes[0] 是虚拟起点
    let mut nodes: Vec<Vec<Node>> = (0..=n).map(|_| Vec::new()).collect();
    nodes[0].push(Node {
        start: 0,
        text: String::new(),
        syllables: Vec::new(),
        placeholder: false,
        arrivals: vec![Arrival::ORIGIN],
    });
    for start in 0..n {
        prune(&mut nodes[start]);
        if nodes[start].is_empty() {
            continue;
        }
        let mut any = false;
        for end in start + 1..=n.min(start + lattice.max_span()) {
            let hits = lattice.words(start, end);
            if hits.is_empty() {
                continue;
            }
            any = true;
            for hit in hits.iter() {
                let bonus = weight_bonus(weight(&hit.text));
                let fallback = fallback_log_prob(hit.frequency, log_total);
                let hit_penalty = hit.penalty + lattice.extra_penalty(start, end, hit);
                let gain = bonus - hit_penalty - personal.interpolation.word_penalty;
                let arrivals = arrivals(
                    &nodes,
                    start,
                    search,
                    &hit.text,
                    model,
                    personal,
                    fallback,
                    hit.reading,
                    (gain, hit_penalty),
                );
                nodes[end].push(Node {
                    start,
                    text: hit.text.clone(),
                    syllables: hit.syllables.clone(),
                    placeholder: false,
                    arrivals,
                });
            }
        }
        // 这个位置连单字都查不到：用敲的原样占位，别让整句断掉
        if !any {
            let text = lattice.placeholder(start).to_owned();
            let arrivals = arrivals(
                &nodes,
                start,
                search,
                &text,
                &NoModel,
                Personal::NONE,
                UNKNOWN_LOG_PROB,
                0.0,
                (0.0, 0.0),
            );
            nodes[start + 1].push(Node {
                start,
                syllables: vec![text.clone()],
                text,
                placeholder: true,
                arrivals,
            });
        }
    }
    // 整段一个词的走法可能被束宽剪掉，剪之前先记下来
    let whole = if search.whole_word {
        whole_word(&nodes[n])
    } else {
        None
    };
    prune(&mut nodes[n]);
    // 终点上全部节点的全部走法一起按得分排：前 k 条可以是同一个句尾词的几种走法
    let mut finals: Vec<(f64, usize, usize)> = nodes[n]
        .iter()
        .enumerate()
        .flat_map(|(index, node)| {
            node.arrivals
                .iter()
                .enumerate()
                .map(move |(rank, arrival)| (arrival.score, index, rank))
        })
        .collect();
    finals.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
    let mut paths: Vec<Conversion> = Vec::with_capacity(k.min(finals.len()));
    for &(_, index, rank) in &finals {
        if paths.len() >= k {
            break;
        }
        let conversion = backtrack(&nodes, n, index, rank);
        if !paths.iter().any(|p| p.text == conversion.text) {
            paths.push(conversion);
        }
    }
    if let Some(whole) = whole {
        add_whole_word(&mut paths, whole, &nodes, &finals);
    }
    paths
}

/// 把整段一个词的读法补进 `paths`（见 [`Search::whole_word`]），整词那条一定在：比分时靠它认出哪段文字是整词。
/// 同一段文字按几个词切的走法分更高时（个人 n-gram 把 工作 + 台 抬得比整词 工作台 高），那条也要在，
/// 整句与整词比分取同文字里最高的；它不在前 k 条里就从终点的全部走法里找。
fn add_whole_word(
    paths: &mut Vec<Conversion>,
    whole: Conversion,
    nodes: &[Vec<Node>],
    finals: &[(f64, usize, usize)],
) {
    let n = nodes.len() - 1;
    if !paths.iter().any(|p| p.text == whole.text) {
        // finals 按得分降序，找到的第一条同文字走法就是剪枝后留下的里面最好的
        let same = finals
            .iter()
            .map(|&(_, index, rank)| backtrack(nodes, n, index, rank))
            .find(|p| p.text == whole.text);
        if let Some(same) = same
            && same.score > whole.score
        {
            paths.push(same);
        }
    }
    if !paths
        .iter()
        .any(|p| p.text == whole.text && p.word_count() == 1)
    {
        paths.push(whole);
    }
}

/// 终点上整段只有一个词、按敲的原样读的节点（没有模糊音 / 敲错代价，不是占位）里得分最高的那条路径。
fn whole_word(nodes: &[Node]) -> Option<Conversion> {
    let node = nodes
        .iter()
        .filter(|node| node.start == 0 && !node.placeholder && node.best().penalty == 0.0)
        .max_by(|a, b| {
            a.best()
                .score
                .partial_cmp(&b.best().score)
                .unwrap_or(std::cmp::Ordering::Equal)
        })?;
    let arrival = node.best();
    Some(Conversion {
        text: node.text.clone(),
        syllables: node.syllables.clone(),
        words: vec![SentenceWord {
            text: node.text.clone(),
            syllables: node.syllables.clone(),
            placeholder: false,
        }],
        score: arrival.score,
        static_score: arrival.static_score,
        penalty: arrival.penalty,
    })
}

/// 从 `nodes[position][index]` 的第 `rank` 种走法回溯出整条路径。
fn backtrack(
    nodes: &[Vec<Node>],
    mut position: usize,
    mut index: usize,
    mut rank: usize,
) -> Conversion {
    let last = nodes[position][index].arrivals[rank];
    let mut words: Vec<SentenceWord> = Vec::new();
    while position > 0 {
        let node = &nodes[position][index];
        let arrival = node.arrivals[rank];
        words.push(SentenceWord {
            text: node.text.clone(),
            syllables: node.syllables.clone(),
            placeholder: node.placeholder,
        });
        position = node.start;
        index = arrival.back;
        rank = arrival.back_rank;
    }
    words.reverse();
    let mut text = String::new();
    let mut syllables = Vec::new();
    for word in &words {
        text.push_str(&word.text);
        syllables.extend(word.syllables.iter().cloned());
    }
    Conversion {
        text,
        syllables,
        words,
        score: last.score,
        static_score: last.static_score,
        penalty: last.penalty,
    }
}

/// 占位音节不问语言模型。
struct NoModel;

impl LanguageModel for NoModel {
    fn log_prob(&self, _context: Context<'_>, _word: &str) -> Option<f64> {
        None
    }
}

/// 到达「`word` 接在 `nodes[start]` 后面」这个节点的前 `search.paths` 种走法，按得分降序。
///
/// 转移概率先问静态模型（不认识就用词库兜底值），再与个人 n-gram 插值；每个前驱只算一次，它的几种走法共用：
/// 前一个词就是前驱的词，前二词取前驱最优走法的回指（近似，不扩状态），静态三元与个人三元都用它。
/// 路径开头接 `search.initial`（这段拼音之前的上文）：第一个词的上文就是它，第二个词的前二词是它的 `previous`。
/// `gain` 是这条边自己的加减分（用户加分 − 代价 − 每词代价）与其中计入 `penalty` 的那部分。
///
/// `reading` 是这条边的读音份额扣分。它进路径分、**不进** `static_score`：`static_score` 是留给神经重打分换掉的那一部分
/// （最终分 = 路径分 + λ·(神经分 − 静态分)），而神经模型是字级的、同样不认识读音，
/// 把份额算进去等于让重排一并把它还回来，多音字又会赢。
#[allow(clippy::too_many_arguments)]
fn arrivals(
    nodes: &[Vec<Node>],
    start: usize,
    search: Search<'_>,
    word: &str,
    model: &dyn LanguageModel,
    personal: Personal<'_>,
    fallback: f64,
    reading: f64,
    gain: (f64, f64),
) -> Vec<Arrival> {
    let (gain, edge_penalty) = gain;
    let initial = search.initial;
    let mut found: Vec<Arrival> = Vec::new();
    for (back, previous) in nodes[start].iter().enumerate() {
        let (step, static_step) = if start == 0 {
            (
                initial_step(model, personal, initial, word, fallback, reading),
                initial_log_prob(model, personal, initial, word, fallback),
            )
        } else {
            let context = Context {
                previous: Some(previous.text.as_str()),
                earlier: if previous.start > 0 {
                    Some(nodes[previous.start][previous.best().back].text.as_str())
                } else {
                    initial.previous
                },
            };
            (
                transition_log_prob(model, personal, context, word, fallback, reading),
                model.log_prob(context, word).unwrap_or(fallback),
            )
        };
        for (back_rank, arrival) in previous.arrivals.iter().enumerate() {
            found.push(Arrival {
                score: arrival.score + step + gain,
                static_score: arrival.static_score + static_step,
                penalty: arrival.penalty + edge_penalty,
                back,
                back_rank,
            });
        }
    }
    // 稳定排序：同分时留靠前的前驱，与只留最优回指时的取法一致
    found.sort_by(|a, b| {
        b.score
            .partial_cmp(&a.score)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    found.truncate(search.paths.max(1));
    found
}

/// 一段拼音第一个词的静态 log 概率：「接着上文」与「句首」两种读法按 `initial_weight` 混（见 `INITIAL_CONTEXT_WEIGHT`）。
/// 没有上文就是句首；模型不认识的词两边都用 `fallback`。
fn initial_log_prob(
    model: &dyn LanguageModel,
    personal: Personal<'_>,
    initial: Context<'_>,
    word: &str,
    fallback: f64,
) -> f64 {
    let fresh = model.log_prob(Context::START, word).unwrap_or(fallback);
    let weight = personal.interpolation.initial_weight.clamp(0.0, 1.0);
    if initial.previous.is_none() || weight <= 0.0 {
        return fresh;
    }
    let following = model.log_prob(initial, word).unwrap_or(fallback);
    mix(weight, following, fresh)
}

/// 一段拼音第一个词的路径得分：同 [`initial_log_prob`] 的混法，两种读法各自先与个人 n-gram 插值再混。
/// 个人部分不能只按上文算：个人二元里「前词 → 的」这类接续攒得最快，全按接续算 `dizhi` 会出 的只。
fn initial_step(
    model: &dyn LanguageModel,
    personal: Personal<'_>,
    initial: Context<'_>,
    word: &str,
    fallback: f64,
    reading: f64,
) -> f64 {
    let fresh = transition_log_prob(model, personal, Context::START, word, fallback, reading);
    let weight = personal.interpolation.initial_weight.clamp(0.0, 1.0);
    if initial.previous.is_none() || weight <= 0.0 {
        return fresh;
    }
    let following = transition_log_prob(model, personal, initial, word, fallback, reading);
    mix(weight, following, fresh)
}

/// `ln(w·e^a + (1−w)·e^b)`。
fn mix(weight: f64, a: f64, b: f64) -> f64 {
    (weight * a.exp() + (1.0 - weight) * b.exp())
        .max(f64::MIN_POSITIVE)
        .ln()
}

/// 按得分降序只留束宽条。
fn prune(nodes: &mut Vec<Node>) {
    nodes.sort_by(|a, b| {
        b.best()
            .score
            .partial_cmp(&a.best().score)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    nodes.truncate(BEAM_WIDTH);
}

#[cfg(test)]
mod tests;
