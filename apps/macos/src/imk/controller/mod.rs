//! IMK 输入控制器：每个输入会话（每个应用的文本框）一个实例。
//!
//! 只做两件事：把按键翻译成 Engine 的调用，把 Engine 返回的候选交给候选窗口。
//! **这里不允许出现排序、词库或翻译逻辑。** 会话状态（候选、高亮、页码）在 [`crate::host::Session`]。

use objc2::rc::{Allocated, Retained};
use objc2::runtime::{AnyObject, Sel};
use std::cell::RefCell;

use glimmer_core::{Candidate, QUESTION_PREFIX};
use objc2::{DefinedClass, define_class, msg_send, sel};
use objc2_app_kit::{NSEvent, NSEventMask, NSMenu};
use objc2_foundation::NSObjectProtocol;
use objc2_input_method_kit::{IMKInputController, IMKServer};

use super::{ShiftTap, TextClient, catch_panic, modifiers, recover_from_panic, secure_input};
use crate::candidates::Preedit;
use crate::host;
use crate::menubar;

mod commit;
mod english_assist;
mod event;
mod next_word;
mod translation;

define_class!(
    // SAFETY:
    // - IMKInputController 允许子类化，Apple 文档的标准用法就是继承它。
    // - 没有实现 Drop。
    #[unsafe(super(IMKInputController))]
    // 名字要和 Info.plist 的 InputMethodServerControllerClass 一致
    #[name = "GlimmerInputController"]
    #[ivars = RefCell<ShiftTap>]
    pub struct GlimmerInputController;

    impl GlimmerInputController {
        /// Shift 通过 flagsChanged 送达。扩展事件掩码后需显式接管鼠标收尾，避免组句残留。
        #[unsafe(method(recognizedEvents:))]
        fn recognized_events(&self, _sender: Option<&AnyObject>) -> usize {
            (NSEventMask::KeyDown | NSEventMask::FlagsChanged
                | NSEventMask::LeftMouseDown | NSEventMask::RightMouseDown
                | NSEventMask::OtherMouseDown).0 as usize
        }

        /// IMKServer 为每个新会话调用的指定初始化方法，在这里放好 ivars。
        #[unsafe(method_id(initWithServer:delegate:client:))]
        fn init_with_server(
            this: Allocated<Self>,
            server: Option<&IMKServer>,
            delegate: Option<&AnyObject>,
            client: Option<&AnyObject>,
        ) -> Option<Retained<Self>> {
            tracing::info!("新建输入会话");
            let this = this.set_ivars(RefCell::new(ShiftTap::default()));
            unsafe { msg_send![super(this), initWithServer: server, delegate: delegate, client: client] }
        }

        /// 所有按键事件都到这里（IMK 第一层协议）。IMK 按控制器实现了哪一层决定路线，实现了这个方法就不会再分发成
        /// `inputText:client:` / `didCommandBySelector:client:`（父类缺省实现也不分发），所以自己分：
        /// Option+数字上屏候选的译文（Option 会把数字键变成 ¡™£ 这类字符，只能按键码认）；命令键按键码映射成原来的选择器；
        /// 其余按事件带的字符走文本路径。返回 true 表示已处理，系统不再把按键交给应用。
        // define_class! 会把返回类型转成 ObjC BOOL，方法体里不能用 `return`，逻辑放在下面的 inherent impl
        #[unsafe(method(handleEvent:client:))]
        fn handle_event(&self, event: Option<&NSEvent>, client: Option<&AnyObject>) -> bool {
            match (event, client) {
                (Some(event), Some(client)) => {
                    let client = TextClient::new(client);
                    // panic 拦下后把缓冲区原样上屏，这个按键交还给应用
                    catch_panic("handleEvent", || self.dispatch_event(event, client))
                        .unwrap_or_else(|| {
                            recover_from_panic(Some(client));
                            false
                        })
                }
                _ => false,
            }
        }

        /// 应用要求立刻结束本次输入（切换焦点、切换输入法等）。
        #[unsafe(method(commitComposition:))]
        fn commit_composition(&self, client: Option<&AnyObject>) {
            self.ivars().borrow_mut().reset();
            let client = client.map(TextClient::new);
            let done = catch_panic("commitComposition", || {
                if let Some(client) = client {
                    self.commit_raw(client);
                }
                host::with(|h| {
                    h.cancel_prediction();
                    h.dismiss_next_word("commitComposition");
                    h.window.hide();
                });
            });
            if done.is_none() {
                recover_from_panic(client);
            }
        }

        #[unsafe(method(activateServer:))]
        fn activate_server(&self, sender: Option<&AnyObject>) {
            self.ivars().borrow_mut().reset();
            tracing::info!("activateServer");
            let done = catch_panic("activateServer", || {
                // 用户要往 [apps] 里加应用时，从这条日志抄 bundle identifier
                let bundle = sender.and_then(|s| TextClient::new(s).bundle_identifier());
                if let Some(bundle) = &bundle {
                    tracing::debug!(%bundle, "当前应用");
                }
                let me = self.address();
                let client = sender.map(objc2::Message::retain);
                host::with(|h| {
                    // 换了会话（切应用、换文本框）：上一处的预测不能挂到这里来
                    h.dismiss_next_word("activateServer");
                    h.active_controller = Some(me);
                    h.active_client = client;
                    h.engine.set_application(bundle);
                    // 换了应用：`[apps] english_candidates_off` 里的应用英文辅助不生效，「英 / 英+」跟着它
                    h.sync_english_assist_indicator();
                    h.refresh_text_replacements();
                    h.reload_config_if_changed();
                    h.indicator.activate();
                    h.watch.start();
                });
            });
            if done.is_none() {
                recover_from_panic(None);
            }
        }

        /// 系统输入源菜单（菜单栏旗帜图标）每次展开前来取输入法自己的条目。
        #[unsafe(method_id(menu))]
        fn menu(&self) -> Option<Retained<NSMenu>> {
            let menu = host::with(|h| h.menu.ns_menu());
            tracing::debug!(
                items = menu.as_ref().map_or(0, |m| m.numberOfItems()),
                "输入源菜单来取条目"
            );
            menu
        }

        /// 输入源菜单里点了条目：IMK 转发到控制器，sender 是带 IMKCommandMenuItem 的字典。
        #[unsafe(method(menuAction:))]
        fn menu_action(&self, sender: Option<&AnyObject>) {
            if let Some(action) = menubar::action_from_sender(sender) {
                host::with(|h| h.perform(action));
            }
        }

        /// 只有当前激活的会话停用才拆全局状态。同一应用里换焦点（浏览器换文本框）时 IMK 先 activate 新会话、
        /// 后 deactivate 旧会话，旧会话若也走完整收尾，会把新会话正在敲的拼音原样上屏、收掉状态项、停掉配置监视。
        #[unsafe(method(deactivateServer:))]
        fn deactivate_server(&self, sender: Option<&AnyObject>) {
            self.ivars().borrow_mut().reset();
            let me = self.address();
            let current = host::with(|h| {
                if h.active_controller == Some(me) {
                    h.active_controller = None;
                    h.active_client = None;
                    true
                } else {
                    false
                }
            })
            .unwrap_or(true);
            if !current {
                tracing::info!("deactivateServer（旧会话，跳过收尾）");
                return;
            }
            tracing::info!("deactivateServer");
            let client = sender.map(TextClient::new);
            let done = catch_panic("deactivateServer", || {
                if let Some(client) = client {
                    self.commit_raw(client);
                }
                // 切换输入源时无论如何都收掉候选框，不能留一个孤儿窗口在屏幕上
                host::with(|h| {
                    h.cancel_prediction();
                    h.dismiss_next_word("deactivateServer");
                    h.window.hide();
                    h.indicator.deactivate();
                    h.watch.stop();
                    h.engine.break_chain();
                    h.engine.flush_learning();
                    h.last_flush = std::time::Instant::now();
                });
            });
            if done.is_none() {
                recover_from_panic(client);
                // 善后里没做的收尾：学习数据还是要落盘
                host::with(|h| {
                    h.indicator.deactivate();
                    h.watch.stop();
                    h.engine.flush_learning();
                });
            }
        }
    }

    unsafe impl NSObjectProtocol for GlimmerInputController {}
);

