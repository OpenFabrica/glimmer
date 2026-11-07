// 中 / 英动作的实现。
#include "action.h"

#include "engine.h"

namespace glimmer {

ModeAction::ModeAction(GlimmerEngine *engine) : engine_(engine) {}

std::string ModeAction::shortText(fcitx::InputContext *ic) const {
    return ic ? engine_->state(ic)->modeLabel() : "中";
}

std::string ModeAction::longText(fcitx::InputContext *ic) const {
    if (!ic || !engine_->state(ic)->english()) {
        return "中文模式（单击 Shift 切换）";
    }
    return engine_->state(ic)->englishAssist() ? "英文模式，带补全、纠错、预测（单击 Shift 切换）"
                                               : "英文模式（单击 Shift 切换）";
}

std::string ModeAction::icon(fcitx::InputContext *) const { return {}; }

void ModeAction::activate(fcitx::InputContext *ic) {
    if (ic) {
        engine_->state(ic)->toggleMode();
    }
}

} // namespace glimmer
