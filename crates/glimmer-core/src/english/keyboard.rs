//! QWERTY 键位的邻键关系，给纠错判断「手指滑到了旁边的键」。

/// 三排字母键，从上到下。
const ROWS: [&[u8]; 3] = [b"qwertyuiop", b"asdfghjkl", b"zxcvbnm"];

/// 小写字母在键盘上的 (排, 列)；不是小写字母为 `None`。
fn position(key: u8) -> Option<(usize, usize)> {
    ROWS.iter()
        .enumerate()
        .find_map(|(row, keys)| keys.iter().position(|&k| k == key).map(|col| (row, col)))
}

/// 两个小写字母在 QWERTY 上是否相邻：同排左右紧挨着，或上下排斜着挨着。
///
/// 下一排比上一排往右错开约半个键，所以上排第 `c` 列与下排第 `c - 1`、`c` 列相邻
/// （`w` 与 `a` `s`，`k` 与 `m`）。按位置算而不查表，对称性由公式本身保证。
pub fn is_adjacent(a: u8, b: u8) -> bool {
    let (Some((ra, ca)), Some((rb, cb))) = (position(a), position(b)) else {
        return false;
    };
    if ra == rb {
        return ca.abs_diff(cb) == 1;
    }
    let ((upper, upper_col), (lower, lower_col)) = if ra < rb {
        ((ra, ca), (rb, cb))
    } else {
        ((rb, cb), (ra, ca))
    };
    lower == upper + 1 && (upper_col == lower_col || upper_col == lower_col + 1)
}

/// `key` 的全部邻键，按字母序；不是小写字母时为空。
pub fn neighbors(key: u8) -> impl Iterator<Item = u8> {
    (b'a'..=b'z').filter(move |&other| is_adjacent(key, other))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn adjacency_is_symmetric_and_irreflexive() {
        for a in b'a'..=b'z' {
            assert!(!is_adjacent(a, a));
            for b in b'a'..=b'z' {
                assert_eq!(is_adjacent(a, b), is_adjacent(b, a), "{a} {b}");
            }
        }
    }

    #[test]
    fn known_neighbors() {
        let of = |k: u8| String::from_utf8(neighbors(k).collect()).unwrap();
        assert_eq!(of(b'q'), "aw");
        assert_eq!(of(b's'), "adewxz");
        assert_eq!(of(b'g'), "bfhtvy");
        assert_eq!(of(b'p'), "lo");
        assert_eq!(of(b'm'), "jkn");
        assert_eq!(of(b'l'), "kop");
        assert!(is_adjacent(b'e', b'w'));
        assert!(is_adjacent(b'h', b'b'));
    }

    #[test]
    fn known_non_neighbors() {
        assert!(!is_adjacent(b'q', b'p'));
        assert!(!is_adjacent(b'a', b'x'));
        assert!(!is_adjacent(b'e', b'f'));
        assert!(!is_adjacent(b'm', b'o'));
        assert!(!is_adjacent(b'a', b'1'));
        assert_eq!(neighbors(b'A').count(), 0);
    }
}
