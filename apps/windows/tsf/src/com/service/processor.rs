//! `ITfTextInputProcessor`：激活时挂击键 sink、登记翻译 / 英文辅助保留键、连 Server、起轮询定时器、挂 profile /
//! 转换模式回调、登记语言栏按钮；停用按相反顺序撤掉，敲了一半的拼音先原样落定。

use windows::Win32::UI::TextServices::{
    ITfKeyEventSink, ITfKeystrokeMgr, ITfTextInputProcessor_Impl, ITfThreadMgr,
};
use windows::core::{IUnknownImpl, Interface, Ref, Result};

use glimmer_platform::protocol::SessionId;

use super::{ACTIVE, TextService_Impl};
use crate::com::key::preserved;
use crate::com::log::log;
use crate::com::poll::PollTimer;
use crate::com::profile;

impl ITfTextInputProcessor_Impl for TextService_Impl {
    fn Activate(&self, ptim: Ref<ITfThreadMgr>, tid: u32) -> Result<()> {
        let thread_mgr = ptim.ok()?.clone();
        let keystroke: ITfKeystrokeMgr = thread_mgr.cast()?;
        let sink: ITfKeyEventSink = self.to_interface();
        unsafe { keystroke.AdviseKeyEventSink(tid, &sink, true)? };
        self.register_preserved_keys(&keystroke, tid);

        self.client_id.set(tid);
        // 连不上 Server、没定时器都不致命。
        self.connect();
        match PollTimer::new(self.engine.clone(), self.shared.clone()) {
            Ok(timer) => *self.poll_timer.borrow_mut() = Some(timer),
            Err(error) => log(&format!("挂云联想轮询定时器失败: {error}")),
        }

        if self.profile_cookie.get().is_none() {
            match profile::advise(&thread_mgr, SessionId(tid as u64)) {
                Ok(cookie) => self.profile_cookie.set(Some(cookie)),
                Err(error) => log(&format!("监听输入法切换失败: {error}")),
            }
        }
        *self.thread_mgr.borrow_mut() = Some(thread_mgr);
        self.mode_state.set_english(false);
        self.add_lang_bar_item();
        self.refresh_mode_indicator();
        // 放在初始写指示器之后，别被自己那次写触发。
        self.advise_conversion_sink();
        ACTIVE.with(|active| *active.borrow_mut() = Some(self.to_object()));
        log(&format!("微明 TSF 已激活 tid={tid}"));
        Ok(())
    }

    fn Deactivate(&self) -> Result<()> {
        // 先撤回调，之后不再有回调碰本服务。
        ACTIVE.with(|active| active.borrow_mut().take());
        self.unadvise_conversion_sink();
        self.remove_lang_bar_item();
        self.poll_timer.borrow_mut().take();
        // 切走输入法时敲了一半的拼音原样落定，再关会话。
        self.commit_pending();
        if let Some(thread_mgr) = self.thread_mgr.borrow_mut().take()
            && let Ok(keystroke) = thread_mgr.cast::<ITfKeystrokeMgr>()
        {
            if let Some(combo) = self.translate_combo.take() {
                preserved::unregister(&keystroke, &preserved::GUID_TRANSLATE, combo);
            }
            if let Some(combo) = self.english_assist_combo.take() {
                preserved::unregister(&keystroke, &preserved::GUID_ENGLISH_ASSIST, combo);
            }
            let _ = unsafe { keystroke.UnadviseKeyEventSink(self.client_id.get()) };
        }
        if let Some(client) = self.engine.borrow_mut().take() {
            let _ = client.close();
        }
        self.shared.reset();
        self.shared.take_server_stale();
        self.shared.set_foreground(false);
        log("微明 TSF 已停用");
        Ok(())
    }
}

impl TextService_Impl {
    /// 按配置登记两个保留键，登记成功的组合记下来供命中与停用时用；英文辅助无效或冲突（`None`）就不登记。
    fn register_preserved_keys(&self, keystroke: &ITfKeystrokeMgr, tid: u32) {
        let shortcuts = preserved::load_shortcuts();
        let translate = shortcuts.translate_selection;
        match preserved::register(
            keystroke,
            tid,
            &preserved::GUID_TRANSLATE,
            translate,
            "翻译选中文字",
        ) {
            Ok(()) => {
                self.translate_combo.set(Some(translate));
                log(&format!("翻译选中文字快捷键已登记为保留键: {translate}"));
            }
            Err(error) => log(&format!("登记翻译快捷键失败: {error}")),
        }
        let Some(assist) = shortcuts.english_assist_keys() else {
            log("英文辅助快捷键无效或与别的快捷键冲突，不登记");
            return;
        };
        match preserved::register(
            keystroke,
            tid,
            &preserved::GUID_ENGLISH_ASSIST,
            assist,
            "开关英文辅助",
        ) {
            Ok(()) => {
                self.english_assist_combo.set(Some(assist));
                log(&format!("英文辅助快捷键已登记为保留键: {assist}"));
            }
            Err(error) => log(&format!("登记英文辅助快捷键失败: {error}")),
        }
    }
}
