//! 下一词预测在壳里的显示阶段：没有 / 等一拍再画 / 显示着。只管状态转移，不碰窗口与 Engine。

/// 下一词预测的显示阶段。预测出不出、出什么由 Core 定，这里只记「窗口上有没有它、Tab 该不该接」。
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum NextWordPhase {
    /// 没有预测在显示，也没在等着画。
    #[default]
    Idle,

    /// 有预测，等定时器下一拍到插入点去画：直通的空格这时还没进应用，现在取插入点会差一个空格。
    /// `visible` 为真表示窗口上还挂着上一个预测（Tab 接受之后），这一拍里 Tab 照样接。
    Pending { visible: bool },

    /// 预测画在窗口上。
    Showing,
}

impl NextWordPhase {
    /// 窗口上是否挂着预测：收起时要不要清会话、收窗口。
    pub fn visible(self) -> bool {
        matches!(self, Self::Showing | Self::Pending { visible: true })
    }

    /// Tab 该不该交给 Core 接受：用户眼前有预测时才接。空格刚敲下、还没画出来时的 Tab 照常交给应用，
    /// 免得接受一个用户没看见的词。
    pub fn accepts_tab(self) -> bool {
        self.visible()
    }

    /// 取了一次预测（直通空格之后、Tab 接受之后）：有就等一拍再画，窗口上已挂着的先留着（免得闪一下）；没有就回到 `Idle`。
    pub fn requested(self, available: bool) -> Self {
        if available {
            Self::Pending {
                visible: self.visible(),
            }
        } else {
            Self::Idle
        }
    }

    /// 定时器一拍：预测没了（过期、模式切走、密码框）回到 `Idle`；否则阶段不变，`Pending` 由调用方去画。
    pub fn tick(self, available: bool) -> Self {
        if available { self } else { Self::Idle }
    }

    /// 这一键按下时预测在显示或等着画：用户看见了还按别的键，就是不要它，这一键之后不再取预测
    /// （与 Server 的 `handle_key` 一致；Tab 接受不算，接受后照样取下一个）。
    pub fn declined_by_key(self) -> bool {
        self != Self::Idle
    }

    /// 画出来了。
    pub fn shown(self) -> Self {
        Self::Showing
    }
}

#[cfg(test)]
mod tests {
    use super::NextWordPhase;

    #[test]
    fn space_waits_a_tick_before_showing_and_tab_is_not_taken_meanwhile() {
        let phase = NextWordPhase::Idle.requested(true);
        assert_eq!(phase, NextWordPhase::Pending { visible: false });
        assert!(!phase.accepts_tab());
        assert!(!phase.visible());
        let phase = phase.tick(true).shown();
        assert_eq!(phase, NextWordPhase::Showing);
        assert!(phase.accepts_tab());
    }

    #[test]
    fn accepting_keeps_the_window_until_the_next_word_is_drawn() {
        let phase = NextWordPhase::Showing.requested(true);
        assert_eq!(phase, NextWordPhase::Pending { visible: true });
        // 连按 Tab：下一个还没画出来时也接
        assert!(phase.accepts_tab());
        assert_eq!(phase.tick(true), phase);
    }

    #[test]
    fn nothing_to_predict_goes_idle() {
        assert_eq!(NextWordPhase::Showing.requested(false), NextWordPhase::Idle);
        assert_eq!(NextWordPhase::Idle.requested(false), NextWordPhase::Idle);
        // 过期 / 模式切走：显示着的、等着画的都回到 Idle
        assert_eq!(NextWordPhase::Showing.tick(false), NextWordPhase::Idle);
        assert_eq!(
            NextWordPhase::Pending { visible: false }.tick(false),
            NextWordPhase::Idle
        );
        assert!(!NextWordPhase::Idle.accepts_tab());
    }

    #[test]
    fn a_key_over_a_prediction_declines_it() {
        // 预测显示着或等着画时敲空格：空格照常交给应用，但不再取预测
        assert!(NextWordPhase::Showing.declined_by_key());
        assert!(NextWordPhase::Pending { visible: false }.declined_by_key());
        assert!(NextWordPhase::Pending { visible: true }.declined_by_key());
        // 没有预测时敲的空格照常取
        assert!(!NextWordPhase::Idle.declined_by_key());
    }
}
