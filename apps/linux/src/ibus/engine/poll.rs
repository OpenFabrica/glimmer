//! 组句中的定时轮询（对应 tsf 的 `PollTimer`）：云联想 / 整句重排的结果是异步到的，没有新按键时靠它拉回来重绘。
//! 一个引擎实例最多一个轮询任务；不在组句、也没显示英文预测就退出，下次再起（预测过期要靠它收起）。

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use tokio::sync::Mutex;
use zbus::object_server::SignalEmitter;

use super::emit::emit;
use crate::frontend::Session;

/// 轮询间隔。
const INTERVAL: Duration = Duration::from_millis(60);

/// 没有轮询任务在跑就起一个。
pub(super) fn ensure(
    session: &Arc<Mutex<Session>>,
    polling: &Arc<AtomicBool>,
    emitter: &SignalEmitter<'_>,
) {
    if polling.swap(true, Ordering::SeqCst) {
        return;
    }
    let session = Arc::clone(session);
    let polling = Arc::clone(polling);
    let emitter = SignalEmitter::from_parts(
        emitter.connection().clone(),
        emitter.path().to_owned().into_owned(),
    );
    tokio::spawn(async move {
        loop {
            tokio::time::sleep(INTERVAL).await;
            let mut guard = session.lock().await;
            // 「不用轮询就退出」与清标志必须在同一段锁里：按键处理在锁外看标志决定要不要重起任务。
            if !guard.polling() {
                polling.store(false, Ordering::SeqCst);
                return;
            }
            let outputs = guard.poll();
            emit(&emitter, outputs).await;
        }
    });
}
