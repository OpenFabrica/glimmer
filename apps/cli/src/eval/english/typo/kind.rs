//! 注错的四种错法。

use glimmer_core::correction::typo::adjacent;

/// 一处敲错的错法；都只动字母，不动撇号。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TypoKind {
    /// 一个字母敲成键盘上挨着它的另一个字母。
    Neighbor,

    /// 漏敲一个字母。
    Missing,

    /// 多敲一个字母（挨着它的键，插在它前面）。
    Extra,

    /// 相邻两个字母敲反。
    Transpose,
}

impl TypoKind {
    pub const ALL: [Self; 4] = [Self::Neighbor, Self::Missing, Self::Extra, Self::Transpose];

    /// 报告里的名字。
    pub fn label(self) -> &'static str {
        match self {
            Self::Neighbor => "邻键替换",
            Self::Missing => "漏一个字母",
            Self::Extra => "多一个字母",
            Self::Transpose => "相邻对调",
        }
    }

    /// 在 [`Self::ALL`] 里的下标，报告按它分桶。
    pub fn index(self) -> usize {
        Self::ALL.iter().position(|&kind| kind == self).unwrap_or(0)
    }

    /// 这种错法能不能落在第 `at` 个字母上。
    ///
    /// 漏敲要求它与后一个字母不同、对调要求两个字母不同：否则敲出来的串在错处之前仍是原词的前缀，
    /// 「从哪一键起错了」就对不上（`hello` 漏第一个 `l` 与漏第二个是同一个串，只算后一个）。
    pub fn applies(self, letters: &[char], at: usize) -> bool {
        let Some(&letter) = letters.get(at) else {
            return false;
        };
        if !letter.is_ascii_alphabetic() {
            return false;
        }
        match self {
            Self::Neighbor | Self::Extra => !neighbors(letter).is_empty(),
            Self::Missing => letters.len() >= 2 && letters.get(at + 1) != Some(&letter),
            Self::Transpose => letters
                .get(at + 1)
                .is_some_and(|&next| next.is_ascii_alphabetic() && next != letter),
        }
    }

    /// 在第 `at` 个字母上注这种错；`choice` 用来在几个邻键里挑一个。调用前要先过 [`Self::applies`]。
    pub fn apply(self, letters: &[char], at: usize, choice: u64) -> String {
        let mut typed = letters.to_vec();
        match self {
            Self::Neighbor => typed[at] = pick_neighbor(letters[at], choice),
            Self::Missing => {
                typed.remove(at);
            }
            Self::Extra => typed.insert(at, pick_neighbor(letters[at], choice)),
            Self::Transpose => typed.swap(at, at + 1),
        }
        typed.into_iter().collect()
    }
}

/// 键盘上挨着 `letter` 的字母（小写，按字母序）。
fn neighbors(letter: char) -> Vec<char> {
    let lower = letter.to_ascii_lowercase();
    ('a'..='z').filter(|&c| adjacent(lower, c)).collect()
}

/// 挑一个邻键，大小写跟着原字母。
fn pick_neighbor(letter: char, choice: u64) -> char {
    let around = neighbors(letter);
    let picked = around[(choice % around.len() as u64) as usize];
    if letter.is_ascii_uppercase() {
        picked.to_ascii_uppercase()
    } else {
        picked
    }
}
