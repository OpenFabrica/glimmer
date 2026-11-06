//! 从 Rust 调 C 接口：回显后端下的按键 → 指令、释放、中英切换、空指针，以及指令换成 C 形状的细节。

use std::ffi::{CStr, c_char, c_int};

use glimmer_linux::frontend::Output;
use glimmer_linux::frontend::view::{CandidateRow, CandidateView, PreeditView};

use super::*;

/// X keysym：Shift_L。
const SHIFT_L: u32 = 0xffe1;

/// X keysym：空格。
const SPACE: u32 = 0x20;

/// 装回显后端（进程内只建一次，各测试共用）并开一个会话。
fn session() -> *mut GlimmerSession {
    // SAFETY: 空指针是合法的 data_root
    assert_eq!(unsafe { glimmer_backend_init(std::ptr::null(), 1) }, 0);
    let session = glimmer_session_new();
    assert!(!session.is_null());
    session
}

/// C 字符串读成 `String`；空指针为 `None`。
fn read(text: *const c_char) -> Option<String> {
    // SAFETY: 访问器返回的指针在 outputs 释放前有效
    (!text.is_null()).then(|| {
        unsafe { CStr::from_ptr(text) }
            .to_string_lossy()
            .into_owned()
    })
}

/// 按一次键（按下或释放），返回（吃不吃，指令种类）。指令读完即释放。
fn key(session: *mut GlimmerSession, keysym: u32, release: bool) -> (bool, Vec<c_int>) {
    let mut handled = -1;
    // SAFETY: 会话有效，handled 可写
    let outputs =
        unsafe { glimmer_session_key(session, keysym, 0, c_int::from(release), &mut handled) };
    assert!(!outputs.is_null());
    // SAFETY: outputs 刚由事件函数返回
    let kinds = unsafe {
        let kinds = (0..glimmer_outputs_len(outputs))
            .map(|index| glimmer_outputs_kind(outputs, index))
            .collect();
        glimmer_outputs_free(outputs);
        kinds
    };
    (handled == 1, kinds)
}

#[test]
fn typing_shows_preedit_and_candidates_then_space_commits() {
    let session = session();
    let mut handled = 0;
    // SAFETY: 会话有效，handled 可写；outputs 用完即释放
    unsafe {
        glimmer_outputs_free(glimmer_session_focus_in(session, c"e2e".as_ptr()));
        let outputs = glimmer_session_key(session, u32::from('n'), 0, 0, &mut handled);
        assert_eq!(handled, 1);
        assert_eq!(glimmer_session_composing(session), 1);
        let kinds: Vec<c_int> = (0..glimmer_outputs_len(outputs))
            .map(|index| glimmer_outputs_kind(outputs, index))
            .collect();
        assert_eq!(
            kinds,
            vec![
                GLIMMER_OUTPUT_PREEDIT,
                GLIMMER_OUTPUT_AUXILIARY,
                GLIMMER_OUTPUT_CANDIDATES
            ]
        );
        assert_eq!(read(glimmer_outputs_text(outputs, 0)).as_deref(), Some("n"));
        assert_eq!(glimmer_outputs_cursor(outputs, 0), 1);
        assert_eq!(glimmer_outputs_segment_count(outputs, 0), 1);
        assert!(read(glimmer_outputs_text(outputs, 1)).is_none());
        assert_eq!(glimmer_outputs_candidate_count(outputs, 2), 1);
        assert_eq!(
            read(glimmer_outputs_candidate_label(outputs, 2, 0)).as_deref(),
            Some("1")
        );
        assert_eq!(
            read(glimmer_outputs_candidate_text(outputs, 2, 0)).as_deref(),
            Some("N")
        );
        assert_eq!(
            read(glimmer_outputs_comment(outputs, 2, 0)).as_deref(),
            Some("echo")
        );
        assert_eq!(glimmer_outputs_has_next(outputs, 2), 0);
        glimmer_outputs_free(outputs);

        let outputs = glimmer_session_key(session, SPACE, 0, 0, &mut handled);
        assert_eq!(handled, 1);
        assert_eq!(glimmer_outputs_kind(outputs, 0), GLIMMER_OUTPUT_COMMIT);
        assert_eq!(read(glimmer_outputs_text(outputs, 0)).as_deref(), Some("N"));
        assert!(read(glimmer_outputs_text(outputs, 1)).is_none());
        assert_eq!(glimmer_outputs_candidate_count(outputs, 3), 0);
        glimmer_outputs_free(outputs);
        assert_eq!(glimmer_session_composing(session), 0);
        glimmer_session_free(session);
    }
}