/// 给本地整句模型看的光标前文最多读多少字符（Engine 自己再按它的前文长度截）。
const RESCORE_LOOKBACK: usize = glimmer_core::RESCORE_CONTEXT_CHARS;

/// 数字行与小键盘的键码对应的数字 1–9（ANSI 布局的物理键）。
fn digit_key(key_code: u16) -> Option<usize> {
    Some(match key_code {
        18 | 83 => 1,
        19 | 84 => 2,
        20 | 85 => 3,
        21 | 86 => 4,
        23 | 87 => 5,
        22 | 88 => 6,
        26 | 89 => 7,
        28 | 91 => 8,
        25 | 92 => 9,
        _ => return None,
    })
}

impl GlimmerInputController {
    /// 控制器对象地址，只用来在 Host 里比对「谁是当前激活的会话」，不解引用。
    fn address(&self) -> usize {
        self as *const Self as usize
    }

    /// 修饰键 + 数字（缺省 ⇧）：删掉当前页第几个候选。不在组句中不管；那格没有候选就吞掉按键不动。
    /// 删完重新查一遍（排序会变），结果那句话显示在拼音行右侧，敲下一键就没了。
    fn handle_delete_key(&self, digit: usize, client: TextClient<'_>) -> bool {
        let composing = host::with(|h| !h.engine.composition().is_empty()).unwrap_or(false);
        if !composing {
            return false;
        }
        let Some(message) = host::with(|h| h.forget_candidate(digit - 1)).flatten() else {
            tracing::debug!(digit, "这一格没有候选，没什么可删");
            return true;
        };
        tracing::info!(%message);
        self.refresh(client);
        host::with(|h| h.status = Some(message));
        self.render(client);
        true
    }

