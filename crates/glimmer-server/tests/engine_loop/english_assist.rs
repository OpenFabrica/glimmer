//! 英文辅助开关（`[shortcut] english_assist`，缺省 Ctrl+Shift+E）：翻转 `[general] english_candidates`、写回配置、
//! 刷状态条；冲突时不拦；正在英文组词时敲过的字母不丢。

use std::sync::atomic::{AtomicU32, Ordering};

use glimmer_platform::Config;

use crate::support::*;

/// Ctrl+Shift，缺省英文辅助组合的修饰键。
const CTRL_SHIFT: KeyModifiers = KeyModifiers {
    ctrl: true,
    shift: true,
    ..ALT_OFF
};

/// 按下缺省的英文辅助组合；`english` 是当时的中英模式状态位。DLL 的保留键报小写字符。
fn assist_key(english: bool) -> KeyEvent {
    KeyEvent::new(
        u32::from(b'E'),
        Some('e'),
        KeyModifiers {
            english_mode: english,
            ..CTRL_SHIFT
        },
    )
}

/// 一段 TOML 写进临时文件再按热加载同一条路读回来（本 crate 不直接依赖 toml）。
fn config_from(text: &str) -> Config {
    static SEQ: AtomicU32 = AtomicU32::new(0);
    let seq = SEQ.fetch_add(1, Ordering::Relaxed);
    let path = std::env::temp_dir().join(format!(
        "glimmer-english-assist-config-{}-{seq}.toml",
        std::process::id()
    ));
    std::fs::write(&path, text).unwrap();
    let config = Config::load(&path).expect("parse config");
    std::fs::remove_file(&path).unwrap();
    config
}

#[test]
fn combo_flips_assist_from_completion_to_passthrough_and_back() {
    let mut router = router();
    let (_, _, frame) = type_english(&mut router, "hel");
    assert!(!frame.candidates.items.is_empty(), "缺省带补全");
    press(&mut router, function_key(0x1B));

    // 关掉：英文模式敲字母直接上屏，不出候选
    assert!(router.english_assist());
    let (outcome, commit, frame) = press(&mut router, assist_key(true));
    assert_eq!((outcome, commit), (KeyOutcome::Consumed, None));
    assert!(frame.is_empty());
    assert!(!router.english_assist(), "公开读取跟着翻");
    let (outcome, commit, frame) = press(&mut router, letter_with('h', ENGLISH));
    assert_eq!(
        (outcome, commit.as_deref()),
        (KeyOutcome::Consumed, Some("h"))
    );
    assert!(frame.is_empty(), "直通不该有候选：{frame:?}");

    // 再按打开：又有候选
    press(&mut router, assist_key(true));
    let (_, _, frame) = type_english(&mut router, "hel");
    assert!(
        candidate_texts(&frame).contains(&"hello"),
        "辅助打开后应有补全：{:?}",
        candidate_texts(&frame)
    );
}

#[test]
fn combo_in_chinese_mode_only_flips_the_setting_and_keeps_pinyin() {
    let mut router = router();
    type_letters(&mut router, "ni");
    let (outcome, commit, frame) = press(&mut router, assist_key(false));
    assert_eq!((outcome, commit), (KeyOutcome::Consumed, None));
    assert_eq!(preedit(&frame), "ni", "中文组句原样留着");
    press(&mut router, function_key(0x1B));

    // 设置已经翻了：切进英文模式就是直通
    let (_, commit, frame) = press(&mut router, letter_with('h', ENGLISH));
    assert_eq!(commit.as_deref(), Some("h"));
    assert!(frame.is_empty());
}

#[test]
fn turning_assist_off_mid_word_commits_the_typed_letters() {
    let mut router = router();
    type_english(&mut router, "hel");
    let (outcome, commit, frame) = press(&mut router, assist_key(true));
    assert_eq!(
        (outcome, commit.as_deref()),
        (KeyOutcome::Consumed, Some("hel")),
        "敲过的字母原样上屏，不丢"
    );
    assert!(frame.is_empty(), "候选窗收起：{frame:?}");
    let (_, commit, frame) = press(&mut router, letter_with('p', ENGLISH));
    assert_eq!(commit.as_deref(), Some("p"));
    assert!(frame.is_empty());
}

