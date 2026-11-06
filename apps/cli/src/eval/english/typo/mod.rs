//! 英文评测的注错：给一个词注一处敲错（邻键替换、漏一个字母、多一个字母、相邻对调）。
//!
//! 错法与位置由「种子 + 词在文本里的序号 + 词本身」决定：同样的文本与种子每次注出来一样，换种子就换一批错；
//! 限制评测词数（`--eval-english-words`）不改变前面那些词注的错。

mod injected;
mod kind;

pub use injected::Typo;
pub use kind::TypoKind;

/// 错前缀组只在前这么多个字母里注错。
pub const FRONT_LETTERS: usize = 3;

/// 给 `word` 注一处错。`front` 为真时只在前 [`FRONT_LETTERS`] 个字母里注（错前缀组），否则整个词都行（错字组）。
/// 先按随机数抽一种错法，这个词上用不了（太短、两个字母相同）就按顺序试下一种；四种都不行返回 `None`。
pub fn pick(word: &str, seed: u64, index: usize, front: bool) -> Option<Typo> {
    let letters: Vec<char> = word.chars().collect();
    let span = if front {
        FRONT_LETTERS.min(letters.len())
    } else {
        letters.len()
    };
    // 错字组与错前缀组对同一个词各抽各的（最高位区分），不然两组的错法总是同一种
    let mut state = seed
        ^ (index as u64).wrapping_mul(0x9e37_79b9_7f4a_7c15)
        ^ fnv1a(word)
        ^ (u64::from(front) << 63);
    let first = next(&mut state) as usize % TypoKind::ALL.len();
    for offset in 0..TypoKind::ALL.len() {
        let kind = TypoKind::ALL[(first + offset) % TypoKind::ALL.len()];
        let positions: Vec<usize> = (0..span).filter(|&at| kind.applies(&letters, at)).collect();
        if positions.is_empty() {
            continue;
        }
        let at = positions[next(&mut state) as usize % positions.len()];
        let typed = kind.apply(&letters, at, next(&mut state));
        let common = common_prefix(&typed, word);
        return Some(Typo {
            typed,
            kind,
            common,
        });
    }
    None
}

/// 两个串从头算起相同的字符数。
pub fn common_prefix(a: &str, b: &str) -> usize {
    a.chars().zip(b.chars()).take_while(|(x, y)| x == y).count()
}

/// splitmix64：只要稳定、分布够匀，不要密码学强度。
fn next(state: &mut u64) -> u64 {
    *state = state.wrapping_add(0x9e37_79b9_7f4a_7c15);
    let mut z = *state;
    z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
    z ^ (z >> 31)
}

/// FNV-1a：把词本身掺进种子。
fn fnv1a(text: &str) -> u64 {
    text.bytes().fold(0xcbf2_9ce4_8422_2325, |hash, byte| {
        (hash ^ u64::from(byte)).wrapping_mul(0x0100_0000_01b3)
    })
}
