//! 英文模式：候选与选词、Caps 直出、按应用关候选、中英切换时的缓冲。

use crate::support::*;

#[test]
fn english_mode_gives_candidates_and_space_picks_highlighted() {
    let mut router = router();
    let (outcome, commit, frame) = type_english(&mut router, "hel");
    assert_eq!((outcome, commit), (KeyOutcome::Consumed, None));
    assert_eq!(preedit(&frame), "hel", "英文模式敲的字母原样显示");
    let texts = candidate_texts(&frame);
    assert!(
        texts.contains(&"hello") && texts.contains(&"help"),
        "候选应来自英文词表：{texts:?}"
    );
    // 空格与中文模式一样选高亮的词，词后接上空格（放行会让应用先插空格）。
    let first = frame.candidates.items[0].text.clone();
    let (outcome, commit, after) = press(&mut router, KeyEvent::new(0x20, Some(' '), ENGLISH));
    assert_eq!(outcome, KeyOutcome::Consumed);
    assert_eq!(commit, Some(format!("{first} ")));
    assert!(after.is_empty());

    // 回车仍把字母原样上屏：词表没有的写法靠它。
    type_english(&mut router, "hel");
    let (outcome, commit, _) = press(&mut router, function_key(0x0D));
    assert_eq!(
        (outcome, commit.as_deref()),
        (KeyOutcome::Consumed, Some("hel"))
    );
}

#[test]
fn english_digits_pick_candidates_or_join_the_word() {
    let mut router = router();
    // 有候选：数字选当前页第 N 个，与中文模式一样。
    let (_, _, frame) = type_english(&mut router, "hel");
    let second = frame.candidates.items[1].text.clone();
    let (outcome, commit, after) = press(&mut router, KeyEvent::new(0x32, Some('2'), ENGLISH));
    assert_eq!((outcome, commit), (KeyOutcome::Consumed, Some(second)));
    assert!(after.is_empty());

    // 没候选（词表没有的词）：数字是标识符的一部分，回车整段原样上屏。
    let (_, _, frame) = type_english(&mut router, "xq");
    assert!(
        frame.candidates.items.is_empty(),
        "样例词表里没有 xq 开头的词"
    );
    let (outcome, commit, frame) = press(&mut router, KeyEvent::new(0x31, Some('1'), ENGLISH));
    assert_eq!((outcome, commit), (KeyOutcome::Consumed, None));
    assert_eq!(preedit(&frame), "xq1");
    let (_, commit, _) = press(&mut router, function_key(0x0D));
    assert_eq!(commit.as_deref(), Some("xq1"));
}

#[test]
fn caps_lock_types_direct_uppercase_english_regardless_of_mode() {
    let mut router = router();
    let (outcome, commit, frame) = press(&mut router, letter_with('H', CAPS));
    assert_eq!(
        (outcome, commit.as_deref()),
        (KeyOutcome::Consumed, Some("H"))
    );
    assert!(frame.is_empty(), "Caps 直接上屏不出候选：{frame:?}");
    // 组句中 Caps 亮着敲字母：拼音先原样上屏，再接大写字母。
    type_letters(&mut router, "ni");
    let (_, commit, after) = press(&mut router, letter_with('A', CAPS));
    assert_eq!(commit.as_deref(), Some("niA"));
    assert!(after.is_empty());
}

#[test]
fn english_candidates_are_off_in_listed_apps_by_exe_name() {
    // VS Code 在缺省名单里（exe 名不区分大小写）：英文模式字母直插、不出候选。
    let mut router = router_in_app("code.exe");
    let (outcome, commit, frame) = press(&mut router, letter_with('h', ENGLISH));
    assert_eq!(
        (outcome, commit.as_deref()),
        (KeyOutcome::Consumed, Some("h"))
    );
    assert!(frame.is_empty(), "名单里的应用不该有候选：{frame:?}");
    let (outcome, commit, _) = press(&mut router, KeyEvent::new(0x20, Some(' '), ENGLISH));
    assert_eq!((outcome, commit), (KeyOutcome::Passthrough, None));
    // 中文模式不受名单影响。
    let (_, _, frame) = type_letters(&mut router, "ni");
    assert!(!frame.candidates.items.is_empty(), "拼音照常出候选");
}

