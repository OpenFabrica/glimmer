//! 把前端的 [`Output`] 逐条发成 IBus 信号。发送失败只记日志：连接断了主循环自会退出，这里不必中断剩下的信号。

use zbus::object_server::SignalEmitter;

use super::Engine;
use crate::frontend::Output;
use crate::frontend::view::PreeditView;
use crate::ibus::variant::{
    Attribute, assist_property, lookup_table, mode_property, property_list, text,
};

/// `IBUS_ENGINE_PREEDIT_CLEAR`：收起 preedit 时用，没有东西要落定。
const PREEDIT_CLEAR: u32 = 0;

/// `IBUS_ENGINE_PREEDIT_COMMIT`：失焦 / 重置 / 切引擎时 daemon 或客户端把屏上的 preedit 落进应用
/// （引擎在 `FocusOut` 里发 `CommitText` 会落空，见 `frontend::session` 的 focus 子模块）。
const PREEDIT_COMMIT: u32 = 1;

/// preedit 里不参与候选那段拼音的灰色。
const DIMMED_RGB: u32 = 0x80_80_80;

/// 按顺序发出。
pub(super) async fn emit(emitter: &SignalEmitter<'_>, outputs: Vec<Output>) {
    for output in outputs {
        if let Err(error) = emit_one(emitter, output).await {
            tracing::warn!(%error, "发 IBus 信号失败");
        }
    }
}

/// 请客户端上报前文。
pub(super) async fn require_surrounding_text(emitter: &SignalEmitter<'_>) {
    if let Err(error) = Engine::require_surrounding_text(emitter).await {
        tracing::warn!(%error, "发 RequireSurroundingText 失败");
    }
}

/// 一条指令对应一个信号。preedit 收起发「空文本、不可见」的 `UpdatePreeditText`，客户端一律会清掉旧的 preedit。
async fn emit_one(emitter: &SignalEmitter<'_>, output: Output) -> zbus::Result<()> {
    match output {
        Output::Commit(content) => Engine::commit_text(emitter, text(&content, &[])).await,
        Output::Preedit(Some(view)) => {
            let cursor = view.cursor as u32;
            Engine::update_preedit_text(emitter, preedit(&view), cursor, true, PREEDIT_COMMIT).await
        }
        Output::Preedit(None) => {
            Engine::update_preedit_text(emitter, text("", &[]), 0, false, PREEDIT_CLEAR).await
        }
        Output::Auxiliary(Some(content)) => {
            Engine::update_auxiliary_text(emitter, text(&content, &[]), true).await
        }
        Output::Auxiliary(None) => Engine::hide_auxiliary_text(emitter).await,
        Output::Candidates(Some(view)) => {
            Engine::update_lookup_table(emitter, lookup_table(&view), true).await
        }
        Output::Candidates(None) => Engine::hide_lookup_table(emitter).await,
        Output::RegisterProperties { english, assist } => {
            let properties = vec![mode_property(english, assist), assist_property(assist)];
            Engine::register_properties(emitter, property_list(properties)).await
        }
        // 两个属性都刷：英文辅助可能在别处刚切过，中英切换时也要对上
        Output::Mode { english, assist } => {
            Engine::update_property(emitter, mode_property(english, assist)).await?;
            Engine::update_property(emitter, assist_property(assist)).await
        }
    }
}

/// preedit 整行单下划线，不参与候选的那段画灰。
fn preedit(view: &PreeditView) -> zbus::zvariant::Value<'static> {
    let mut attributes = vec![Attribute::underline(0, view.text.chars().count())];
    attributes.extend(
        view.dimmed
            .iter()
            .map(|range| Attribute::foreground(DIMMED_RGB, range.start, range.end)),
    );
    text(&view.text, &attributes)
}