#[test]
fn shift_reported_uppercase_still_matches() {
    let mut router = router();
    let event = KeyEvent::new(u32::from(b'E'), Some('E'), CTRL_SHIFT);
    let (outcome, ..) = press(&mut router, event);
    assert_eq!(outcome, KeyOutcome::Consumed);
    let (_, commit, _) = press(&mut router, letter_with('h', ENGLISH));
    assert_eq!(commit.as_deref(), Some("h"), "已关掉辅助");
}

#[test]
fn toggle_is_written_back_to_the_config_file() {
    let dir = std::env::temp_dir().join(format!("glimmer-english-assist-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("config.toml");
    std::fs::write(&path, "[general]\nenglish_candidates = true\n").unwrap();
    let config = Config::load(&path).unwrap();
    let mut router = router_with(RouterConfig::from(&config));
    router.watch_config(&config, path.clone(), dir.clone(), None, None);

    press(&mut router, assist_key(true));
    assert!(!Config::load(&path).unwrap().general.english_candidates);
    press(&mut router, assist_key(false));
    assert!(Config::load(&path).unwrap().general.english_candidates);
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn conflicting_combo_is_not_intercepted() {
    // 与中英切换组合键相同：英文辅助让路，这个组合照常交给应用
    let config = config_from("[shortcut]\nmode_switch = \"ctrl+shift+e\"\n");
    let router_config = RouterConfig::from(&config);
    assert_eq!(router_config.english_assist, None);
    let mut router = router_with(router_config);
    assert_eq!(router.english_assist_keys(), None);
    let (outcome, ..) = press(&mut router, assist_key(true));
    assert_eq!(outcome, KeyOutcome::Passthrough);
    let (_, _, frame) = type_english(&mut router, "hel");
    assert!(!frame.candidates.items.is_empty(), "设置没被翻");

    // 与翻译选中文字相同：同样禁用
    let config = config_from("[shortcut]\nenglish_assist = \"ctrl+alt+t\"\n");
    assert_eq!(RouterConfig::from(&config).english_assist, None);
}

#[test]
fn hot_reload_picks_up_a_new_combo() {
    let mut router = router();
    router.apply_config(&config_from(
        "[shortcut]\nenglish_assist = \"ctrl+alt+k\"\n",
    ));
    // 旧组合不再拦
    let (outcome, ..) = press(&mut router, assist_key(true));
    assert_eq!(outcome, KeyOutcome::Passthrough);
    let ctrl_alt_k = KeyEvent::new(
        u32::from(b'K'),
        Some('k'),
        KeyModifiers {
            alt: true,
            english_mode: true,
            ..CTRL
        },
    );
    let (outcome, ..) = press(&mut router, ctrl_alt_k);
    assert_eq!(outcome, KeyOutcome::Consumed);
    let (_, commit, _) = press(&mut router, letter_with('h', ENGLISH));
    assert_eq!(commit.as_deref(), Some("h"), "新组合关掉了辅助");
}

/// 只记状态条上的 `english_assist`。
#[derive(Clone, Default)]
struct AssistStatus(Arc<Mutex<Vec<bool>>>);

impl StatusSink for AssistStatus {
    fn show_status(&self, view: StatusView) {
        self.0.lock().unwrap().push(view.english_assist);
    }

    fn hide_status(&self) {}
}

#[test]
fn status_bar_reflects_the_assist_switch() {
    let mut router = router_with(RouterConfig {
        status_enabled: true,
        ..RouterConfig::default()
    });
    let recorder = AssistStatus::default();
    router.set_status_sink(Box::new(recorder.clone()));
    router.handle(ClientMessage::ModeChanged {
        session: SESSION,
        english: true,
    });
    press(&mut router, assist_key(true));
    press(&mut router, assist_key(true));
    let calls = recorder.0.lock().unwrap().clone();
    assert_eq!(calls, vec![true, false, true]);
}