#[test]
fn english_candidates_stay_on_in_other_apps() {
    let mut router = router_in_app("notepad.exe");
    let (outcome, commit, frame) = type_english(&mut router, "hel");
    assert_eq!((outcome, commit), (KeyOutcome::Consumed, None));
    assert!(
        candidate_texts(&frame).contains(&"hello"),
        "不在名单里的应用照常给英文候选：{frame:?}"
    );
}

#[test]
fn app_list_is_looked_up_per_session() {
    // 两个应用同时在线：切会话时按各自的 exe 名判断。
    let mut router = router_in_app("Code.exe");
    let notepad = SessionId(2);
    router.handle(ClientMessage::OpenSession {
        session: notepad,
        app: Some("notepad.exe".to_owned()),
        protocol: PROTOCOL_VERSION,
    });
    let (_, _, frame) = key_result(router.handle(ClientMessage::Key {
        session: notepad,
        event: letter_with('h', ENGLISH),
    }));
    assert_eq!(preedit(&frame), "h", "记事本会话组词");
    // 切回编辑器会话：残留组句清掉，字母直插。
    let (outcome, commit, after) = press(&mut router, letter_with('e', ENGLISH));
    assert_eq!(
        (outcome, commit.as_deref()),
        (KeyOutcome::Consumed, Some("e"))
    );
    assert!(after.is_empty());
}

#[test]
fn english_tab_and_arrow_keys_pick_candidates() {
    let mut router = router();
    let (_, _, frame) = type_english(&mut router, "hel");
    let first = frame.candidates.items[0].text.clone();
    // Tab 选高亮的词。
    let (outcome, commit, _) = press(&mut router, KeyEvent::new(0x09, None, ENGLISH));
    assert_eq!(
        (outcome, commit.as_deref()),
        (KeyOutcome::Consumed, Some(first.as_str()))
    );

    // 方向键移到第二个之后，空格选的是它，再接上空格。
    let (_, _, frame) = type_english(&mut router, "hel");
    let second = frame.candidates.items[1].text.clone();
    let (outcome, _, _) = press(&mut router, KeyEvent::new(0x28, None, ENGLISH));
    assert_eq!(outcome, KeyOutcome::Consumed);
    let (_, commit, after) = press(&mut router, KeyEvent::new(0x20, Some(' '), ENGLISH));
    assert_eq!(commit, Some(format!("{second} ")));
    assert!(after.is_empty());
}

#[test]
fn english_without_candidates_is_passthrough_with_shift_case() {
    let mut router = router_with(RouterConfig {
        english_candidates: false,
        ..RouterConfig::default()
    });
    // 字母由我们插入，大小写按 Shift；不组句。
    let (outcome, commit, frame) = press(&mut router, letter_with('h', ENGLISH));
    assert_eq!(
        (outcome, commit.as_deref()),
        (KeyOutcome::Consumed, Some("h"))
    );
    assert!(frame.is_empty());
    let shifted = KeyModifiers {
        shift: true,
        ..ENGLISH
    };
    let (_, commit, _) = press(&mut router, letter_with('H', shifted));
    assert_eq!(commit.as_deref(), Some("H"));
    // 其他键交给应用。
    let (outcome, commit, _) = press(&mut router, KeyEvent::new(0x20, Some(' '), ENGLISH));
    assert_eq!((outcome, commit), (KeyOutcome::Passthrough, None));
}

#[test]
fn switching_to_chinese_mid_word_flushes_english_letters() {
    let mut router = router();
    type_english(&mut router, "hel");
    // 切回中文模式再敲字母：之前的英文字母原样上屏，新字母从头当拼音。
    let (outcome, commit, frame) = press(&mut router, letter('l'));
    assert_eq!(
        (outcome, commit.as_deref()),
        (KeyOutcome::Consumed, Some("hel"))
    );
    assert_eq!(preedit(&frame), "l");
}