    fn handle_text(&self, text: &str, shift: bool, client: TextClient<'_>) -> bool {
        tracing::debug!(%text, "inputText");
        self.note_application(&client);
        let mut composing = host::with(|h| !h.engine.composition().is_empty()).unwrap_or(false);
        let english = modifiers::english_mode();
        // 终端、编辑器这类应用（`[apps] english_candidates_off`）里英文模式是纯直通
        let english_candidates = english
            && host::with(|h| h.english_candidates_in(client.bundle_identifier().as_deref()))
                .unwrap_or(false);
        // 英文模式结束（或候选开关关了）：敲的字母先原样上屏，别把它们当拼音
        if composing
            && !english_candidates
            && host::with(|h| h.engine.english_mode()).unwrap_or(false)
        {
            self.commit_raw(client);
            composing = false;
        }
        let [byte] = text.as_bytes() else {
            // 多字符文本（如输入法联动、粘贴）：先把当前候选（英文模式下是敲的字母）上屏，再交给应用
            if composing {
                if host::with(|h| h.engine.english_mode()).unwrap_or(false) {
                    self.commit_raw(client);
                } else {
                    self.commit_highlighted(client);
                }
            }
            return false;
        };
        // Caps Lock 亮着也能用组合键切回中文；去掉它造成的大写，保留 Shift 临时大写。
        let c = if !english && modifiers::caps_lock_on() && !shift {
            char::from(*byte).to_ascii_lowercase()
        } else {
            char::from(*byte)
        };
        host::with(|h| h.indicator.update());
        // 缓冲区为空时敲 ? 先进问字模式（配置 `[shortcut] question_mark`，缺省关），中英文模式都行：
        // 后面跟字母就是在问字，跟别的键就还原成问号
        if !composing
            && c == QUESTION_PREFIX
            && host::with(|h| h.engine.takes_question_mark()).unwrap_or(false)
        {
            host::with(|h| h.engine.push(c));
            self.refresh(client);
            return true;
        }
        // 双拼下 Shift+V / Shift+U 进表达式 / 问字模式（全拼下的 v / u 被音节占了）
        if !composing && !english && host::with(|h| h.engine.takes_mode_letter(c)).unwrap_or(false)
        {
            host::with(|h| h.engine.push(c));
            self.refresh(client);
            return true;
        }
        let question = composing && host::with(|h| h.engine.question_mode()).unwrap_or(false);
        // 英文模式下问字：Caps Lock 让字母以大写送来，按小写收进问题
        let c = if question && english && c.is_ascii_uppercase() {
            c.to_ascii_lowercase()
        } else {
            c
        };
        host::with(|h| h.engine.set_english_mode(english_candidates && !question));
        let (page_previous, page_next) =
            host::with(|h| h.page_keys).unwrap_or(glimmer_platform::DEFAULT_PAGE_KEYS);
        // Caps Lock 亮着 = 英文模式：不组句、不转标点，字母默认小写、按住 Shift 才大写
        if english && !question {
            // 使用事件里的 Shift，避免处理延迟期间用户松键造成大小写错误。
            let letter = if shift {
                c.to_ascii_uppercase()
            } else {
                c.to_ascii_lowercase()
            };
            if !english_candidates {
                if composing {
                    self.commit_raw(client);
                }
                if c.is_ascii_alphabetic() {
                    client.insert_text(&letter.to_string());
                    host::with(|h| h.engine.note_passthrough(letter));
                    return true;
                }
                host::with(|h| h.engine.note_passthrough(c));
                return false;
            }
            // 英文候选：字母（以及组词中的 _ ' -）进缓冲区，候选来自英文词表。选词与中文模式一样：
            // 空格选高亮（词上屏后空格照样交给应用，接着打下一个词）、数字选当前页第 N 个、翻页键翻页；
            // 数字对应的格子没有候选（kubectl 这类词表没有的词、候选不足 N 个）时是标识符的一部分（foo1）。
            // 回车、标点先把敲的字母原样上屏再交给应用
            if composing
                && let Some(offset) = c.to_digit(10).filter(|d| *d > 0)
                && let Some(index) =
                    host::with(|h| h.session.index_on_page(offset as usize - 1)).flatten()
            {
                return self.commit_index(index, client);
            }
            if c.is_ascii_alphabetic()
                || (composing && (c.is_ascii_digit() || matches!(c, '_' | '\'' | '-')))
            {
                host::with(|h| h.engine.push(letter));
                self.refresh(client);
                return true;
            }
            if composing && c == page_previous {
                return self.turn_page(-1, client);
            }
            if composing && c == page_next {
                return self.turn_page(1, client);
            }
            if composing {
                if c == ' ' {
                    self.commit_highlighted(client);
                } else {
                    self.commit_raw(client);
                }
            }
            host::with(|h| {
                h.engine.note_passthrough(c);
                // 词后直通的空格（组句中空格选词之后、空组句时直接敲的空格）：问 Core 有没有下一词预测。
                // 空格要等这里返回 false 才进应用，所以不在这里取插入点画，交给定时器下一拍。
                // 预测在显示（或等着画）时敲的空格是「不要它」：空格照常交给应用，但不再取，免得同一个词又弹出来
                if c == ' ' && !h.next_word_declined() {
                    h.request_next_word();
                }
            });
            return false;
        }
        // 表达式模式（v 开头）：数字与运算符进缓冲区，不当选词 / 翻页键
        let expression = composing && host::with(|h| h.engine.expression_mode()).unwrap_or(false);
        // 英文直输段（缓冲区里已有 `-` 这类字符）：可见字符一律追加，空格 / 回车整段原样上屏
        let raw = composing && host::with(|h| h.engine.raw_mode()).unwrap_or(false);
        // 问字模式下敲的还可能是码点（`u4e00`、`u+1f600`）：数字与 `+` 进缓冲区而不是选词
        let unicode = question && host::with(|h| h.engine.unicode_entry()).unwrap_or(false);
        // 微软 / 搜狗双拼的 `;` 是 ing 键：末尾有落单声母时进缓冲区，其他时候还是标点
        let semicolon =
            composing && c == ';' && host::with(|h| h.engine.takes_semicolon()).unwrap_or(false);
        // 组句中敲半角标点（含 `-`）：进不进缓冲区由 Core 按 `[general] punctuation_mode` 定——进了整段成为
        // 英文直输段（`hello,` `no-way`），不进就先把高亮候选上屏再当普通标点处理。翻页键除外（`-` 只在 `[general] page_keys` 选 `-=` 时是翻页键）；
        // ⇧+数字（! @ # …）在前面已被删候选 / 译词键截走
        let punctuation = composing
            && !question
            && !expression
            && c.is_ascii_punctuation()
            && c != page_previous
            && c != page_next
            && host::with(|h| h.engine.takes_punctuation()).unwrap_or(true);
        // 大千注音：数字键与 `- ; , . /` 都分给了注音符号与声调，一律进缓冲区，不再选词 / 翻页 / 转标点，
        // 选词改用 Enter（Windows 壳同样的取舍，见 `dispatch_event`）；没在组句时敲也照样起一段
        let zhuyin_key = host::with(|h| h.engine.is_zhuyin_mode()).unwrap_or(false)
            && (c.is_ascii_digit() || matches!(c, '-' | ';' | ',' | '.' | '/'));
        // 注音这一段还差声调时，空格是一声 / 轻声，进缓冲区而不是上屏候选
        let zhuyin_tone =
            composing && c == ' ' && host::with(|h| h.engine.zhuyin_needs_tone()).unwrap_or(false);
        if c.is_ascii_lowercase()
            || (composing && c == '\'')
            || semicolon
            || zhuyin_key
            || zhuyin_tone
            || (expression && glimmer_core::shortcut::is_expression_char(c))
            || (raw && c.is_ascii_graphic())
            || (unicode && (c.is_ascii_digit() || c == '+'))
            || punctuation
        {
            self.push_and_refresh(c, client);
            return true;
        }
        // 直输段里的空格：整段原样上屏，空格本身也交给应用（`hello, world` 里的空格要在）
        if raw && c == ' ' {
            self.commit_highlighted(client);
            host::with(|h| h.engine.note_passthrough(c));
            return false;
        }
        if composing && self.restore_bare_question(client) {
            // 空格只是「把这个 ? 上屏」，不再多打一个空格；其他键按非组句状态继续处理
            if c == ' ' {
                return true;
            }
            return self.handle_text(text, shift, client);
        }
        // 按住 Shift 打的大写字母：临时打英文，先把拼音原样上屏，再把字母交给应用
        if c.is_ascii_uppercase() {
            if composing {
                self.commit_raw(client);
            }
            host::with(|h| h.engine.note_passthrough(c));
            return false;
        }
        if composing {
            match c {
                ' ' => return self.commit_highlighted(client),
                '1'..='9' => {
                    let offset = usize::from(*byte - b'1');
                    if let Some(index) = host::with(|h| h.session.index_on_page(offset)).flatten() {
                        return self.commit_index(index, client);
                    }
                    // 这一页没有这一格（`gpt6` 只有三个候选）：数字当内容进缓冲区，成为英文直输段；
                    // 问字模式里数字不是问题的一部分，不算
                    if !question {
                        host::with(|h| h.engine.push(c));
                        self.refresh(client);
                    }
                    return true;
                }
                c if c == page_previous => return self.turn_page(-1, client),
                c if c == page_next => return self.turn_page(1, client),
                // 其他字符：把当前高亮候选上屏，再按非组句状态处理这个字符
                _ => {
                    self.commit_highlighted(client);
                }
            }
        }
        // 中文模式下的全角标点；转不了的（数字、字母以外的其他键）原样交给应用
        match host::with(|h| h.engine.punctuate(c)).flatten() {
            Some(full_width) => {
                client.insert_text(full_width);
                true
            }
            None => {
                host::with(|h| h.engine.note_passthrough(c));
                false
            }
        }
    }

