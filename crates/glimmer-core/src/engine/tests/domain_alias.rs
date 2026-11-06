//! 开发名称和混合词必须可选择、可上屏，并随词库开关撤销。

use super::engine;
use crate::CandidateKind;
use glimmer_dictionary::Dictionary;

const WORDS: &str = "C++\t@cpp\t40\nC#\t@csharp\t40\nC++\t@cplusplus\t40\nGit分支\t@gitfenzhi\t30\nAPI接口\t@apijiekou\t30\nAGENTS.md\t@agentsmd\t40\n";

#[test]
fn aliases_commit_full_text_in_chinese_and_english_modes() {
    for english in [false, true] {
        for (input, expected) in [
            ("cpp", "C++"),
            ("csharp", "C#"),
            ("gitfenzhi", "Git分支"),
            ("apijiekou", "API接口"),
            ("agentsmd", "AGENTS.md"),
        ] {
            let mut e = engine();
            e.set_extra_dictionaries(vec![Dictionary::parse(WORDS).unwrap()]);
            e.set_english_mode(english);
            e.set_input(input);
            let query = e.query().unwrap();
            let hits: Vec<_> = query
                .candidates
                .items
                .iter()
                .filter(|c| c.text == expected)
                .collect();
            assert_eq!(hits.len(), 1, "{input}");
            assert_eq!(hits[0].kind, CandidateKind::English);
            assert_eq!(e.commit(hits[0]), expected);
            assert!(e.composition().is_empty(), "{input}");
        }
    }
}

#[test]
fn disabling_domain_removes_mixed_completions_and_alias_duplicates() {
    let mut e = engine();
    e.set_extra_dictionaries(vec![Dictionary::parse(WORDS).unwrap()]);
    e.set_english_mode(true);
    e.set_input("c");
    let q = e.query().unwrap();
    assert_eq!(
        q.candidates
            .items
            .iter()
            .filter(|c| c.text == "C++")
            .count(),
        1
    );
    e.set_input("gitfen");
    assert!(
        e.query()
            .unwrap()
            .candidates
            .items
            .iter()
            .any(|c| c.text == "Git分支")
    );
    e.set_extra_dictionaries(vec![]);
    assert!(
        !e.query()
            .unwrap()
            .candidates
            .items
            .iter()
            .any(|c| c.text == "Git分支")
    );
}

#[test]
fn enabled_domain_completion_beats_generic_frequency_but_not_exact_match() {
    // 词频是 Zipf × 1000：spring 约 5（6 是 about 这一级的高频词，会让「多敲了一个 b」的纠错与补全打平）
    let general = glimmer_dictionary::WordList::parse(
        "springboard\tspringboard\t4000\nspringbok\tspringbok\t3500\nspring\tspring\t5000\n",
    )
    .unwrap();
    let mut e = engine().with_english(general);
    e.set_extra_dictionaries(vec![
        Dictionary::parse("Spring Boot\t@springboot\t40\n").unwrap(),
    ]);
    e.set_english_mode(true);
    e.set_input("springb");
    assert_eq!(e.query().unwrap().candidates.items[0].text, "Spring Boot");
    e.set_input("spring");
    assert_eq!(e.query().unwrap().candidates.items[0].text, "spring");
    e.set_extra_dictionaries(vec![]);
    e.set_input("springb");
    assert_eq!(e.query().unwrap().candidates.items[0].text, "springboard");
}
