// 微明 Fcitx5 输入法引擎：把 Fcitx5 的事件转给每个输入上下文的 GlimmerState，并持有进程级的空闲节拍与中英状态动作。
#ifndef GLIMMER_FCITX5_ENGINE_H
#define GLIMMER_FCITX5_ENGINE_H

#include <memory>
#include <string>

#include <fcitx-utils/event.h>
#include <fcitx/addonfactory.h>
#include <fcitx/addoninstance.h>
#include <fcitx/inputcontextproperty.h>
#include <fcitx/inputmethodengine.h>
#include <fcitx/instance.h>

#include "action.h"
#include "state.h"

namespace glimmer {

// 引擎：一个 fcitx5 进程一个实例，后端（Router）进程内共用。
class GlimmerEngine final : public fcitx::InputMethodEngineV2 {
public:
    // 建后端、登记会话属性与中英动作、起空闲节拍。
    explicit GlimmerEngine(fcitx::Instance *instance);

    // 退出前让后端落盘学习数据。
    ~GlimmerEngine() override;

    // 按键：送给会话，吃掉的键 filterAndAccept。
    void keyEvent(const fcitx::InputMethodEntry &entry, fcitx::KeyEvent &event) override;

    // 切到本输入法或获焦：开会话、状态区挂上中英动作。
    void activate(const fcitx::InputMethodEntry &entry, fcitx::InputContextEvent &event) override;

    // 切走或失焦：清掉组句、关会话。
    void deactivate(const fcitx::InputMethodEntry &entry, fcitx::InputContextEvent &event) override;

    // 应用要求重置：清掉组句。
    void reset(const fcitx::InputMethodEntry &entry, fcitx::InputContextEvent &event) override;

    // 子模式全称（托盘提示）：中文 / 英文。
    std::string subMode(const fcitx::InputMethodEntry &entry, fcitx::InputContext &ic) override;

    // 子模式短标签（面板指示）：中 / 英 / 英+。
    std::string subModeLabelImpl(const fcitx::InputMethodEntry &entry, fcitx::InputContext &ic) override;

    // fcitx5 实例（定时器要用它的事件循环）。
    fcitx::Instance *instance() const { return instance_; }

    // 输入上下文对应的会话状态。
    GlimmerState *state(fcitx::InputContext *ic);

    // 中英模式变了：刷新状态区的动作与子模式标签。
    void refreshMode(fcitx::InputContext *ic);

private:
    // fcitx5 实例。
    fcitx::Instance *instance_;

    // 状态区的中 / 英动作。
    ModeAction modeAction_;

    // 空闲节拍定时器：按 glimmer_tick 的返回值重排。
    std::unique_ptr<fcitx::EventSourceTime> tickTimer_;

    // 每个输入上下文一个 GlimmerState；放最后，析构时最先销毁、先关会话。
    fcitx::FactoryFor<GlimmerState> factory_;
};

// 插件工厂：fcitx5 加载 glimmer.so 后经它建引擎。
class GlimmerFactory final : public fcitx::AddonFactory {
public:
    // 建引擎实例。
    fcitx::AddonInstance *create(fcitx::AddonManager *manager) override;
};

} // namespace glimmer

#endif