    /// 组句期间所有编辑动作都由我们接管；不认识的一律吞掉，否则应用会动光标、丢 marked text。
    fn handle_command(&self, selector: Sel, client: TextClient<'_>) -> bool {
        tracing::debug!(selector = %selector, "didCommandBySelector");
        self.note_application(&client);
        let composing = host::with(|h| !h.engine.composition().is_empty()).unwrap_or(false);
        if !composing {
            // 删的是应用里的文字：刚上屏的词被整个删掉是「选错了」的信号，Engine 记着；
            // 按词 / 按行删的数不清删了几个字，撤销的账就不记了
            if selector == sel!(deleteBackward:) {
                host::with(|h| h.engine.note_backspace());
            } else if selector == sel!(deleteWordBackward:)
                || selector == sel!(deleteToBeginningOfLine:)
            {
                host::with(|h| h.engine.break_chain());
            } else if selector == sel!(insertNewline:) {
                // 回车交给应用：文本流里是一个段落边界
                host::with(|h| h.engine.note_passthrough('\n'));
            } else {
                // 其余编辑动作（方向键、按词 / 按行跳、翻页、Tab、Esc、向前删）交给应用：光标可能动了，英文上文作废
                host::with(|h| h.engine.break_english_context());
            }
            return false;
        }
        if selector == sel!(deleteBackward:) {
            host::with(|h| h.engine.backspace());
            self.refresh(client);
        } else if selector == sel!(deleteWordBackward:) {
            // ⌥⌫：删光标前一个音节
            host::with(|h| h.engine.delete_syllable_backward());
            self.refresh(client);
        } else if selector == sel!(deleteToBeginningOfLine:) {
            // ⌘⌫：删光标前的全部拼音
            host::with(|h| h.engine.delete_to_start());
            self.refresh(client);
        } else if selector != sel!(cancelOperation:)
            && selector != sel!(complete:)
            && self.restore_bare_question(client)
        {
            // 只有一个 ? 时按了回车：回车就是「把这个 ? 上屏」，吞掉，否则聊天框里会连消息一起发出去；
            // 方向键等其他键还原后交给应用
            return selector == sel!(insertNewline:);
        } else if selector == sel!(insertNewline:) {
            // 注音下 Enter 是选词（数字键被大千布局占了），Shift+Enter 才原样上屏注音符号；
            // dispatch_event 把不带 Shift 的注音 Enter 提前截走，到这里的一定是「原样上屏」
            self.commit_raw(client);
        } else if selector == sel!(cancelOperation:) || selector == sel!(complete:) {
            // TextEdit 等应用把 Esc 绑成 complete:（自动补全），也当作取消
            host::with(|h| {
                h.engine.clear();
                h.cancel_prediction();
            });
            self.refresh(client);
        } else if selector == sel!(insertTab:) {
            // 英文模式 Tab 选中高亮的词；中文模式有整句补全时接受它，否则翻页
            if host::with(|h| h.engine.english_mode()).unwrap_or(false) {
                self.commit_highlighted(client);
            } else if !self.accept_sentence(client) {
                self.turn_page(1, client);
            }
        } else if selector == sel!(deleteForward:) {
            host::with(|h| h.engine.delete_forward());
            self.refresh(client);
        } else if selector == sel!(moveDown:) {
            self.move_highlight(1, client);
        } else if selector == sel!(moveUp:) {
            self.move_highlight(-1, client);
        } else if selector == sel!(moveLeft:) {
            host::with(|h| h.engine.move_cursor_left());
            self.refresh(client);
        } else if selector == sel!(moveRight:) {
            host::with(|h| h.engine.move_cursor_right());
            self.refresh(client);
        } else if selector == sel!(moveWordLeft:) {
            // ⌥←：光标往左跳一个音节
            host::with(|h| h.engine.move_cursor_syllable_left());
            self.refresh(client);
        } else if selector == sel!(moveWordRight:) {
            // ⌥→：光标往右跳一个音节
            host::with(|h| h.engine.move_cursor_syllable_right());
            self.refresh(client);
        } else if selector == sel!(moveToBeginningOfLine:) || selector == sel!(moveToLeftEndOfLine:)
        {
            host::with(|h| h.engine.move_cursor_home());
            self.refresh(client);
        } else if selector == sel!(moveToEndOfLine:) || selector == sel!(moveToRightEndOfLine:) {
            host::with(|h| h.engine.move_cursor_end());
            self.refresh(client);
        } else if selector == sel!(pageDown:) || selector == sel!(scrollPageDown:) {
            self.turn_page(1, client);
        } else if selector == sel!(pageUp:)
            || selector == sel!(scrollPageUp:)
            || selector == sel!(insertBacktab:)
        {
            self.turn_page(-1, client);
        }
        true
    }

