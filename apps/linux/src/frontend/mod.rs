//! 与 D-Bus 无关的前端逻辑：一个引擎实例的状态机（[`Session`]），它产出的指令（[`Output`]），
//! 以及帧到显示内容的换算（[`view`]）。

mod output;
mod session;

pub mod view;

pub use output::Output;
pub use session::Session;

/// 中英模式属性的名字（`IBusProperty` 的 key），面板点它时 `PropertyActivate` 带回来。
pub const MODE_PROPERTY: &str = "InputMode";

/// 英文辅助属性的名字：点它开关英文模式的补全、纠错、预测。
pub const ASSIST_PROPERTY: &str = "EnglishAssist";
