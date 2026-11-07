//! 按键相关：TSF 虚拟键码到协议 [`KeyEvent`](glimmer_platform::protocol::KeyEvent) 的翻译（[`event`]）、
//! 单击 Shift 切中英的判定（[`shift`]）、「翻译选中文字」与「开关英文辅助」快捷键的保留键登记（[`preserved`]）。

pub(crate) mod event;
mod layout;
pub(crate) mod preserved;
mod shift;

pub(crate) use self::shift::ShiftTap;
