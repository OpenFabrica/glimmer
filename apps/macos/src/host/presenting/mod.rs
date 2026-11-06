//! 呈现：删候选、按应用关英文候选、翻译选区的起止、提示气泡、会话重置与候选窗口绘制。

mod notice;
mod translation_job;

pub(super) use notice::Notice;
pub use translation_job::TranslationJob;

use glimmer_core::{Candidate, CandidateKind, Cell};
use objc2::MainThreadMarker;
use objc2::rc::Retained;
use objc2::runtime::AnyObject;
use objc2_foundation::NSRect;

use super::cloud::cloud_candidate;
use super::{Host, NextWordPhase};
use crate::candidates::{Frame, Preedit, Row};

impl Host {
    /// 删掉当前页第 `offset` 格的候选：用户词整个删、词库词清学习。返回给用户看的一句话；那格没有候选返回 `None`。
    pub fn forget_candidate(&mut self, offset: usize) -> Option<String> {
        let index = self.session.index_on_page(offset)?;
        let candidate = self.session.candidate(index)?;
        let forgotten = self.engine.forget(&candidate);
        let text = &candidate.text;
        Some(if forgotten.user_word {
            format!("已删除用户词「{text}」")
        } else if forgotten.learning {
            format!("已忘掉对「{text}」的学习记录")
        } else {
            format!("「{text}」是词库里的词，也没有学习记录，没什么可删")
        })
    }

    /// 这个应用里英文模式给不给候选：全局开关开着，且应用不在 `[apps] english_candidates_off` 里。
    pub fn english_candidates_in(&self, bundle: Option<&str>) -> bool {
        self.english_candidates && !bundle.is_some_and(|b| self.apps.english_candidates_off(b))
    }

    /// 开始一次翻译：记下选区，窗口先显示「翻译中…」。调用方已发出请求。
    pub fn begin_translation(&mut self, range: objc2_foundation::NSRange) {
        self.translation = Some(TranslationJob {
            range,
            result: None,
        });
        self.reset_session(None, vec![cloud_candidate("翻译中…".to_owned())]);
        self.await_prediction();
        self.render();
    }

    /// 在候选窗口里显示一行提示，几秒后自动收起（敲键也收）。
    pub fn show_notice(&mut self, text: &str, anchor: NSRect) {
        self.anchor = anchor;
        self.reset_session(None, vec![cloud_candidate(text.to_owned())]);
        self.render();
        let mtm = MainThreadMarker::new().expect("Host 只在主线程用");
        self.notice = Some(Notice::schedule(mtm));
    }

    /// 收起提示；没在显示就什么都不做。
    pub fn clear_notice(&mut self) {
        if self.notice.take().is_some() && self.translation.is_none() {
            self.reset_session(None, Vec::new());
            self.window.hide();
        }
    }

    /// 翻译结束（接受、放弃或失败）：收窗、停轮询。
    pub fn end_translation(&mut self) {
        if self.translation.take().is_some() {
            self.cancel_prediction();
            self.reset_session(None, Vec::new());
            self.window.hide();
        }
    }

    /// 不在按键回调里把正在组句的内容整个作废（配置热加载换了输入方案，缓冲区里的键换了含义）：
    /// 清引擎缓冲区与上屏链、作废联想与整句补全、清会话、收窗。应用里的 marked text 这里够不着，
    /// 记一个 `marked_stale`，由拿得到客户端的路径（[`Self::take_stale_client`]）去清。
    pub fn reset_composition(&mut self) {
        let composing = !self.engine.composition().is_empty();
        self.dismiss_next_word("组句作废");
        self.engine.break_chain();
        self.engine.clear();
        self.cancel_prediction();
        self.sentence = None;
        self.reset_session(None, Vec::new());
        self.window.hide();
        if composing {
            self.marked_stale = true;
        }
    }

    /// [`Self::reset_composition`] 之后要清 marked text 的客户端；没有欠着的返回 `None`。取一次就清了标记。
    pub fn take_stale_client(&mut self) -> Option<Retained<AnyObject>> {
        if !std::mem::take(&mut self.marked_stale) {
            return None;
        }
        self.active_client.clone()
    }

    /// 新一轮候选：每页格数取配置与窗口能画的行数中较小者，云端槽位数取配置。
    pub fn reset_session(&mut self, preedit: Option<Preedit>, candidates: Vec<Candidate>) {
        self.status = None;
        let page_size = self.page_size.min(self.window.max_rows()).max(1);
        self.session
            .reset(preedit, candidates, page_size, self.cloud_slots);
    }

    /// 按会话状态画候选窗口。候选为空且没有 preedit 时收窗。
    pub fn render(&mut self) {
        let size = self.session.layout.page_size();
        let page = self.session.page;
        let mut rows: Vec<Row> = self
            .session
            .page_cells()
            .iter()
            .enumerate()
            .map(|(i, cell)| {
                let Some(candidate) = cell.candidate() else {
                    return Row {
                        index: (i + 1).to_string(),
                        text: String::new(),
                        annotation: Vec::new(),
                        cloud: false,
                    };
                };
                let mut row = Row::from_candidate(i, candidate);
                row.cloud = candidate.kind == CandidateKind::Cloud;
                row
            })
            .collect();
        label_next_word_rows(&mut rows, self.next_word_phase());
        // 页上的译词告诉 Engine：用户上屏那一刻它们在屏幕上，算「见过」（词汇记录）；窗口收起时传空
        let cells = self.session.page_cells();
        self.engine
            .note_displayed(cells.iter().copied().filter_map(Cell::candidate));
        // 配置成只在行内显示时，窗口顶部不画拼音行
        let preedit = self
            .preedit_mode
            .in_window()
            .then(|| self.session.preedit.clone())
            .flatten();
        if rows.is_empty() && self.session.preedit.is_none() {
            self.window.hide();
            return;
        }
        let pages = self.session.pages();
        let footer = (pages > 1).then(|| format!("{}/{pages}", page + 1));
        let frame = Frame {
            preedit,
            rows,
            highlighted: self.session.highlighted.saturating_sub(page * size),
            footer,
            sentence: self.sentence.clone(),
            status: self.status.clone(),
        };
        self.window.show(frame, self.anchor);
    }
}

/// 下一词预测只有 Tab 能接（数字照常交给应用）：第一行的序号换成 ⇥，其余行不标序号，免得用户去按数字。
fn label_next_word_rows(rows: &mut [Row], phase: NextWordPhase) {
    if phase == NextWordPhase::Idle {
        return;
    }
    for (i, row) in rows.iter_mut().enumerate() {
        row.index = if i == 0 {
            "⇥".to_owned()
        } else {
            String::new()
        };
    }
}