#[test]
fn release_is_never_eaten_and_idle_enter_passes() {
    let session = session();
    assert_eq!(key(session, u32::from('a'), true), (false, vec![]));
    assert_eq!(key(session, 0xff0d, false), (false, vec![]));
    // SAFETY: 会话有效
    unsafe { glimmer_session_free(session) };
}

#[test]
fn shift_tap_and_panel_toggle_switch_mode() {
    let session = session();
    assert_eq!(key(session, SHIFT_L, false), (false, vec![]));
    assert_eq!(
        key(session, SHIFT_L, true),
        (false, vec![GLIMMER_OUTPUT_MODE])
    );
    // SAFETY: 会话有效；outputs 用完即释放
    unsafe {
        assert_eq!(glimmer_session_english(session), 1);
        let outputs = glimmer_session_toggle_mode(session);
        assert_eq!(glimmer_outputs_kind(outputs, 0), GLIMMER_OUTPUT_MODE);
        assert_eq!(glimmer_outputs_english(outputs, 0), 0);
        glimmer_outputs_free(outputs);
        assert_eq!(glimmer_session_english(session), 0);
        glimmer_session_free(session);
    }
}

#[test]
fn null_pointers_are_harmless() {
    let null = std::ptr::null_mut();
    let mut handled = 7;
    // SAFETY: 全部是空指针，接口约定判空
    unsafe {
        assert!(glimmer_session_key(null, u32::from('a'), 0, 0, &mut handled).is_null());
        assert_eq!(handled, 0);
        assert!(glimmer_session_key(null, u32::from('a'), 0, 0, null.cast()).is_null());
        assert!(glimmer_session_focus_in(null, std::ptr::null()).is_null());
        assert!(glimmer_session_poll(null).is_null());
        glimmer_session_set_surrounding(null, std::ptr::null(), 0);
        glimmer_session_set_private(null, 1);
        assert_eq!(glimmer_session_composing(null), 0);
        glimmer_session_free(null);
        assert_eq!(glimmer_outputs_len(std::ptr::null()), 0);
        assert_eq!(glimmer_outputs_kind(std::ptr::null(), 0), -1);
        assert!(glimmer_outputs_text(std::ptr::null(), 0).is_null());
        assert!(glimmer_outputs_candidate_text(std::ptr::null(), 0, 0).is_null());
        glimmer_outputs_free(std::ptr::null_mut());
    }
    let session = session();
    // SAFETY: 会话有效，第一个参数之外是空指针；越界下标约定返回缺省值
    unsafe {
        let outputs = glimmer_session_focus_in(session, std::ptr::null());
        assert_eq!(glimmer_outputs_kind(outputs, 99), -1);
        assert!(glimmer_outputs_segment_text(outputs, 0, 99).is_null());
        glimmer_outputs_free(outputs);
        glimmer_session_free(session);
    }
}

#[test]
fn preedit_splits_dimmed_segments_and_counts_cursor_in_bytes() {
    let entry = outputs::Entry::from(Output::Preedit(Some(PreeditView {
        text: "你好ma".to_owned(),
        cursor: 2,
        dimmed: std::iter::once(2..4).collect(),
    })));
    assert_eq!(entry.cursor, 6);
    let segments: Vec<(&str, bool)> = entry
        .segments
        .iter()
        .map(|segment| (segment.text.to_str().unwrap(), segment.dimmed))
        .collect();
    assert_eq!(segments, vec![("你好", false), ("ma", true)]);
}

#[test]
fn candidate_page_flags_and_labels() {
    let rows = (0..10)
        .map(|index: usize| CandidateRow {
            label: if index < 9 {
                (index + 1).to_string()
            } else {
                String::new()
            },
            text: format!("词{index}"),
            annotation: None,
        })
        .collect();
    let entry = outputs::Entry::from(Output::Candidates(Some(CandidateView {
        rows,
        highlight: 3,
        vertical: true,
        page: 1,
        page_count: 3,
    })));
    assert!(entry.has_prev && entry.has_next && entry.vertical);
    assert_eq!(entry.highlight, 3);
    assert_eq!(entry.candidates[8].label.to_str().unwrap(), "9");
    assert_eq!(entry.candidates[9].label.to_str().unwrap(), "");
    assert!(entry.candidates[0].comment.is_none());
}