#[test]
fn english_digit_without_a_slot_joins_the_word() {
    let mut router = router();
    let (_, _, frame) = type_english(&mut router, "hello");
    let shown = frame.candidates.items.len();
    assert!((1..9).contains(&shown), "hello 的候选应不满 9 个：{shown}");
    let (outcome, commit, frame) = press(&mut router, digit_with(9, ENGLISH));
    assert_eq!((outcome, commit), (KeyOutcome::Consumed, None));
    assert_eq!(preedit(&frame), "hello9");
}

#[test]
fn keys_passed_through_outside_composition_reset_the_english_context() {
    use glimmer_core::english::NgramHistory;

    let mut router = router();
    type_english(&mut router, "hello");
    let (_, commit, _) = press(&mut router, KeyEvent::new(0x20, Some(' '), ENGLISH));
    assert_eq!(commit.as_deref(), Some("hello "));
    assert_eq!(
        router.engine_mut().english_history(),
        NgramHistory::Previous("hello"),
        "选中的英文词记进英文上文，空格不断开"
    );
    // 组句之外的方向键交给应用：光标可能动了，上文作废
    let (outcome, _, _) = press(&mut router, function_key(0x25));
    assert_eq!(outcome, KeyOutcome::Passthrough);
    assert_eq!(router.engine_mut().english_history(), NgramHistory::Unknown);
    // 回车是句首
    type_english(&mut router, "hello");
    press(&mut router, KeyEvent::new(0x20, Some(' '), ENGLISH));
    press(&mut router, function_key(0x0D));
    assert_eq!(router.engine_mut().english_history(), NgramHistory::START);
}

/// 英文模式敲 `word` 再按空格选词上屏，返回空格那一键的结果。
fn commit_with_space(router: &mut Router, word: &str) -> (KeyOutcome, Option<String>, Frame) {
    type_english(router, word);
    press(router, english_space())
}

#[test]
fn space_after_an_english_word_shows_the_next_word_prediction() {
    let mut router = router_predicting(true);
    let (outcome, commit, frame) = commit_with_space(&mut router, "hello");
    assert_eq!(
        (outcome, commit.as_deref()),
        (KeyOutcome::Consumed, Some("hello "))
    );
    assert!(frame.preedit.is_empty(), "预测帧没有 preedit：{frame:?}");
    assert_eq!(candidate_texts(&frame), ["world"]);
    assert_eq!(frame.highlight, 0);
    // 轮询拿到的是同一帧（没过期就不收）
    assert_eq!(poll(&mut router), frame);
}

#[test]
fn tab_accepts_the_prediction_and_chains_to_the_next() {
    let mut router = router_predicting(true);
    commit_with_space(&mut router, "hello");
    let (outcome, commit, frame) = press(&mut router, english_key(0x09));
    assert_eq!(
        (outcome, commit.as_deref()),
        (KeyOutcome::Consumed, Some("world "))
    );
    assert_eq!(candidate_texts(&frame), ["the"], "接受后马上出再下一个");
    assert!(frame.preedit.is_empty());
    let (outcome, commit, frame) = press(&mut router, english_key(0x09));
    assert_eq!(
        (outcome, commit.as_deref()),
        (KeyOutcome::Consumed, Some("the "))
    );
    assert!(frame.is_empty(), "the 之后没有预测：{frame:?}");
    // 没有预测时 Tab 照旧交给应用
    let (outcome, commit, frame) = press(&mut router, english_key(0x09));
    assert_eq!((outcome, commit), (KeyOutcome::Passthrough, None));
    assert!(frame.is_empty());
}

#[test]
fn shift_tab_does_not_accept_the_prediction() {
    let mut router = router_predicting(true);
    commit_with_space(&mut router, "hello");
    let shifted = KeyModifiers {
        shift: true,
        ..ENGLISH
    };
    let (outcome, commit, frame) = press(&mut router, KeyEvent::new(0x09, None, shifted));
    assert_eq!((outcome, commit), (KeyOutcome::Passthrough, None));
    assert!(frame.is_empty());
}