    /// 按当前缓冲区重新查候选、更新 marked text，回到第一页并重画候选窗口。
    fn refresh(&self, client: TextClient<'_>) {
        // 光标前文既是整句与词级排序的上文（接着对方的话、挪过光标之后接着打都靠它），也是本地整句模型看的前文：
        // 有没有模型都读。一段组句只在第一键读一次（组句中它不变；应用偶尔不回话也不至于让前文来回换），
        // 读应用文本要等应用回话，放在借 Host 之外（见 request_prediction）
        let wants_context = host::with(|h| {
            h.attach_loaded_model();
            h.engine.composition().text().chars().count() == 1
        })
        .unwrap_or(false);
        let before = if wants_context && !secure_input::enabled() {
            Some(
                client
                    .surrounding_text(RESCORE_LOOKBACK, 0)
                    .map(|text| text.before),
            )
        } else {
            None
        };
        let Some((marked, cursor, inline)) = host::with(|h| {
            if let Some(before) = before {
                h.engine.set_surrounding_before(before);
            }
            // 查询失败（整段切不动）时退回显示原始字母
            let mut marked = h.engine.composition().text().to_owned();
            let mut cursor = h.engine.composition().cursor();
            let mut preedit = Preedit::plain(&marked, cursor);
            let candidates = h
                .engine
                .query()
                .map(|mut query| {
                    h.engine.annotate(&mut query.candidates);
                    marked = query.marked_text();
                    cursor = query.marked_cursor();
                    preedit = Preedit::from_marked(&query.marked_segments(), cursor);
                    query.candidates.items
                })
                .unwrap_or_default();
            h.reset_session(preedit, candidates);
            h.schedule_rescoring();
            (marked, cursor, h.preedit_mode.inline())
        }) else {
            return;
        };
        // 配置成只在候选窗口显示拼音时，应用里不放 marked text（光标位置仍按插入点取）
        if inline {
            client.set_marked_text(&marked, cursor);
        } else {
            client.set_marked_text("", 0);
        }
        // 先发联想再画：发出去就留好云端槽位，画出来的第一帧本地候选就已经在最终位置
        if !marked.is_empty() {
            let candidates = host::with(|h| h.session.layout.local().to_vec()).unwrap_or_default();
            self.request_prediction(client, &candidates);
        }
        self.render(client);
    }

