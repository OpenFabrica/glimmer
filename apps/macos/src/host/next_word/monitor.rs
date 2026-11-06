//! 下一词预测的定时器：有预测在等着画或显示着时跑，第一拍到插入点画出来，之后每拍问 Core 预测还在不在（过期就收）。
//! 回到 `Idle` 就停，平时不占 CPU。

use objc2::rc::Retained;
use objc2::runtime::AnyObject;
use objc2::{MainThreadMarker, MainThreadOnly, define_class, msg_send, sel};
use objc2_foundation::{NSObject, NSObjectProtocol, NSTimer};

/// 轮询间隔。也是空格之后到画出预测的延迟：要等应用把空格排进文本，插入点才是新的。
const POLL_INTERVAL: f64 = 0.05;

/// 下一词预测的轮询定时器。
pub struct NextWordMonitor {
    /// 定时器；没在跑时为 `None`。
    timer: Option<Retained<NSTimer>>,

    mtm: MainThreadMarker,
}

impl NextWordMonitor {
    /// 建一个没在跑的定时器。
    pub fn new(mtm: MainThreadMarker) -> Self {
        Self { timer: None, mtm }
    }

    /// 开始轮询；已经在跑就不动（不重排第一拍）。
    pub fn start(&mut self) {
        if self.timer.is_some() {
            return;
        }
        let target = NextWordTicker::new(self.mtm);
        let timer = unsafe {
            NSTimer::scheduledTimerWithTimeInterval_target_selector_userInfo_repeats(
                POLL_INTERVAL,
                &target,
                sel!(tick:),
                None,
                true,
            )
        };
        self.timer = Some(timer);
    }

    /// 停表。
    pub fn stop(&mut self) {
        if let Some(timer) = self.timer.take() {
            timer.invalidate();
        }
    }
}

define_class!(
    // SAFETY: NSObject 没有子类化要求；没有实现 Drop。
    #[unsafe(super(NSObject))]
    #[thread_kind = MainThreadOnly]
    #[ivars = ()]
    struct NextWordTicker;

    impl NextWordTicker {
        /// 定时器回调：ObjC 运行时直接调的，panic 不能穿出去。
        #[unsafe(method(tick:))]
        fn tick(&self, _timer: Option<&AnyObject>) {
            if crate::imk::catch_panic("下一词预测定时器", super::poll_next_word).is_none() {
                crate::imk::recover_from_panic(None);
            }
        }
    }

    unsafe impl NSObjectProtocol for NextWordTicker {}
);

impl NextWordTicker {
    /// 定时器的 target。
    fn new(mtm: MainThreadMarker) -> Retained<Self> {
        let this = mtm.alloc::<Self>().set_ivars(());
        unsafe { msg_send![super(this), init] }
    }
}
