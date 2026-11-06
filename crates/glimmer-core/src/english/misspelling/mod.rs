//! 误拼表查询：已敲的串正好是、或正是某个常见误拼的开头时，给出正确词。
//!
//! 表本身（[`glimmer_dictionary::Misspellings`]）与词表一样放数据层，文件读取也在那边；Core 只拿表做查询。

mod hit;

use glimmer_dictionary::Misspellings;

use crate::english::channel::should_correct;
pub use hit::MisspellingHit;

/// `code`（小写检索编码）在误拼表里的命中，最多 `limit` 条，按词去重（留排在前面的那条）。
///
/// 顺序：正好命中的在前（按表里先后）；然后是「`code` 是误拼的开头」的，误拼短的在前（离敲完最近），
/// 同长按误拼字节序。[`should_correct`] 不通过（少于 4 个字母、带数字或符号）时为空。
/// 开头命中是两次二分取区间，不扫全表。
pub fn misspelling_hits<'a>(
    table: &'a Misspellings,
    code: &str,
    limit: usize,
) -> Vec<MisspellingHit<'a>> {
    if limit == 0 || !should_correct(code) {
        return Vec::new();
    }
    let rows = table.with_prefix(code);
    let mut hits: Vec<MisspellingHit<'a>> = rows
        .iter()
        .map(|(typo, word)| MisspellingHit {
            word,
            typo,
            exact: typo == code,
        })
        .collect();
    // 稳定排序：同一误拼的几个正确词保持表里的先后
    hits.sort_by(|a, b| {
        b.exact
            .cmp(&a.exact)
            .then_with(|| a.typo.len().cmp(&b.typo.len()))
            .then_with(|| a.typo.cmp(b.typo))
    });
    let mut out: Vec<MisspellingHit<'a>> = Vec::with_capacity(limit.min(hits.len()));
    for hit in hits {
        if out.len() >= limit {
            break;
        }
        if out.iter().all(|kept| kept.word != hit.word) {
            out.push(hit);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn table() -> Misspellings {
        Misspellings::parse(
            "recieve\treceive\nrecieved\treceived\nreciept\treceipt\nrecieveing\treceiving\n\
             aache\tache\naache\tcache\nteh\tthe\nrecievd\treceived\n",
        )
        .unwrap()
    }

    #[test]
    fn exact_hits_come_first_and_one_typo_can_mean_several_words() {
        let t = table();
        let words = |code: &str| {
            misspelling_hits(&t, code, 9)
                .into_iter()
                .map(|h| (h.word, h.exact))
                .collect::<Vec<_>>()
        };
        assert_eq!(words("aache"), [("ache", true), ("cache", true)]);
        assert_eq!(
            words("recieve"),
            [("receive", true), ("received", false), ("receiving", false)]
        );
    }

    #[test]
    fn unfinished_typos_hit_by_prefix_shortest_first_and_deduplicated() {
        let t = table();
        let hits = misspelling_hits(&t, "recie", 9);
        let words: Vec<&str> = hits.iter().map(|h| h.word).collect();
        // reciept / recievd / recieve 同为 7 个字母按字节序，recievd 与 recieved 都是 received，只留一次
        assert_eq!(words, ["receipt", "received", "receive", "receiving"]);
        assert!(hits.iter().all(|h| !h.exact));
        assert_eq!(misspelling_hits(&t, "recie", 2).len(), 2);
    }

    #[test]
    fn short_or_non_letter_input_is_not_corrected() {
        let t = table();
        assert!(misspelling_hits(&t, "teh", 9).is_empty());
        assert!(misspelling_hits(&t, "rec1", 9).is_empty());
        assert!(misspelling_hits(&t, "zzzz", 9).is_empty());
        assert!(misspelling_hits(&t, "recieve", 0).is_empty());
    }
}