    /// 缓冲区里只有一个 `?` 而用户按了别的键：把它还原成问号上屏（中文遵循标点设置、英文半角）、清空缓冲区。
    /// 返回是否发生了还原。
    fn restore_bare_question(&self, client: TextClient<'_>) -> bool {
        let english = modifiers::english_mode();
        let restored = host::with(|h| {
            let mark = h.engine.restore_bare_question(english)?;
            h.cancel_prediction();
            Some(mark)
        })
        .flatten();
        let Some(mark) = restored else {
            return false;
        };
        client.insert_text(&mark);
        self.refresh(client);
        true
    }

    /// 记下光标位置并按会话状态重画候选窗口。
    fn render(&self, client: TextClient<'_>) {
        let anchor = client.caret_rect();
        host::with(|h| {
            h.anchor = anchor;
            h.render();
        });
    }

    /// 发一次联想请求。Secure Input 里绝不发；没接联想器时是空操作。
    ///
    /// 读上下文要等应用回话，这段时间 IMK 可能把 `deactivateServer:` 之类的回调插进来，
    /// 所以分两次借 Host：先拿策略、放开借用去读、再借回来发请求。
    fn request_prediction(&self, client: TextClient<'_>, candidates: &[Candidate]) {
        let policy = host::with(|h| {
            if !h.engine.prediction_enabled() {
                return None;
            }
            if secure_input::enabled() {
                tracing::debug!("Secure Input 中，不联想");
                h.cancel_prediction();
                return None;
            }
            Some(h.engine.prediction_policy())
        })
        .flatten();
        let Some(policy) = policy else {
            return;
        };
        let surrounding = client.surrounding_text(policy.before, policy.after);
        host::with(|h| {
            tracing::debug!(
                has_context = surrounding.is_some(),
                pinyin = h.engine.composition().scope(),
                "联想请求"
            );
            match h.engine.request_prediction(surrounding, candidates) {
                Some(_) => h.await_prediction(),
                None => h.cancel_prediction(),
            }
        });
    }

