// 候选页与候选词的实现。
#include "candidate.h"

#include <string>

#include "state.h"

namespace glimmer {

namespace {

// C 字符串转 std::string，空指针为空串。
std::string str(const char *text) { return text ? std::string(text) : std::string(); }

} // namespace

GlimmerCandidateWord::GlimmerCandidateWord(GlimmerState *state, int index, const char *text, const char *comment)
    : state_(state), index_(index) {
    fcitx::Text word(str(text));
#ifdef GLIMMER_HAS_CANDIDATE_COMMENT
    if (comment) {
        setComment(fcitx::Text(comment));
    }
#else
    // fcitx5 < 5.1.9 没有 comment 字段，译词接在候选文字后面（空格隔开，与 IBus 候选表一致）
    if (comment) {
        word.append(" " + std::string(comment));
    }
#endif
    setText(std::move(word));
}

void GlimmerCandidateWord::select(fcitx::InputContext *) const { state_->select(index_); }

GlimmerCandidateList::GlimmerCandidateList(GlimmerState *state, const GlimmerOutputs *outputs, size_t index)
    : state_(state) {
    setPageable(this);
    setCursorMovable(this);
    const size_t count = glimmer_outputs_candidate_count(outputs, index);
    for (size_t j = 0; j < count; ++j) {
        const std::string label = str(glimmer_outputs_candidate_label(outputs, index, j));
        // 数字序号写成「1. 」；英文下一词预测的「⇥」（只有 Tab 能接）后面只空一格
        const bool numbered = !label.empty() && label.back() >= '0' && label.back() <= '9';
        labels_.emplace_back(label.empty() ? std::string() : label + (numbered ? ". " : " "));
        words_.push_back(std::make_unique<GlimmerCandidateWord>(
            state, static_cast<int>(j), glimmer_outputs_candidate_text(outputs, index, j),
            glimmer_outputs_comment(outputs, index, j)));
    }
    cursor_ = glimmer_outputs_highlight(outputs, index);
    layout_ = glimmer_outputs_vertical(outputs, index) ? fcitx::CandidateLayoutHint::Vertical
                                                       : fcitx::CandidateLayoutHint::Horizontal;
    hasPrev_ = glimmer_outputs_has_prev(outputs, index) != 0;
    hasNext_ = glimmer_outputs_has_next(outputs, index) != 0;
}

size_t GlimmerCandidateList::clamp(int idx) const {
    if (idx < 0) {
        return 0;
    }
    const size_t last = words_.empty() ? 0 : words_.size() - 1;
    return static_cast<size_t>(idx) > last ? last : static_cast<size_t>(idx);
}

const fcitx::Text &GlimmerCandidateList::label(int idx) const { return labels_[clamp(idx)]; }

const fcitx::CandidateWord &GlimmerCandidateList::candidate(int idx) const { return *words_[clamp(idx)]; }

void GlimmerCandidateList::prev() { state_->page(false); }

void GlimmerCandidateList::next() { state_->page(true); }

void GlimmerCandidateList::prevCandidate() { state_->moveCursor(false); }

void GlimmerCandidateList::nextCandidate() { state_->moveCursor(true); }

} // namespace glimmer
