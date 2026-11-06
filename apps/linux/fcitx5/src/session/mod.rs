//! 一个 Fcitx5 输入上下文对应的会话（[`GlimmerSession`]）：包着 `glimmer-linux` 的前端状态机，生命周期由 C++ 管。
//! 创建 / 释放 / 查询在这里，事件函数在 `event`。

mod event;

use std::ffi::c_int;

use glimmer_linux::frontend::{Output, Session};

pub use event::{
    glimmer_session_focus_in, glimmer_session_focus_out, glimmer_session_key,
    glimmer_session_move_cursor, glimmer_session_page, glimmer_session_poll, glimmer_session_reset,
    glimmer_session_select, glimmer_session_set_cursor_rect, glimmer_session_set_private,
    glimmer_session_set_surrounding, glimmer_session_toggle_mode,
};

use crate::global::backend;
use crate::guard;
use crate::outputs::GlimmerOutputs;

/// C 侧看到的不透明会话。
pub struct GlimmerSession {
    /// 前端状态机（与 IBus 引擎共用的那一份实现）。
    session: Session,
}

/// 在会话上跑一段代码；空指针或 panic 返回 `fallback`。
///
/// # Safety
/// `session` 为空或是 `glimmer_session_new` 返回、尚未释放的指针，且没有别的线程同时在用。
unsafe fn with<T>(
    what: &str,
    session: *mut GlimmerSession,
    fallback: T,
    body: impl FnOnce(&mut Session) -> T,
) -> T {
    // SAFETY: 见函数的 Safety 约定
    let Some(session) = (unsafe { session.as_mut() }) else {
        return fallback;
    };
    guard(what, fallback, || body(&mut session.session))
}

/// 在会话上跑一个事件并把指令装箱；空指针或 panic 返回空指针。
///
/// # Safety
/// 同 [`with`]。
unsafe fn run(
    what: &str,
    session: *mut GlimmerSession,
    handle: impl FnOnce(&mut Session) -> Vec<Output>,
) -> *mut GlimmerOutputs {
    // SAFETY: 见函数的 Safety 约定
    unsafe {
        with(what, session, std::ptr::null_mut(), |session| {
            GlimmerOutputs::boxed(handle(session))
        })
    }
}

/// 开一个会话（中文模式、还没获焦）；后端没初始化返回空指针。
#[unsafe(no_mangle)]
pub extern "C" fn glimmer_session_new() -> *mut GlimmerSession {
    guard("glimmer_session_new", std::ptr::null_mut(), || {
        let Some(backend) = backend() else {
            tracing::error!("glimmer_backend_init 之前就建会话");
            return std::ptr::null_mut();
        };
        Box::into_raw(Box::new(GlimmerSession {
            session: Session::new(backend),
        }))
    })
}

/// 关掉会话（通知 Router 关协议会话）并释放；空指针无事发生。
///
/// # Safety
/// `session` 为空或是 `glimmer_session_new` 返回、尚未释放的指针；释放后不能再用。
#[unsafe(no_mangle)]
pub unsafe extern "C" fn glimmer_session_free(session: *mut GlimmerSession) {
    if session.is_null() {
        return;
    }
    guard("glimmer_session_free", (), || {
        // SAFETY: 指针来自 `glimmer_session_new` 的 `Box::into_raw`，调用方保证只释放一次
        let mut session = unsafe { Box::from_raw(session) };
        session.session.close();
    });
}

/// 正在组句（1 / 0，不含英文预测）：C++ 据此决定按键前要不要同步一次输入框状态（光标矩形、前文、私密）。
///
/// # Safety
/// `session` 为空或是 `glimmer_session_new` 返回、尚未释放的指针。
#[unsafe(no_mangle)]
pub unsafe extern "C" fn glimmer_session_composing(session: *const GlimmerSession) -> c_int {
    guard("glimmer_session_composing", 0, || {
        // SAFETY: 见函数的 Safety 约定
        c_int::from(unsafe { session.as_ref() }.is_some_and(|s| s.session.composing()))
    })
}

/// 要不要开着轮询（1 / 0）：组句中，或显示着英文下一词预测（过期要靠轮询收起）。C++ 据此开关 60 ms 的轮询定时器。
///
/// # Safety
/// `session` 为空或是 `glimmer_session_new` 返回、尚未释放的指针。
#[unsafe(no_mangle)]
pub unsafe extern "C" fn glimmer_session_polling(session: *const GlimmerSession) -> c_int {
    guard("glimmer_session_polling", 0, || {
        // SAFETY: 见函数的 Safety 约定
        c_int::from(unsafe { session.as_ref() }.is_some_and(|s| s.session.polling()))
    })
}

/// 当前是英文模式（1 / 0）：状态区与子模式标签据此显示「中 / 英」。
///
/// # Safety
/// `session` 为空或是 `glimmer_session_new` 返回、尚未释放的指针。
#[unsafe(no_mangle)]
pub unsafe extern "C" fn glimmer_session_english(session: *const GlimmerSession) -> c_int {
    guard("glimmer_session_english", 0, || {
        // SAFETY: 见函数的 Safety 约定
        c_int::from(unsafe { session.as_ref() }.is_some_and(|s| s.session.english()))
    })
}
