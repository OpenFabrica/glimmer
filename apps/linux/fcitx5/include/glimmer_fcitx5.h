/*
 * 微明 Fcitx5 插件的 C 接口：Rust 静态库 libglimmer_fcitx5.a（apps/linux/fcitx5/src）导出，C++ 插件调用。
 *
 * 约定：
 * - 所有函数都在 Rust 侧 catch_unwind，panic 不穿过边界，出错返回空指针 / 0 / -1；
 * - 指针参数一律判空，空指针是安全的空操作；
 * - 字符串一律 UTF-8、以 NUL 结尾；访问器返回的字符串活到对应的 glimmer_outputs_free 为止；
 * - 不是线程安全的：同一个会话只在 fcitx5 的事件循环线程里用。
 */
#ifndef GLIMMER_FCITX5_H
#define GLIMMER_FCITX5_H

#include <stddef.h>
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

/* 一个输入上下文的会话（不透明）。 */
typedef struct GlimmerSession GlimmerSession;

/* 一次事件产出的指令串（不透明），按下标顺序执行，用完 glimmer_outputs_free。 */
typedef struct GlimmerOutputs GlimmerOutputs;

/* 指令种类（glimmer_outputs_kind 的返回值）。 */
enum {
    /* 上屏：glimmer_outputs_text 是要上屏的文字。 */
    GLIMMER_OUTPUT_COMMIT = 0,
    /* 更新 preedit：text 为 NULL 表示收起；否则看 cursor（字节偏移）与分段。 */
    GLIMMER_OUTPUT_PREEDIT = 1,
    /* 更新候选页：candidate_count 为 0 表示收起。 */
    GLIMMER_OUTPUT_CANDIDATES = 2,
    /* 更新辅助文字（候选上方一行）：text 为 NULL 表示收起。 */
    GLIMMER_OUTPUT_AUXILIARY = 3,
    /* 中英模式（含初次登记）：看 glimmer_outputs_english。 */
    GLIMMER_OUTPUT_MODE = 4
};

/* 进程级 */

/* 建全局后端并装日志，成功或已建过返回 0，装配失败返回 -1。
 * data_root 是随包资源根（装机 /usr/lib/glimmer），NULL 按可执行文件位置找；echo 非零用回显后端（只验链路）。 */
int glimmer_backend_init(const char *data_root, int echo);

/* 空闲节拍：推进后端（接模型、看配置、落盘学习数据），返回下一次最晚多少毫秒后再调（≥ 1）。 */
uint32_t glimmer_tick(void);

/* 退出前调一次：学习数据落盘、冲刷日志。 */
void glimmer_shutdown(void);

/* 会话 */

/* 开会话（中文模式）；后端没初始化返回 NULL。 */
GlimmerSession *glimmer_session_new(void);

/* 关协议会话并释放。 */
void glimmer_session_free(GlimmerSession *session);

/* 正在组句（1 / 0，不含英文预测）：按键前要不要同步输入框状态。 */
int glimmer_session_composing(const GlimmerSession *session);

/* 要不要开着轮询（1 / 0）：组句中或显示着英文预测，据此开关 60 ms 轮询。 */
int glimmer_session_polling(const GlimmerSession *session);

/* 当前是英文模式（1 / 0）。 */
int glimmer_session_english(const GlimmerSession *session);

/* 英文模式带补全、纠错、预测（1 / 0）：英文模式下状态区显示「英+」。开关是全局的，按 Ctrl+Shift+E（可配）切换。 */
int glimmer_session_english_assist(const GlimmerSession *session);

/* 事件：都返回要执行的指令，会话为 NULL 或内部出错时返回 NULL */

/* 按键。keysym 是 X keysym（fcitx::KeySym），states 是 fcitx::KeyStates；handled 非 NULL 时写入吃不吃（1 / 0）。 */
GlimmerOutputs *glimmer_session_key(GlimmerSession *session, uint32_t keysym, uint32_t states,
                                    int is_release, int *handled);

/* 获焦（activate）；program 是应用名，可为 NULL。 */
GlimmerOutputs *glimmer_session_focus_in(GlimmerSession *session, const char *program);

/* 失焦 / 切走输入法（deactivate）：清掉组句、关协议会话。 */
GlimmerOutputs *glimmer_session_focus_out(GlimmerSession *session);

