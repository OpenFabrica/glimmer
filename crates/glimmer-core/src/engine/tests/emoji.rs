//! emoji。

use super::*;

/// 英文词带出自己的 emoji，选 emoji 吃掉整段输入。
#[test]
fn english_words_bring_their_emoji_and_the_emoji_consumes_the_whole_input() {
    let words = WordList::parse("smile\nsmiled\n").unwrap();
    let table = EmojiTable::parse("smile\t😀 😄\n笑\t😄\n").unwrap();
    let mut engine = engine().with_english(words).with_emoji(table);
    engine.set_input("smile");
    let items = engine.query().unwrap().candidates.items;
    let position = items
        .iter()
        .position(|c| c.kind == CandidateKind::English && c.text == "smile")
        .unwrap();
    // 英文词也一样：emoji 跟在它后面，但不进前三位；候选不够时两个 emoji 排在最后两位
    let first = items
        .iter()
        .position(|c| c.kind == CandidateKind::Emoji)
        .unwrap();
    assert_eq!(first, (position + 1).max(3).min(items.len() - 2));
    let emoji = items[first].clone();
    assert_eq!(emoji.text, "😀");
    assert_eq!(emoji.reading.as_deref(), Some("smile"));
    assert_eq!(engine.commit(&emoji), "😀");
    assert!(engine.composition().is_empty());
}

/// emoji 跟在对应的词后面（不进前三位），选 emoji 只吃掉那个词的音节。
#[test]
fn emoji_follow_their_word_and_consume_its_syllables() {
    let table = EmojiTable::parse("开发\t👨‍💻 🛠️ 🧑‍💻\n开\t🔓\n").unwrap();
    let mut engine = engine().with_emoji(table);
    engine.set_input("kaifazhe");
    let query = engine.query().unwrap();
    let items = &query.candidates.items;
    let kaifa = items.iter().position(|c| c.text == "开发").unwrap();
    // 开发 在前三位里：它的 emoji 排到第四位起，仍按顺序跟在后面
    let first = items
        .iter()
        .position(|c| c.kind == CandidateKind::Emoji)
        .unwrap();
    assert_eq!(first, (kaifa + 1).max(3));
    assert_eq!(items[first].text, "👨‍💻");
    assert_eq!(items[first].reading.as_deref(), Some("开发"));
    assert_eq!(items[first + 1].text, "🛠️");
    // 每个词最多两个，一次最多三个
    assert_eq!(
        items
            .iter()
            .filter(|c| c.kind == CandidateKind::Emoji)
            .count(),
        3
    );
    let emoji = items[first].clone();
    assert_eq!(engine.commit(&emoji), "👨‍💻");
    assert_eq!(engine.composition().text(), "zhe");
}

/// 首选配了 emoji 时 emoji 不紧跟首选：前三位留给字词（`zhengshi` 的 🎩 👔 不把 正是 挤到第四）；候选不够三个时排在末尾。
#[test]
fn emoji_leave_the_first_three_places_to_words() {
    let table = EmojiTable::parse("开发\t👨‍💻\n不\t🚫\n").unwrap();
    let mut engine = engine().with_emoji(table);
    engine.set_input("kaifa");
    let items = engine.query().unwrap().candidates.items;
    assert_eq!(items[0].text, "开发");
    assert!(items[..3].iter().all(|c| c.kind != CandidateKind::Emoji));
    assert_eq!(items[3].text, "👨‍💻");
    assert_eq!(items[3].reading.as_deref(), Some("开发"));

    engine.set_input("bu");
    let texts: Vec<String> = engine
        .query()
        .unwrap()
        .candidates
        .items
        .into_iter()
        .map(|c| c.text)
        .collect();
    assert_eq!(texts, ["不", "🚫"]);
}

/// emoji 不拆开一串快捷写法：`sj` 的几种时间写法紧跟首选 时间，emoji 排在这一串之后。
#[test]
fn emoji_do_not_split_the_shortcut_forms() {
    let table = EmojiTable::parse("时间\t⌛ ⌚\n").unwrap();
    let mut engine =
        Engine::new(Dictionary::parse("时间\tshi jian\t1000\n").unwrap()).with_emoji(table);
    engine.set_input("sj");
    let kinds: Vec<CandidateKind> = engine
        .query()
        .unwrap()
        .candidates
        .items
        .iter()
        .map(|c| c.kind)
        .collect();
    assert_eq!(kinds[0], CandidateKind::Chinese);
    let shortcuts = kinds
        .iter()
        .filter(|k| **k == CandidateKind::Shortcut)
        .count();
    assert!(shortcuts >= 3);
    assert!(
        kinds[1..=shortcuts]
            .iter()
            .all(|k| *k == CandidateKind::Shortcut)
    );
    assert_eq!(kinds[shortcuts + 1], CandidateKind::Emoji);
}

#[test]
fn emoji_candidates_can_be_switched_off() {
    let table = EmojiTable::parse("开发\t👨‍💻 🛠️\n").unwrap();
    let mut engine = engine().with_emoji(table);
    assert!(engine.emoji_candidates());
    engine.set_emoji_candidates(false);
    engine.set_input("kaifa");
    let items = engine.query().unwrap().candidates.items;
    assert!(items.iter().any(|c| c.text == "开发"));
    assert!(items.iter().all(|c| c.kind != CandidateKind::Emoji));
    // 再打开立即生效，不用重建 Engine
    engine.set_emoji_candidates(true);
    engine.set_input("kaifa");
    let items = engine.query().unwrap().candidates.items;
    assert!(items.iter().any(|c| c.kind == CandidateKind::Emoji));
}
