//! `org.freedesktop.IBus.Engine` 的导出对象：D-Bus 方法 → [`Session`]，[`Output`] → D-Bus 信号。
//! 方法、信号与签名对照 ibus `src/ibusengine.c` 的 introspection XML；无参的 `Hide*` / `RequireSurroundingText`
//! 信号不在那段 XML 里，但 `ibus_engine_hide_*` 照发、daemon（`bus/engineproxy.c`）照收。
//!
//! `spawn = false`：方法按到达顺序串行处理，按键不会乱序。信号在方法返回前发出，daemon 先收到上屏再收到「吃掉」的答复。

mod emit;
mod poll;

use std::sync::Arc;
use std::sync::atomic::AtomicBool;

use tokio::sync::Mutex;
use zbus::interface;
use zbus::object_server::SignalEmitter;
use zbus::zvariant::Value;

use crate::frontend::{Output, Session};

/// 一个 IBus 引擎实例。
pub struct Engine {
    /// 前端状态；轮询任务也要用，所以共享。
    session: Arc<Mutex<Session>>,

    /// 轮询任务在跑。
    polling: Arc<AtomicBool>,
}

impl Engine {
    /// 包一个前端状态。
    pub fn new(session: Arc<Mutex<Session>>) -> Self {
        Self {
            session,
            polling: Arc::new(AtomicBool::new(false)),
        }
    }

    /// 在持有会话锁的情况下跑 `handle`，发出它产出的信号，组句中或显示着英文预测时就确保轮询在跑。
    /// 锁一直持有到信号发完，轮询任务的重绘不会插到按键的信号中间。
    async fn run<T>(
        &self,
        emitter: &SignalEmitter<'_>,
        handle: impl FnOnce(&mut Session) -> (T, Vec<Output>),
    ) -> T {
        let mut session = self.session.lock().await;
        let (result, outputs) = handle(&mut session);
        emit::emit(emitter, outputs).await;
        let polling = session.polling();
        drop(session);
        if polling {
            poll::ensure(&self.session, &self.polling, emitter);
        }
        result
    }
}

/// 从 `SetSurroundingText` 的 `IBusText` 里取出文字（结构体第 3 个字段）。
fn text_of(value: &Value<'_>) -> Option<String> {
    match value {
        Value::Value(inner) => text_of(inner),
        Value::Structure(structure) => match structure.fields().get(2) {
            Some(Value::Str(text)) => Some(text.to_string()),
            _ => None,
        },
        _ => None,
    }
}

#[interface(name = "org.freedesktop.IBus.Engine", spawn = false)]
impl Engine {
    /// 按键：返回吃不吃。
    async fn process_key_event(
        &self,
        keyval: u32,
        _keycode: u32,
        state: u32,
        #[zbus(signal_emitter)] emitter: SignalEmitter<'_>,
    ) -> bool {
        self.run(&emitter, |session| session.process_key(keyval, state))
            .await
    }

    /// 应用光标的屏幕矩形。
    async fn set_cursor_location(&self, x: i32, y: i32, w: i32, h: i32) {
        self.session.lock().await.set_cursor_location(x, y, w, h);
    }

    /// 客户端能力位（preedit / 候选表 / surrounding text 等）；我们总按全能力发信号，daemon 自会转给面板。
    async fn set_capabilities(&self, caps: u32) {
        tracing::debug!(caps, "客户端能力");
    }

    /// 点了面板上的属性。
    async fn property_activate(
        &self,
        name: String,
        _state: u32,
        #[zbus(signal_emitter)] emitter: SignalEmitter<'_>,
    ) {
        self.run(&emitter, |session| ((), session.property_activate(&name)))
            .await;
    }

    /// 面板显示属性：没有子菜单，无事可做。
    async fn property_show(&self, _name: String) {}

    /// 面板隐藏属性：同上。
    async fn property_hide(&self, _name: String) {}

    /// 鼠标点了候选。
    async fn candidate_clicked(
        &self,
        index: u32,
        _button: u32,
        _state: u32,
        #[zbus(signal_emitter)] emitter: SignalEmitter<'_>,
    ) {
        self.run(&emitter, |session| ((), session.candidate_clicked(index)))
            .await;
    }

    /// 老式获焦（引擎不报 `FocusId` 时 daemon 调这个）。
    async fn focus_in(&self, #[zbus(signal_emitter)] emitter: SignalEmitter<'_>) {
        self.run(&emitter, |session| ((), session.focus_in(None)))
            .await;
        emit::require_surrounding_text(&emitter).await;
    }

    /// 带输入上下文与客户端名的获焦（ibus ≥ 1.5.28，引擎的 `FocusId` 属性为真时 daemon 调这个）。
    async fn focus_in_id(
        &self,
        _object_path: String,
        client: String,
        #[zbus(signal_emitter)] emitter: SignalEmitter<'_>,
    ) {
        let app = (!client.is_empty()).then_some(client);
        self.run(&emitter, |session| ((), session.focus_in(app)))
            .await;
        emit::require_surrounding_text(&emitter).await;
    }

    /// 老式失焦。
    async fn focus_out(&self, #[zbus(signal_emitter)] emitter: SignalEmitter<'_>) {
        self.run(&emitter, |session| ((), session.focus_out()))
            .await;
    }

    /// 带输入上下文的失焦。
    async fn focus_out_id(
        &self,
        _object_path: String,
        #[zbus(signal_emitter)] emitter: SignalEmitter<'_>,
    ) {
        self.run(&emitter, |session| ((), session.focus_out()))
            .await;
    }

