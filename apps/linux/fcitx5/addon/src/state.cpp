// 会话状态的实现：事件转发、指令应用、轮询定时器。
#include "state.h"

#include <ctime>
#include <string>

#include <fcitx-utils/capabilityflags.h>
#include <fcitx-utils/textformatflags.h>
#include <fcitx/inputpanel.h>
#include <fcitx/instance.h>
#include <fcitx/text.h>
#include <fcitx/userinterface.h>

#include "candidate.h"
#include "engine.h"

namespace glimmer {

namespace {

// 组句中轮询异步结果（云联想 / 整句重排）、显示着英文预测时看它过没过期的间隔（微秒），与 IBus 前端一致。
constexpr uint64_t POLL_INTERVAL_USEC = 60 * 1000;

// C 字符串转 std::string，空指针为空串。
std::string str(const char *text) { return text ? std::string(text) : std::string(); }

} // namespace

GlimmerState::GlimmerState(GlimmerEngine *engine, fcitx::InputContext *ic)
    : engine_(engine), ic_(ic), session_(glimmer_session_new()) {}

GlimmerState::~GlimmerState() {
    pollTimer_.reset();
    glimmer_session_free(session_);
}

void GlimmerState::keyEvent(fcitx::KeyEvent &event) {
    if (!glimmer_session_composing(session_)) {
        syncContext();
    }
    const fcitx::Key key = event.rawKey();
    int handled = 0;
    apply(glimmer_session_key(session_, static_cast<uint32_t>(key.sym()),
                              static_cast<uint32_t>(key.states()), event.isRelease() ? 1 : 0, &handled));
    if (handled) {
        event.filterAndAccept();
    }
}

void GlimmerState::activate() {
    syncContext();
    apply(glimmer_session_focus_in(session_, ic_->program().c_str()));
}

void GlimmerState::deactivate() { apply(glimmer_session_focus_out(session_)); }

void GlimmerState::reset() { apply(glimmer_session_reset(session_)); }

void GlimmerState::page(bool next) { apply(glimmer_session_page(session_, next ? 1 : 0)); }

void GlimmerState::moveCursor(bool down) { apply(glimmer_session_move_cursor(session_, down ? 1 : 0)); }

void GlimmerState::select(int index) {
    if (index >= 0) {
        apply(glimmer_session_select(session_, static_cast<uint32_t>(index)));
    }
}

void GlimmerState::toggleMode() { apply(glimmer_session_toggle_mode(session_)); }

bool GlimmerState::english() const { return glimmer_session_english(session_) != 0; }

void GlimmerState::syncContext() {
    const auto caps = ic_->capabilityFlags();
    glimmer_session_set_private(
        session_, caps.test(fcitx::CapabilityFlag::Password) || caps.test(fcitx::CapabilityFlag::Sensitive));
    const auto &rect = ic_->cursorRect();
    glimmer_session_set_cursor_rect(session_, rect.left(), rect.top(), rect.width(), rect.height());
    const auto &surrounding = ic_->surroundingText();
    if (caps.test(fcitx::CapabilityFlag::SurroundingText) && surrounding.isValid()) {
        glimmer_session_set_surrounding(session_, surrounding.text().c_str(), surrounding.cursor());
    }
}

void GlimmerState::apply(GlimmerOutputs *outputs) {
    if (!outputs) {
        return;
    }
    bool panelChanged = false;
    bool modeChanged = false;
    auto &panel = ic_->inputPanel();
    const size_t count = glimmer_outputs_len(outputs);
    for (size_t i = 0; i < count; ++i) {
        switch (glimmer_outputs_kind(outputs, i)) {
        case GLIMMER_OUTPUT_COMMIT:
            ic_->commitString(str(glimmer_outputs_text(outputs, i)));
            break;
        case GLIMMER_OUTPUT_PREEDIT:
            applyPreedit(outputs, i);
            panelChanged = true;
            break;
        case GLIMMER_OUTPUT_CANDIDATES:
            if (glimmer_outputs_candidate_count(outputs, i) > 0) {
                panel.setCandidateList(std::make_unique<GlimmerCandidateList>(this, outputs, i));
            } else {
                panel.setCandidateList(nullptr);
            }
            panelChanged = true;
            break;
        case GLIMMER_OUTPUT_AUXILIARY: {
            const char *text = glimmer_outputs_text(outputs, i);
            panel.setAuxUp(text ? fcitx::Text(text) : fcitx::Text());
            panelChanged = true;
            break;
        }
        case GLIMMER_OUTPUT_MODE:
            modeChanged = true;
            break;
        default:
            break;
        }
    }
    glimmer_outputs_free(outputs);
    if (panelChanged) {
        ic_->updatePreedit();
        ic_->updateUserInterface(fcitx::UserInterfaceComponent::InputPanel);
    }
    if (modeChanged) {
        engine_->refreshMode(ic_);
    }
    schedulePoll();
}

void GlimmerState::applyPreedit(GlimmerOutputs *outputs, size_t index) {
    auto &panel = ic_->inputPanel();
    fcitx::Text text;
    if (glimmer_outputs_text(outputs, index)) {
        const size_t segments = glimmer_outputs_segment_count(outputs, index);
        for (size_t j = 0; j < segments; ++j) {
            // 光标后不参与候选的拼音：fcitx5 的文字格式没有颜色，用斜体区分
            fcitx::TextFormatFlags flags = fcitx::TextFormatFlag::Underline;
            if (glimmer_outputs_segment_dimmed(outputs, index, j)) {
                flags |= fcitx::TextFormatFlag::Italic;
            }
            text.append(str(glimmer_outputs_segment_text(outputs, index, j)), flags);
        }
        text.setCursor(glimmer_outputs_cursor(outputs, index));
    }
    if (ic_->capabilityFlags().test(fcitx::CapabilityFlag::Preedit)) {
        panel.setClientPreedit(text);
        panel.setPreedit(fcitx::Text());
    } else {
        panel.setPreedit(text);
        panel.setClientPreedit(fcitx::Text());
    }
}

void GlimmerState::schedulePoll() {
    if (pollArmed_ || !glimmer_session_polling(session_)) {
        return;
    }
    pollArmed_ = true;
    if (pollTimer_) {
        pollTimer_->setNextInterval(POLL_INTERVAL_USEC);
        pollTimer_->setOneShot();
        return;
    }
    pollTimer_ = engine_->instance()->eventLoop().addTimeEvent(
        CLOCK_MONOTONIC, fcitx::now(CLOCK_MONOTONIC) + POLL_INTERVAL_USEC, 0,
        [this](fcitx::EventSourceTime *, uint64_t) {
            pollArmed_ = false;
            // apply 末尾的 schedulePoll 在还要轮询（组句中 / 显示着预测）时重排本定时器
            apply(glimmer_session_poll(session_));
            return true;
        });
}

} // namespace glimmer