#[test]
fn letter_during_prediction_starts_a_normal_composition() {
    let mut router = router_predicting(true);
    commit_with_space(&mut router, "hello");
    let (outcome, commit, frame) = press(&mut router, letter_with('w', ENGLISH));
    assert_eq!((outcome, commit), (KeyOutcome::Consumed, None));
    assert_eq!(preedit(&frame), "w");
    let texts = candidate_texts(&frame);
    assert!(
        texts.contains(&"world") && texts.contains(&"word"),
        "正常的英文候选：{texts:?}"
    );
}

/// 预测显示着时按 `event` 与预测关着时按它，回话一模一样，帧都是空的。
fn assert_same_as_without_prediction(event: KeyEvent, what: &str) {
    let mut shown = router_predicting(true);
    let mut plain = router_predicting(false);
    let (_, _, frame) = commit_with_space(&mut shown, "hello");
    assert!(!frame.is_empty(), "前提：预测显示着");
    let (_, _, frame) = commit_with_space(&mut plain, "hello");
    assert!(frame.is_empty(), "前提：预测关着");
    let with = press(&mut shown, event);
    let without = press(&mut plain, event);
    assert_eq!(
        (with.0, &with.1),
        (without.0, &without.1),
        "{what}：返回值应与预测没显示时一样"
    );
    assert!(with.2.is_empty(), "{what}：预测收起：{:?}", with.2);
    assert!(without.2.is_empty());
}

#[test]
fn other_keys_during_prediction_behave_exactly_as_without_it() {
    assert_same_as_without_prediction(digit_with(1, ENGLISH), "数字");
    assert_same_as_without_prediction(english_space(), "空格");
    assert_same_as_without_prediction(english_key(0x0D), "回车");
    assert_same_as_without_prediction(english_key(0x1B), "Esc");
    assert_same_as_without_prediction(english_key(0x08), "退格");
    for (vk, what) in [(0x25, "左"), (0x26, "上"), (0x27, "右"), (0x28, "下")] {
        assert_same_as_without_prediction(english_key(vk), what);
    }
    assert_same_as_without_prediction(KeyEvent::new(0xBC, Some(','), ENGLISH), "逗号");
    let ctrl = KeyModifiers {
        ctrl: true,
        ..ENGLISH
    };
    assert_same_as_without_prediction(KeyEvent::new(0x43, Some('c'), ctrl), "Ctrl+C");
    assert_same_as_without_prediction(KeyEvent::new(0x5A, Some('z'), ctrl), "Ctrl+Z");
    let shift = KeyModifiers {
        shift: true,
        ..ENGLISH
    };
    assert_same_as_without_prediction(KeyEvent::new(0x25, None, shift), "Shift+左");
}

#[test]
fn second_space_does_not_bring_the_prediction_back() {
    let mut router = router_predicting(true);
    commit_with_space(&mut router, "hello");
    let (outcome, _, frame) = press(&mut router, english_space());
    assert_eq!(outcome, KeyOutcome::Passthrough);
    assert!(frame.is_empty());
    assert!(poll(&mut router).is_empty(), "轮询也不重新取");
}

#[test]
fn prediction_off_shows_nothing_and_tab_passes_through() {
    let mut router = router_predicting(false);
    let (_, commit, frame) = commit_with_space(&mut router, "hello");
    assert_eq!(commit.as_deref(), Some("hello "));
    assert!(frame.is_empty());
    let (outcome, commit, _) = press(&mut router, english_key(0x09));
    assert_eq!((outcome, commit), (KeyOutcome::Passthrough, None));
}

#[test]
fn prediction_follows_the_config_reload() {
    let mut router = router_predicting(true);
    let mut config = glimmer_platform::Config::default();
    config.general.english_prediction = false;
    router.apply_config(&config);
    let (_, _, frame) = commit_with_space(&mut router, "hello");
    assert!(frame.is_empty(), "热加载关掉后不出");
    config.general.english_prediction = true;
    router.apply_config(&config);
    let (_, _, frame) = commit_with_space(&mut router, "hello");
    assert_eq!(candidate_texts(&frame), ["world"]);
}

