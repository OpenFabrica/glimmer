// 一个输入上下文的会话状态：持有 Rust 会话，把 fcitx5 事件送进去、把产出的指令画到输入面板。
#ifndef GLIMMER_FCITX5_STATE_H
#define GLIMMER_FCITX5_STATE_H

#include <cstddef>
#include <memory>

#include <fcitx-utils/event.h>
#include <fcitx/event.h>
#include <fcitx/inputcontext.h>
#include <fcitx/inputcontextproperty.h>

#include "glimmer_fcitx5.h"

namespace glimmer {

class GlimmerEngine;

// 会话状态（InputContextProperty）：随输入上下文创建与销毁。
class GlimmerState final : public fcitx::InputContextProperty {
public:
    // 开 Rust 会话；后端没起来时会话为空，所有事件都是空操作。
    GlimmerState(GlimmerEngine *engine, fcitx::InputContext *ic);

    // 关会话。
    ~GlimmerState() override;

    GlimmerState(const GlimmerState &) = delete;
    GlimmerState &operator=(const GlimmerState &) = delete;

    // 按键。
    void keyEvent(fcitx::KeyEvent &event);

    // 获焦 / 切到本输入法。
    void activate();

    // 失焦 / 切走。
    void deactivate();

    // 应用要求重置。
    void reset();

    // 面板翻页。
    void page(bool next);

    // 面板移动高亮。
    void moveCursor(bool down);

    // 点选本页第 index 个候选。
    void select(int index);

    // 状态区点了中 / 英。
    void toggleMode();

    // 当前是英文模式。
    bool english() const;

    // 英文模式带补全、纠错、预测（全局开关，Ctrl+Shift+E 切）。
    bool englishAssist() const;

    // 状态区短文字：中 / 英 / 英+（英文模式且辅助开着）。
    const char *modeLabel() const;

private:
    // 把光标前文、光标矩形、私密状态报给会话（起组句前调，会话只在需要时用）。
    void syncContext();

    // 执行一串指令并释放；最后刷新面板、按需起轮询。
    void apply(GlimmerOutputs *outputs);

    // 画 preedit：客户端支持内联 preedit 就放客户端，否则放面板。
    void applyPreedit(GlimmerOutputs *outputs, size_t index);

    // 组句中或显示着英文预测时确保 60 ms 轮询在排着；都不是就让它自然停下。
    void schedulePoll();

    // 所属引擎。
    GlimmerEngine *engine_;

    // 对应的输入上下文。
    fcitx::InputContext *ic_;

    // Rust 会话；后端初始化失败时为空。
    GlimmerSession *session_;

    // 组句中的轮询定时器（一次性，触发后按需重排）。
    std::unique_ptr<fcitx::EventSourceTime> pollTimer_;

    // 轮询定时器已排上、还没触发。
    bool pollArmed_ = false;
};

} // namespace glimmer

#endif