/* 应用要求重置：清掉组句，会话保留。 */
GlimmerOutputs *glimmer_session_reset(GlimmerSession *session);

/* 组句中取一次异步结果（云联想 / 整句重排），没变化时指令为空。 */
GlimmerOutputs *glimmer_session_poll(GlimmerSession *session);

/* 面板翻页：next 非零下一页，否则上一页。 */
GlimmerOutputs *glimmer_session_page(GlimmerSession *session, int next);

/* 面板移动高亮：down 非零下移，否则上移。 */
GlimmerOutputs *glimmer_session_move_cursor(GlimmerSession *session, int down);

/* 点选本页第 index 个候选（从 0 起；第十个起没有数字键可按，忽略）。 */
GlimmerOutputs *glimmer_session_select(GlimmerSession *session, uint32_t index);

/* 状态区点了中 / 英：切换模式，组着的拼音先原样上屏。 */
GlimmerOutputs *glimmer_session_toggle_mode(GlimmerSession *session);

/* 应用报来的上下文（不产出指令） */

/* 光标周围的文字，cursor 是光标处的字符（Unicode 标量）下标。 */
void glimmer_session_set_surrounding(GlimmerSession *session, const char *text, uint32_t cursor);

/* 光标矩形（屏幕坐标）。 */
void glimmer_session_set_cursor_rect(GlimmerSession *session, int x, int y, int width, int height);

/* 输入框是否私密（密码 / 敏感）：私密时不学习、不发云端。 */
void glimmer_session_set_private(GlimmerSession *session, int is_private);

/* 指令访问器：outputs 为 NULL 或下标越界时返回 0 / -1 / NULL */

/* 指令条数。 */
size_t glimmer_outputs_len(const GlimmerOutputs *outputs);

/* 第 index 条的种类（GLIMMER_OUTPUT_*），越界 -1。 */
int glimmer_outputs_kind(const GlimmerOutputs *outputs, size_t index);

/* 上屏文字 / preedit 整行 / 辅助文字；收起时 NULL。 */
const char *glimmer_outputs_text(const GlimmerOutputs *outputs, size_t index);

/* preedit 光标，UTF-8 字节偏移。 */
int glimmer_outputs_cursor(const GlimmerOutputs *outputs, size_t index);

/* preedit 按样式分的段数。 */
size_t glimmer_outputs_segment_count(const GlimmerOutputs *outputs, size_t index);

/* 第 segment 段的文字。 */
const char *glimmer_outputs_segment_text(const GlimmerOutputs *outputs, size_t index, size_t segment);

/* 第 segment 段是否画淡（光标之后不参与候选的拼音），1 / 0。 */
int glimmer_outputs_segment_dimmed(const GlimmerOutputs *outputs, size_t index, size_t segment);

/* 本页候选数；0 表示收起候选。 */
size_t glimmer_outputs_candidate_count(const GlimmerOutputs *outputs, size_t index);

/* 候选的数字标签（"1"…"9"，第十个起为空串）。 */
const char *glimmer_outputs_candidate_label(const GlimmerOutputs *outputs, size_t index, size_t position);

/* 候选文字。 */
const char *glimmer_outputs_candidate_text(const GlimmerOutputs *outputs, size_t index, size_t position);

/* 候选的译词注解；没有时 NULL。 */
const char *glimmer_outputs_comment(const GlimmerOutputs *outputs, size_t index, size_t position);

/* 高亮的候选下标（页内）。 */
int glimmer_outputs_highlight(const GlimmerOutputs *outputs, size_t index);

/* 候选竖排 1、横排 0。 */
int glimmer_outputs_vertical(const GlimmerOutputs *outputs, size_t index);

/* 有上一页 1 / 0。 */
int glimmer_outputs_has_prev(const GlimmerOutputs *outputs, size_t index);

/* 有下一页 1 / 0。 */
int glimmer_outputs_has_next(const GlimmerOutputs *outputs, size_t index);

/* 中英模式指令里现在是英文 1 / 0。 */
int glimmer_outputs_english(const GlimmerOutputs *outputs, size_t index);

/* 释放指令串。 */
void glimmer_outputs_free(GlimmerOutputs *outputs);

#ifdef __cplusplus
}
#endif

#endif /* GLIMMER_FCITX5_H */