#[test]
fn chinese_mode_never_predicts() {
    let mut router = router_predicting(true);
    type_letters(&mut router, "nihao");
    let (_, commit, frame) = press(&mut router, KeyEvent::new(0x20, Some(' '), ALT_OFF));
    assert!(commit.is_some());
    assert!(frame.is_empty());
    // 中文模式里敲英文词上屏也不出（预测只在英文模式）
    type_letters(&mut router, "hello");
    let (_, _, frame) = press(&mut router, KeyEvent::new(0x20, Some(' '), ALT_OFF));
    assert!(frame.is_empty(), "{frame:?}");
}

#[test]
fn poll_hides_the_prediction_once_core_stops_giving_it() {
    // 真实的过期是英文上文 5 秒不动；这里用关掉开关造出「Core 不再给」，看轮询会不会收起
    let mut router = router_predicting(true);
    commit_with_space(&mut router, "hello");
    router.engine_mut().set_english_prediction(false);
    assert!(poll(&mut router).is_empty());
    router.engine_mut().set_english_prediction(true);
    assert!(poll(&mut router).is_empty(), "收起后轮询不再重新取");
}

#[test]
fn hide_candidates_during_prediction_drops_it_and_the_english_context() {
    // 壳不送的键（空组句时的空格、退格、方向键、Ctrl 组合）原样交给应用，只发 HideCandidates
    let mut router = router_predicting(true);
    commit_with_space(&mut router, "hello");
    router.handle(ClientMessage::HideCandidates { session: SESSION });
    assert!(poll(&mut router).is_empty());
    assert_eq!(router.engine_mut().english_history(), NgramHistory::Unknown);
    let (outcome, commit, _) = press(&mut router, english_key(0x09));
    assert_eq!((outcome, commit), (KeyOutcome::Passthrough, None));
}

#[test]
fn focus_and_mode_changes_clear_the_prediction() {
    let mut router = router_predicting(true);
    commit_with_space(&mut router, "hello");
    router.handle(ClientMessage::ModeChanged {
        session: SESSION,
        english: false,
    });
    assert!(poll(&mut router).is_empty(), "切中英收起");

    commit_with_space(&mut router, "hello");
    router.handle(ClientMessage::Commit { session: SESSION });
    assert!(poll(&mut router).is_empty(), "失焦收起");

    commit_with_space(&mut router, "hello");
    let other = SessionId(2);
    router.handle(ClientMessage::OpenSession {
        session: other,
        app: None,
        protocol: PROTOCOL_VERSION,
    });
    let (outcome, commit, frame) = key_result(router.handle(ClientMessage::Key {
        session: other,
        event: english_key(0x09),
    }));
    assert_eq!(
        (outcome, commit),
        (KeyOutcome::Passthrough, None),
        "切会话后 Tab 不接受上一个会话的预测"
    );
    assert!(frame.is_empty());
}

/// 记下候选窗口的显示 / 收起：`Some(第一个候选)` 是显示，`None` 是收起。
#[derive(Clone, Default)]
struct RecordingWindow(Arc<Mutex<Vec<Option<String>>>>);

impl glimmer_server::dispatch::CandidateSink for RecordingWindow {
    fn show(&self, frame: Frame, _rect: glimmer_platform::protocol::ScreenRect) {
        let first = frame.candidates.items.first().map(|c| c.text.clone());
        self.0.lock().unwrap().push(Some(first.unwrap_or_default()));
    }

    fn hide(&self) {
        self.0.lock().unwrap().push(None);
    }

    fn configure(&self, _settings: glimmer_server::dispatch::RenderSettings) {}
}

#[test]
fn prediction_window_waits_for_the_new_caret_position() {
    let window = RecordingWindow::default();
    let mut router = router_predicting(true);
    router.set_candidate_sink(Box::new(window.clone()));
    let rect = glimmer_platform::protocol::ScreenRect {
        left: 10,
        top: 10,
        right: 12,
        bottom: 30,
    };
    type_english(&mut router, "hello");
    router.handle(ClientMessage::PositionCandidates {
        session: SESSION,
        rect,
    });
    window.0.lock().unwrap().clear();
    press(&mut router, english_space());
    // 上屏后先收起，不在组句的旧位置画预测
    assert_eq!(window.0.lock().unwrap().last(), Some(&None));
    // 壳报来插入点后才画
    router.handle(ClientMessage::PositionCandidates {
        session: SESSION,
        rect,
    });
    assert_eq!(
        window.0.lock().unwrap().last(),
        Some(&Some("world".to_owned()))
    );
}

