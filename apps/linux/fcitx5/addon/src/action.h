// 状态区的中 / 英切换动作：文字按各输入上下文自己的模式显示，点击切换。
#ifndef GLIMMER_FCITX5_ACTION_H
#define GLIMMER_FCITX5_ACTION_H

#include <string>

#include <fcitx/action.h>

namespace glimmer {

class GlimmerEngine;

// 中 / 英动作。不用 SimpleAction：它的文字是全局一份，而模式是每个输入上下文各自的。
class ModeAction final : public fcitx::Action {
public:
    // 绑定引擎，点击时经它找到会话。
    explicit ModeAction(GlimmerEngine *engine);

    // 短文字：中 / 英 / 英+。
    std::string shortText(fcitx::InputContext *ic) const override;

    // 长文字（悬停提示）。
    std::string longText(fcitx::InputContext *ic) const override;

    // 图标：不设，面板显示短文字。
    std::string icon(fcitx::InputContext *ic) const override;

    // 点击：切换中英模式。
    void activate(fcitx::InputContext *ic) override;

private:
    // 所属引擎。
    GlimmerEngine *engine_;
};

} // namespace glimmer

#endif
