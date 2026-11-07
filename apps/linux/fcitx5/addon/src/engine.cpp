// 微明 Fcitx5 引擎的实现：后端初始化、事件分派、空闲节拍。
#include "engine.h"

#include <cstdlib>
#include <cstring>
#include <ctime>

#include <fcitx-utils/log.h>
#include <fcitx/addonmanager.h>
#include <fcitx/inputcontext.h>
#include <fcitx/inputcontextmanager.h>
#include <fcitx/statusarea.h>
#include <fcitx/userinterfacemanager.h>

#include "glimmer_fcitx5.h"

namespace glimmer {

namespace {

// 首次空闲节拍前的等待（微秒）。
constexpr uint64_t FIRST_TICK_USEC = 1000 * 1000;

// 读后端配置：资源根优先取环境变量 GLIMMER_DATA_ROOT（开发测试用），否则编译期的装机路径；GLIMMER_ECHO=1 用回显后端。
int initBackend() {
    const char *root = std::getenv("GLIMMER_DATA_ROOT");
    if (!root || !*root) {
        root = GLIMMER_DATA_ROOT;
    }
    const char *echo = std::getenv("GLIMMER_ECHO");
    const bool useEcho = echo && std::strcmp(echo, "1") == 0;
    return glimmer_backend_init(root, useEcho ? 1 : 0);
}

} // namespace

GlimmerEngine::GlimmerEngine(fcitx::Instance *instance)
    : instance_(instance),
      modeAction_(this),
      factory_([this](fcitx::InputContext &ic) { return new GlimmerState(this, &ic); }) {
    if (initBackend() != 0) {
        // 后端装不起来时会话建不出来，按键全部放行，不拖垮 fcitx5
        FCITX_ERROR() << "微明后端初始化失败，按键将全部放行";
    }
    instance_->inputContextManager().registerProperty("glimmerState", &factory_);
    instance_->userInterfaceManager().registerAction("glimmer-mode", &modeAction_);
    tickTimer_ = instance_->eventLoop().addTimeEvent(
        CLOCK_MONOTONIC, fcitx::now(CLOCK_MONOTONIC) + FIRST_TICK_USEC, 0,
        [](fcitx::EventSourceTime *source, uint64_t) {
            const uint64_t waitMs = glimmer_tick();
            source->setNextInterval(waitMs * 1000);
            source->setOneShot();
            return true;
        });
}

GlimmerEngine::~GlimmerEngine() {
    tickTimer_.reset();
    glimmer_shutdown();
}

GlimmerState *GlimmerEngine::state(fcitx::InputContext *ic) {
    return ic->propertyFor(&factory_);
}

void GlimmerEngine::keyEvent(const fcitx::InputMethodEntry &, fcitx::KeyEvent &event) {
    state(event.inputContext())->keyEvent(event);
}

void GlimmerEngine::activate(const fcitx::InputMethodEntry &, fcitx::InputContextEvent &event) {
    auto *ic = event.inputContext();
    auto &statusArea = ic->statusArea();
    statusArea.clearGroup(fcitx::StatusGroup::InputMethod);
    statusArea.addAction(fcitx::StatusGroup::InputMethod, &modeAction_);
    state(ic)->activate();
}

void GlimmerEngine::deactivate(const fcitx::InputMethodEntry &, fcitx::InputContextEvent &event) {
    state(event.inputContext())->deactivate();
}

void GlimmerEngine::reset(const fcitx::InputMethodEntry &, fcitx::InputContextEvent &event) {
    state(event.inputContext())->reset();
}

std::string GlimmerEngine::subMode(const fcitx::InputMethodEntry &, fcitx::InputContext &ic) {
    return state(&ic)->english() ? "英文" : "中文";
}

std::string GlimmerEngine::subModeLabelImpl(const fcitx::InputMethodEntry &, fcitx::InputContext &ic) {
    return state(&ic)->modeLabel();
}

void GlimmerEngine::refreshMode(fcitx::InputContext *ic) {
    // 别的输入法（如密码框临时切到的键盘布局）占着状态区时不去动它
    if (instance_->inputMethod(ic) != "glimmer") {
        return;
    }
    modeAction_.update(ic);
    ic->updateUserInterface(fcitx::UserInterfaceComponent::StatusArea);
}

fcitx::AddonInstance *GlimmerFactory::create(fcitx::AddonManager *manager) {
    return new GlimmerEngine(manager->instance());
}

} // namespace glimmer

FCITX_ADDON_FACTORY(glimmer::GlimmerFactory)