#[test]
fn tab_before_the_window_is_drawn_is_not_accepted() {
    // Windows 上屏走异步编辑会话：插入点位置晚到时 Tab 可能先到，那时窗口还没画，不接受一个没看见的词
    let window = RecordingWindow::default();
    let mut router = router_predicting(true);
    router.set_candidate_sink(Box::new(window));
    let (_, _, frame) = commit_with_space(&mut router, "hello");
    assert_eq!(candidate_texts(&frame), ["world"]);
    let (outcome, commit, frame) = press(&mut router, english_key(0x09));
    assert_eq!((outcome, commit), (KeyOutcome::Passthrough, None));
    assert!(frame.is_empty());

    // 位置到了、窗口画出来以后才接受
    commit_with_space(&mut router, "hello");
    router.handle(ClientMessage::PositionCandidates {
        session: SESSION,
        rect: glimmer_platform::protocol::ScreenRect {
            left: 10,
            top: 10,
            right: 12,
            bottom: 30,
        },
    });
    let (outcome, commit, _) = press(&mut router, english_key(0x09));
    assert_eq!(
        (outcome, commit.as_deref()),
        (KeyOutcome::Consumed, Some("world "))
    );
}

#[test]
fn next_word_frames_are_told_apart_from_composing_and_translation() {
    use glimmer_core::{Candidate, CandidateKind, CandidateList};
    use glimmer_server::dispatch::{is_next_word_frame, next_word_label};

    let mut router = router_predicting(true);
    let (_, _, composing) = type_english(&mut router, "hello");
    assert!(!is_next_word_frame(&composing));
    let (_, _, predicted) = press(&mut router, english_space());
    assert!(is_next_word_frame(&predicted));
    let translation = Frame {
        candidates: CandidateList {
            items: vec![Candidate {
                text: "你好".to_owned(),
                kind: CandidateKind::Cloud,
                syllables: Vec::new(),
                reading: None,
                translation: None,
            }],
        },
        ..Frame::default()
    };
    assert!(!is_next_word_frame(&translation), "翻译评审帧照常标 1");
    assert_eq!((next_word_label(0), next_word_label(1)), ("⇥", ""));
}

#[test]
fn private_input_never_predicts_and_tab_passes_through() {
    // 密码 / PIN / 应用要求不记录的输入框：DLL / IBus 报 Privacy，之后英文模式词后空格不出预测
    let mut router = router_predicting(true);
    router.handle(ClientMessage::Privacy {
        session: SESSION,
        private: true,
    });
    let (_, commit, frame) = commit_with_space(&mut router, "hello");
    assert_eq!(commit.as_deref(), Some("hello "), "英文候选照常");
    assert!(frame.is_empty(), "私密输入框不出预测：{frame:?}");
    let (outcome, commit, _) = press(&mut router, english_key(0x09));
    assert_eq!((outcome, commit), (KeyOutcome::Passthrough, None));

    // 离开私密后照常出
    router.handle(ClientMessage::Privacy {
        session: SESSION,
        private: false,
    });
    let (_, _, frame) = commit_with_space(&mut router, "hello");
    assert_eq!(candidate_texts(&frame), ["world"]);
}

#[test]
fn late_privacy_report_drops_a_shown_prediction() {
    // Windows 在起组句的异步编辑会话里才报私密：报晚了时预测可能已出，收到就收起
    let mut router = router_predicting(true);
    let (_, _, frame) = commit_with_space(&mut router, "hello");
    assert_eq!(candidate_texts(&frame), ["world"]);
    router.handle(ClientMessage::Privacy {
        session: SESSION,
        private: true,
    });
    assert!(poll(&mut router).is_empty());
    let (outcome, commit, _) = press(&mut router, english_key(0x09));
    assert_eq!((outcome, commit), (KeyOutcome::Passthrough, None));
}
