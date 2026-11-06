//! Fcitx5 插件的 Rust 半边：把 `glimmer-linux` 的前端会话（[`glimmer_linux::frontend::Session`]）与后端包成 C 接口，
//! 编成静态库链进 C++ 插件 `glimmer.so`。C++ 那边只管 Fcitx5 的对象（输入上下文、面板、定时器），不碰 Rust 类型。
//!
//! 接口声明在 `include/glimmer_fcitx5.h`，与这里的 `extern "C"` 函数一一对应。约定：
//! - 所有函数都 `catch_unwind`，panic 不跨过 FFI 边界，出错返回空指针 / 0 / -1；
//! - 指针参数一律判空；
//! - 事件函数返回的 [`GlimmerOutputs`] 由调用方 `glimmer_outputs_free`，里面的字符串活到那时为止。

mod global;
mod outputs;
mod session;

use std::panic::{AssertUnwindSafe, catch_unwind};

pub use global::{glimmer_backend_init, glimmer_shutdown, glimmer_tick};
pub use outputs::{
    GLIMMER_OUTPUT_AUXILIARY, GLIMMER_OUTPUT_CANDIDATES, GLIMMER_OUTPUT_COMMIT,
    GLIMMER_OUTPUT_MODE, GLIMMER_OUTPUT_PREEDIT, GlimmerOutputs, glimmer_outputs_candidate_count,
    glimmer_outputs_candidate_label, glimmer_outputs_candidate_text, glimmer_outputs_comment,
    glimmer_outputs_cursor, glimmer_outputs_english, glimmer_outputs_free,
    glimmer_outputs_has_next, glimmer_outputs_has_prev, glimmer_outputs_highlight,
    glimmer_outputs_kind, glimmer_outputs_len, glimmer_outputs_segment_count,
    glimmer_outputs_segment_dimmed, glimmer_outputs_segment_text, glimmer_outputs_text,
    glimmer_outputs_vertical,
};
pub use session::{
    GlimmerSession, glimmer_session_composing, glimmer_session_english, glimmer_session_focus_in,
    glimmer_session_focus_out, glimmer_session_free, glimmer_session_key,
    glimmer_session_move_cursor, glimmer_session_new, glimmer_session_page, glimmer_session_poll,
    glimmer_session_polling, glimmer_session_reset, glimmer_session_select,
    glimmer_session_set_cursor_rect, glimmer_session_set_private, glimmer_session_set_surrounding,
    glimmer_session_toggle_mode,
};

/// 跑一段可能 panic 的代码；panic 了记日志并返回 `fallback`，不让 unwind 穿过 C 调用方。
fn guard<T>(what: &str, fallback: T, body: impl FnOnce() -> T) -> T {
    match catch_unwind(AssertUnwindSafe(body)) {
        Ok(value) => value,
        Err(_) => {
            tracing::error!(what, "C 接口里 panic 了，按失败返回");
            fallback
        }
    }
}

#[cfg(test)]
mod tests;