    /// 接受组句中的整句补全：作用域内的拼音作废，句子上屏。没有补全返回 false。
    fn accept_sentence(&self, client: TextClient<'_>) -> bool {
        let Some(text) = host::with(|h| h.sentence.take()).flatten() else {
            return false;
        };
        host::with(|h| h.engine.accept_prediction(&text));
        tracing::debug!(%text, "接受整句补全");
        client.insert_text(&text);
        self.refresh(client);
        true
    }

    /// 高亮上下移动，越过页边自动翻页。
    fn move_highlight(&self, delta: isize, client: TextClient<'_>) {
        if host::with(|h| h.session.move_highlight(delta)).unwrap_or(false) {
            self.render(client);
        }
    }

    /// 每个键都问一次应用标识（activateServer 时进程刚拉起可能还拿不到），变了才告诉 Engine、刷新「英 / 英+」。
    fn note_application(&self, client: &TextClient<'_>) {
        let app = client.bundle_identifier();
        host::with(|h| {
            if h.engine.application() != app.as_deref() {
                h.engine.set_application(app);
                h.sync_english_assist_indicator();
                h.indicator.update();
            }
        });
    }

    /// 翻页，高亮落到新页第一项。已在首页 / 末页时不动。
    fn turn_page(&self, delta: isize, client: TextClient<'_>) -> bool {
        let turned = host::with(|h| {
            let turned = h.session.turn_page(delta);
            if turned {
                h.engine.note_page_turn();
            }
            turned
        })
        .unwrap_or(false);
        if turned {
            self.render(client);
        }
        true
    }
}