    /// 应用要求重置。
    async fn reset(&self, #[zbus(signal_emitter)] emitter: SignalEmitter<'_>) {
        self.run(&emitter, |session| ((), session.reset())).await;
    }

    /// 切到本输入法。
    async fn enable(&self, #[zbus(signal_emitter)] emitter: SignalEmitter<'_>) {
        self.run(&emitter, |session| ((), session.enable())).await;
        emit::require_surrounding_text(&emitter).await;
    }

    /// 切走本输入法。
    async fn disable(&self, #[zbus(signal_emitter)] emitter: SignalEmitter<'_>) {
        self.run(&emitter, |session| ((), session.disable())).await;
    }

    /// 面板上一页。
    async fn page_up(&self, #[zbus(signal_emitter)] emitter: SignalEmitter<'_>) {
        self.run(&emitter, |session| ((), session.page(false)))
            .await;
    }

    /// 面板下一页。
    async fn page_down(&self, #[zbus(signal_emitter)] emitter: SignalEmitter<'_>) {
        self.run(&emitter, |session| ((), session.page(true))).await;
    }

    /// 面板上移高亮。
    async fn cursor_up(&self, #[zbus(signal_emitter)] emitter: SignalEmitter<'_>) {
        self.run(&emitter, |session| ((), session.move_cursor(false)))
            .await;
    }

    /// 面板下移高亮。
    async fn cursor_down(&self, #[zbus(signal_emitter)] emitter: SignalEmitter<'_>) {
        self.run(&emitter, |session| ((), session.move_cursor(true)))
            .await;
    }

    /// 应用报来光标周围的文字（`IBusText`）与光标、选区锚点的字符下标。
    async fn set_surrounding_text(&self, text: Value<'_>, cursor_pos: u32, _anchor_pos: u32) {
        let Some(text) = text_of(&text) else {
            tracing::warn!("SetSurroundingText 的参数不是 IBusText");
            return;
        };
        self.session
            .lock()
            .await
            .set_surrounding_text(&text, cursor_pos);
    }

    /// 手写输入：不支持。
    async fn process_hand_writing_event(&self, _coordinates: Vec<f64>) {}

    /// 取消手写：不支持。
    async fn cancel_hand_writing(&self, _n_strokes: u32) {}

    /// 面板扩展（emoji 选择器等）的事件：不参与。
    async fn panel_extension_received(&self, _event: Value<'_>) {}

    /// 面板扩展登记快捷键：不参与。
    async fn panel_extension_register_keys(&self, _data: Value<'_>) {}

    /// 输入框的用途与提示，daemon 经 `org.freedesktop.DBus.Properties.Set` 写入。
    #[zbus(property)]
    async fn set_content_type(&self, value: (u32, u32)) {
        self.session.lock().await.set_content_type(value.0, value.1);
    }

    /// 支持 `FocusInId` / `FocusOutId`。daemon 读的是裸 `b`（`g_variant_get_boolean`），不是 XML 里写的 `(b)`。
    #[zbus(property(emits_changed_signal = "const"))]
    async fn focus_id(&self) -> bool {
        true
    }

    /// 要用 surrounding text（前文）：daemon 据此在获焦时向客户端要。
    #[zbus(property(emits_changed_signal = "const"))]
    async fn active_surrounding_text(&self) -> bool {
        true
    }

    /// 上屏。
    #[zbus(signal)]
    async fn commit_text(emitter: &SignalEmitter<'_>, text: Value<'_>) -> zbus::Result<()>;

    /// 更新 preedit；`mode` 是 `IBusPreeditFocusMode`。
    #[zbus(signal)]
    async fn update_preedit_text(
        emitter: &SignalEmitter<'_>,
        text: Value<'_>,
        cursor_pos: u32,
        visible: bool,
        mode: u32,
    ) -> zbus::Result<()>;

    /// 收起 preedit。
    #[zbus(signal)]
    async fn hide_preedit_text(emitter: &SignalEmitter<'_>) -> zbus::Result<()>;

    /// 更新辅助文字。
    #[zbus(signal)]
    async fn update_auxiliary_text(
        emitter: &SignalEmitter<'_>,
        text: Value<'_>,
        visible: bool,
    ) -> zbus::Result<()>;

    /// 收起辅助文字。
    #[zbus(signal)]
    async fn hide_auxiliary_text(emitter: &SignalEmitter<'_>) -> zbus::Result<()>;

    /// 更新候选表。
    #[zbus(signal)]
    async fn update_lookup_table(
        emitter: &SignalEmitter<'_>,
        table: Value<'_>,
        visible: bool,
    ) -> zbus::Result<()>;

    /// 收起候选表。
    #[zbus(signal)]
    async fn hide_lookup_table(emitter: &SignalEmitter<'_>) -> zbus::Result<()>;

    /// 登记属性表（`IBusPropList`）。
    #[zbus(signal)]
    async fn register_properties(emitter: &SignalEmitter<'_>, props: Value<'_>)
    -> zbus::Result<()>;

    /// 更新一个属性（`IBusProperty`）。
    #[zbus(signal)]
    async fn update_property(emitter: &SignalEmitter<'_>, prop: Value<'_>) -> zbus::Result<()>;

    /// 把按键转回给应用。当前没有用到（不吃的键直接返回 `false`），留着对齐接口。
    #[zbus(signal)]
    async fn forward_key_event(
        emitter: &SignalEmitter<'_>,
        keyval: u32,
        keycode: u32,
        state: u32,
    ) -> zbus::Result<()>;

    /// 请客户端开始上报 surrounding text。
    #[zbus(signal)]
    async fn require_surrounding_text(emitter: &SignalEmitter<'_>) -> zbus::Result<()>;
}
