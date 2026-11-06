use super::*;

fn candidate(text: &str) -> Candidate {
    Candidate {
        text: text.to_owned(),
        kind: glimmer_core::CandidateKind::Chinese,
        syllables: Vec::new(),
        reading: None,
        translation: None,
    }
}

#[test]
fn records_and_round_trips_through_tsv() {
    let mut learner = FrequencyLearner::default();
    learner.record(&candidate("开发"));
    learner.record(&candidate("开发"));
    learner.record(&candidate("中文"));
    assert_eq!(learner.weight("开发"), 2);
    assert!(learner.is_dirty());

    let dir = std::env::temp_dir().join(format!("glimmer-learning-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("user.tsv");
    learner.save_to(&path).unwrap();
    assert!(!learner.is_dirty());

    let mut restored = FrequencyLearner::from_path(&path).unwrap();
    assert_eq!(restored.weight("开发"), 2);
    assert_eq!(restored.weight("中文"), 1);
    assert_eq!(restored.weight("没有"), 0);

    // flush 写回加载时的路径
    restored.record(&candidate("中文"));
    restored.flush();
    assert_eq!(
        FrequencyLearner::from_path(&path).unwrap().weight("中文"),
        2
    );
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn choices_are_keyed_by_input_and_round_trip() {
    let dir = std::env::temp_dir().join("glimmer-user-choices-test");
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("user.tsv");
    let _ = std::fs::remove_file(dir.join(USER_CHOICES_FILE));

    let mut learner = FrequencyLearner::from_path(&path).unwrap();
    learner.record_choice("ba", "吧", ChoicePosition::SentenceStart);
    learner.record_choice("ba", "吧", ChoicePosition::SentenceStart);
    learner.record_choice("bazhege", "把", ChoicePosition::SentenceStart);
    learner.record_raw("nihooma");
    assert_eq!(learner.raw_count("nihooma"), 1);
    assert_eq!(
        learner.choice_weight("ba", "吧", ChoicePosition::SentenceStart),
        2
    );
    assert_eq!(
        learner.choice_weight("ba", "把", ChoicePosition::SentenceStart),
        0
    );
    assert_eq!(
        learner.choice_weight("bazhege", "把", ChoicePosition::SentenceStart),
        1
    );
    learner.flush();

    let reloaded = FrequencyLearner::from_path(&path).unwrap();
    assert_eq!(reloaded.choice_count(), 3);
    assert_eq!(
        reloaded.choice_weight("ba", "吧", ChoicePosition::SentenceStart),
        2
    );
    assert_eq!(reloaded.raw_count("nihooma"), 1);
    assert_eq!(
        reloaded.choice_weight("bazhege", "把", ChoicePosition::SentenceStart),
        1
    );
}

#[test]
fn unrecord_reverses_each_kind_of_record() {
    let mut learner = FrequencyLearner::default();
    let candidate = Candidate {
        text: "开放".into(),
        kind: glimmer_core::CandidateKind::Chinese,
        syllables: vec!["kai".into(), "fang".into()],
        reading: None,
        translation: None,
    };
    learner.record(&candidate);
    learner.record_choice("kaifa", "开放", ChoicePosition::SentenceStart);
    learner.record_transition(Context::START, "开放", 2);
    learner.unrecord("开放");
    learner.unrecord_choice("kaifa", "开放", ChoicePosition::SentenceStart);
    learner.unrecord_transition(Context::START, "开放", 2);
    assert_eq!(learner.weight("开放"), 0);
    assert_eq!(
        learner.choice_weight("kaifa", "开放", ChoicePosition::SentenceStart),
        0
    );
    assert_eq!(learner.choice_count(), 0);
    assert!(learner.user_ngram().is_none());
    // 没记过的撤销不会变成负数
    learner.unrecord("没有");
    learner.unrecord_choice("x", "没有", ChoicePosition::SentenceStart);
    assert_eq!(learner.weight("没有"), 0);
}

#[test]
fn choices_decay_when_over_the_cap() {
    let mut learner = FrequencyLearner::default();
    learner.record_choice("a", "甲", ChoicePosition::SentenceStart);
    learner.record_choice("a", "甲", ChoicePosition::SentenceStart);
    learner.record_choice("a", "甲", ChoicePosition::SentenceStart);
    learner.record_choice("a", "乙", ChoicePosition::SentenceStart);
    learner.decay_choices();
    assert_eq!(
        learner.choice_weight("a", "甲", ChoicePosition::SentenceStart),
        1
    );
    assert_eq!(
        learner.choice_weight("a", "乙", ChoicePosition::SentenceStart),
        0
    );
    assert_eq!(learner.choice_count(), 1);
}

#[test]
fn transitions_round_trip_through_ngram_tsv() {
    let dir = std::env::temp_dir().join("glimmer-user-ngram-test");
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("user.tsv");
    let _ = std::fs::remove_file(dir.join(USER_NGRAM_FILE));

    let mut learner = FrequencyLearner::from_path(&path).unwrap();
    assert!(learner.user_ngram().is_none());
    learner.record_transition(Context::START, "我", 1);
    learner.record_transition(Context::after("我"), "想", 1);
    learner.record_transition(Context::after("我"), "想", 1);
    learner.record_transition(Context::after_two("我", "想"), "去", 1);
    assert_eq!(learner.user_ngram().unwrap().pair(Some("我"), "想"), 2);
    learner.flush();

    let reloaded = FrequencyLearner::from_path(&path).unwrap();
    // 二元 <s>我 / 我想 / 想去，三元 <s>我想 / 我想去
    assert_eq!(reloaded.ngram_transition_count(), 5);
    assert_eq!(reloaded.user_ngram().unwrap().pair(None, "我"), 1);
    assert_eq!(reloaded.user_ngram().unwrap().count("想"), 2);
    assert_eq!(
        reloaded
            .user_ngram()
            .unwrap()
            .triple(Some("我"), "想", "去"),
        1
    );
}

#[test]
fn typos_round_trip_and_unrecord() {
    let dir = std::env::temp_dir().join("glimmer-user-typos-test");
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("user.tsv");
    let _ = std::fs::remove_file(dir.join(USER_TYPOS_FILE));

    let mut learner = FrequencyLearner::from_path(&path).unwrap();
    learner.record_typo("gan", "guan");
    learner.record_typo("gan", "guan");
    learner.record_typo("shou", "shuo");
    learner.unrecord_typo("shou", "shuo");
    learner.unrecord_typo("mei", "you");
    assert_eq!(learner.typo_count("gan", "guan"), 2);
    assert_eq!(learner.typo_count("shou", "shuo"), 0);
    assert_eq!(learner.typo_count_total(), 1);
    learner.flush();

    let reloaded = FrequencyLearner::from_path(&path).unwrap();
    assert_eq!(reloaded.typo_count("gan", "guan"), 2);
    assert_eq!(reloaded.typo_count_total(), 1);
}

#[test]
fn broken_lines_are_skipped_instead_of_failing_the_load() {
    let dir = std::env::temp_dir().join("glimmer-broken-lines-test");
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("user.tsv");
    std::fs::write(&path, "开发\t3\n没有次数\n中文\tabc\n中文\t1\n").unwrap();
    std::fs::write(dir.join(USER_NGRAM_FILE), "<s>\t我\t3\n坏行\n我\t想\t2\n").unwrap();
    std::fs::write(dir.join(USER_CHOICES_FILE), "ba\t吧\t2\nba\n").unwrap();
    std::fs::write(dir.join(USER_TYPOS_FILE), "gan\tguan\tx\ngan\tguan\t1\n").unwrap();
    std::fs::write(dir.join(USER_WORDS_FILE), "账套\tzhang tao\t100\n只有词\n").unwrap();
    // 编码坏掉的字节也不能让整个文件读不了
    std::fs::write(dir.join(USER_ENGLISH_FILE), b"gist\t2\n\xff\xfe\t1\n").unwrap();

    let learner = FrequencyLearner::from_path(&path).unwrap();
    assert_eq!(learner.weight("开发"), 3);
    assert_eq!(learner.weight("中文"), 1);
    assert_eq!(learner.user_ngram().unwrap().pair(Some("我"), "想"), 2);
    assert_eq!(
        learner.choice_weight("ba", "吧", ChoicePosition::SentenceStart),
        2
    );
    assert_eq!(learner.typo_count("gan", "guan"), 1);
    assert_eq!(learner.word_count(), 1);
    assert_eq!(learner.english_count(), 2);
    assert!(learner.path.is_some());
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn saving_is_atomic_and_leaves_no_temporary_files() {
    let dir = std::env::temp_dir().join("glimmer-atomic-save-test");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("user.tsv");
    let mut learner = FrequencyLearner::from_path(&path).unwrap();
    learner.record(&candidate("开发"));
    learner.record_choice("kaifa", "开发", ChoicePosition::SentenceStart);
    learner.record_transition(Context::START, "开发", 1);
    learner.record_typo("gan", "guan");
    learner.learn_word("账套", &["zhang".into(), "tao".into()]);
    learner.learn_english("gist");
    assert!(learner.has_unsaved());
    learner.flush();
    assert!(!learner.has_unsaved());
    let names: Vec<String> = std::fs::read_dir(&dir)
        .unwrap()
        .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    assert_eq!(names.len(), 6, "{names:?}");
    assert!(names.iter().all(|name| !name.contains(".tmp")), "{names:?}");
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn missing_file_is_empty_table() {
    let learner = FrequencyLearner::from_path("/nonexistent/glimmer-user.tsv").unwrap();
    assert!(learner.is_empty());
}

#[test]
fn forget_removes_the_user_word_and_every_trace_of_learning() {
    let mut learner = FrequencyLearner::default();
    learner.learn_word("账套", &["zhang".into(), "tao".into()]);
    learner.record(&candidate("账套"));
    learner.record_choice("zhangtao", "账套", ChoicePosition::SentenceStart);
    learner.record_choice("zt", "账套", ChoicePosition::SentenceStart);
    learner.record_choice("zt", "周天", ChoicePosition::SentenceStart);
    learner.record_transition(Context::START, "账套", 1);
    learner.record_transition(Context::after("账套"), "建好", 1);
    let forgotten = learner.forget("账套");
    assert!(forgotten.user_word && forgotten.learning);
    assert!(learner.user_words().is_none());
    assert_eq!(learner.weight("账套"), 0);
    assert_eq!(
        learner.choice_weight("zt", "账套", ChoicePosition::SentenceStart),
        0
    );
    assert_eq!(
        learner.choice_weight("zt", "周天", ChoicePosition::SentenceStart),
        1
    );
    assert!(learner.user_ngram().is_none());
    // 词库词、没学过：什么都没清
    assert!(learner.forget("开发").is_nothing());
    // 只学过、不是用户词
    learner.record(&candidate("开发"));
    let forgotten = learner.forget("开发");
    assert!(!forgotten.user_word && forgotten.learning);
    // 个人英文词
    learner.learn_english("gist");
    assert!(learner.forget_english("Gist"));
    assert!(!learner.forget_english("gist"));
    assert!(learner.user_english().is_none());
}

#[test]
fn user_words_round_trip_through_tsv_and_form_a_dictionary() {
    let dir = std::env::temp_dir().join("glimmer-user-words-test");
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("user.tsv");
    let _ = std::fs::remove_file(&path);
    let _ = std::fs::remove_file(dir.join(USER_WORDS_FILE));

    let mut learner = FrequencyLearner::from_path(&path).unwrap();
    assert!(learner.user_words().is_none());
    learner.learn_word("账套", &["zhang".to_owned(), "tao".to_owned()]);
    learner.learn_word("账套", &["zhang".to_owned(), "tao".to_owned()]);
    assert_eq!(learner.word_count(), 1);
    let hits = learner
        .user_words()
        .unwrap()
        .lookup(&["zhang", "tao"], false);
    assert_eq!(hits[0].text, "账套");
    learner.flush();

    let reloaded = FrequencyLearner::from_path(&path).unwrap();
    assert_eq!(reloaded.word_count(), 1);
    let hits = reloaded.user_words().unwrap().lookup(&["zhang"], true);
    assert_eq!(hits[0].text, "账套");
}

#[test]
fn english_words_round_trip_through_tsv_and_form_a_word_list() {
    let dir = std::env::temp_dir().join("glimmer-user-english-test");
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("user.tsv");
    let _ = std::fs::remove_file(FrequencyLearner::english_path(&path));
    let mut learner = FrequencyLearner::from_path(&path).unwrap();
    assert!(learner.user_english().is_none());
    learner.learn_english("gist");
    learner.learn_english("Gist");
    learner.learn_english("python");
    assert_eq!(learner.english_count(), 2);
    // 第一次敲的写法留下，次数合并
    assert_eq!(learner.user_english().unwrap().get("gist"), Some("gist"));
    assert_eq!(learner.user_english().unwrap().complete("g", 3), ["gist"]);
    learner.flush();
    let reloaded = FrequencyLearner::from_path(&path).unwrap();
    assert_eq!(reloaded.english_count(), 2);
    assert_eq!(
        reloaded.user_english().unwrap().get("python"),
        Some("python")
    );
    let saved = std::fs::read_to_string(FrequencyLearner::english_path(&path)).unwrap();
    assert!(saved.contains("gist\t2"));
}

/// 个人英文词表的编码与随包词表一样去撇号：学过 `don't` 后敲 `dont` 能查到；`dont` 与 `don't` 各记各的次数。
/// 旧文件只存 `词\t次数`，读进来照样按新编码建表，不用迁移。
#[test]
fn personal_english_codes_drop_apostrophes_like_the_bundled_list() {
    let dir =
        std::env::temp_dir().join(format!("glimmer-user-english-code-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("user.tsv");
    let mut learner = FrequencyLearner::from_path(&path).unwrap();
    learner.learn_english("don't");
    learner.learn_english("don't");
    learner.learn_english("dont");
    assert_eq!(learner.english_count(), 2);
    let forms: Vec<(&str, u32)> = learner
        .user_english()
        .unwrap()
        .get_all("dont")
        .iter()
        .map(|e| (e.word(), e.frequency()))
        .collect();
    assert!(forms.contains(&("don't", 2)), "{forms:?}");
    assert!(forms.contains(&("dont", 1)), "{forms:?}");
    // 带撇号的编码查不到：全仓库只有去撇号这一种编码
    assert!(learner.user_english().unwrap().get_all("don't").is_empty());
    // 以前写下的文件
    std::fs::write(
        FrequencyLearner::english_path(&path),
        "# 微明个人英文词：词\\t次数\nI'm\t3\nrock'n'roll\t1\n",
    )
    .unwrap();
    let reloaded = FrequencyLearner::from_path(&path).unwrap();
    let list = reloaded.user_english().unwrap();
    assert_eq!(list.get("im"), Some("I'm"));
    assert_eq!(list.get("rocknroll"), Some("rock'n'roll"));
    assert_eq!(list.complete("rockn", 3), ["rock'n'roll"]);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn scheme_scoped_tables_live_in_a_subdirectory_and_share_text_keyed_tables() {
    let dir = std::env::temp_dir().join(format!("glimmer-learning-scheme-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("user.tsv");

    let mut wubi = FrequencyLearner::from_path_with_scheme(&path, Some("wubi86")).unwrap();
    assert_eq!(wubi.scheme_dir(), Some(dir.join("wubi86").as_path()));
    wubi.record(&candidate("中国"));
    wubi.record_choice("a", "工", ChoicePosition::SentenceStart);
    wubi.record_raw("xxxx");
    wubi.learn_word("中国", &["khlg".to_owned()]);
    wubi.record_typo("gg", "gy");
    wubi.flush();
    assert!(dir.join("wubi86").join(USER_WORDS_FILE).is_file());
    assert!(dir.join("wubi86").join(USER_CHOICES_FILE).is_file());
    assert!(dir.join("wubi86").join(USER_TYPOS_FILE).is_file());
    assert!(!dir.join(USER_WORDS_FILE).exists());
    assert!(!dir.join(USER_CHOICES_FILE).exists());

    // 拼音的学习器：词频共用，按输入串记的三张表看不到五笔的
    let pinyin = FrequencyLearner::from_path(&path).unwrap();
    assert_eq!(pinyin.weight("中国"), 1);
    assert_eq!(
        pinyin.choice_weight("a", "工", ChoicePosition::SentenceStart),
        0
    );
    assert_eq!(pinyin.raw_count("xxxx"), 0);
    assert!(pinyin.user_words().is_none());
    assert_eq!(pinyin.typo_count("gg", "gy"), 0);

    let reloaded = FrequencyLearner::from_path_with_scheme(&path, Some("wubi86")).unwrap();
    assert_eq!(
        reloaded.choice_weight("a", "工", ChoicePosition::SentenceStart),
        1
    );
    assert_eq!(reloaded.raw_count("xxxx"), 1);
    assert_eq!(reloaded.typo_count("gg", "gy"), 1);
    assert_eq!(reloaded.word_count(), 1);
    let _ = std::fs::remove_dir_all(&dir);
}

/// 给迁移测试准备一个目录：写好个人 n-gram 与老三列格式的选择表。
fn migration_dir(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("glimmer-choice-migrate-{name}"));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    // 把：句首 28 次、总 84 次；吧：句首 4 次、总 50 次
    std::fs::write(
        dir.join(USER_NGRAM_FILE),
        "<s>\t把\t28\n了\t把\t56\n<s>\t吧\t4\n好\t吧\t46\n",
    )
    .unwrap();
    std::fs::write(dir.join(USER_CHOICES_FILE), "ba\t把\t21\nba\t吧\t25\n").unwrap();
    dir
}

#[test]
fn choices_are_counted_separately_at_the_start_of_a_sentence() {
    let mut learner = FrequencyLearner::default();
    learner.record_choice("ba", "把", ChoicePosition::SentenceStart);
    learner.record_choice("ba", "吧", ChoicePosition::Continuation);
    learner.record_choice("ba", "吧", ChoicePosition::Continuation);
    assert_eq!(
        learner.choice_weight("ba", "把", ChoicePosition::SentenceStart),
        1
    );
    assert_eq!(
        learner.choice_weight("ba", "把", ChoicePosition::Continuation),
        0
    );
    assert_eq!(
        learner.choice_weight("ba", "吧", ChoicePosition::Continuation),
        2
    );
    assert_eq!(
        learner.choice_weight("ba", "吧", ChoicePosition::SentenceStart),
        0
    );
    // 一对 (输入串, 词) 仍算一条，不因为分桶多占上限
    assert_eq!(learner.choice_count(), 2);
}

#[test]
fn old_three_column_rows_load_as_unpositioned_and_count_in_both() {
    let dir = std::env::temp_dir().join("glimmer-choice-any");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    // 个人 n-gram 里没有 吧：拆不动，留在不分位置那一桶
    std::fs::write(dir.join(USER_CHOICES_FILE), "ba\t吧\t3\n").unwrap();
    let learner = FrequencyLearner::from_path(dir.join("user.tsv")).unwrap();
    assert_eq!(
        learner.choice_weight("ba", "吧", ChoicePosition::SentenceStart),
        3
    );
    assert_eq!(
        learner.choice_weight("ba", "吧", ChoicePosition::Continuation),
        3
    );
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn old_counts_are_split_by_the_personal_ngram_and_the_split_is_idempotent() {
    let dir = migration_dir("split");
    let mut learner = FrequencyLearner::from_path(dir.join("user.tsv")).unwrap();
    // 21 × 28 / 84 = 7 给句首，其余 14 给句中；25 × 4 / 50 = 2 给句首，其余 23 给句中
    let weight = |learner: &FrequencyLearner, text: &str, position| {
        learner.choice_weight("ba", text, position)
    };
    assert_eq!(weight(&learner, "把", ChoicePosition::SentenceStart), 7);
    assert_eq!(weight(&learner, "把", ChoicePosition::Continuation), 14);
    assert_eq!(weight(&learner, "吧", ChoicePosition::SentenceStart), 2);
    assert_eq!(weight(&learner, "吧", ChoicePosition::Continuation), 23);
    // 句首 把 反超 吧，这正是分桶要解决的
    assert!(
        weight(&learner, "把", ChoicePosition::SentenceStart)
            > weight(&learner, "吧", ChoicePosition::SentenceStart)
    );
    // 再迁移一次不变（新格式的行没有可拆的计数）
    learner.migrate_choices();
    assert_eq!(weight(&learner, "把", ChoicePosition::SentenceStart), 7);
    assert_eq!(weight(&learner, "把", ChoicePosition::Continuation), 14);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn migrated_choices_are_written_back_with_a_position_column() {
    let dir = migration_dir("write");
    let mut learner = FrequencyLearner::from_path(dir.join("user.tsv")).unwrap();
    assert!(learner.has_unsaved(), "迁移过就该标脏，下次落盘换成新格式");
    learner.flush();
    let written = std::fs::read_to_string(dir.join(USER_CHOICES_FILE)).unwrap();
    assert!(written.contains("ba\t把\t7\tstart\n"), "{written}");
    assert!(written.contains("ba\t把\t14\tafter\n"), "{written}");
    assert!(!written.contains("\tany\n"), "{written}");
    // 新格式读回来还是那两桶，不会再被拆一次
    let reloaded = FrequencyLearner::from_path(dir.join("user.tsv")).unwrap();
    assert_eq!(
        reloaded.choice_weight("ba", "把", ChoicePosition::SentenceStart),
        7
    );
    assert_eq!(
        reloaded.choice_weight("ba", "把", ChoicePosition::Continuation),
        14
    );
    assert!(!reloaded.has_unsaved());
    let _ = std::fs::remove_dir_all(&dir);
}
