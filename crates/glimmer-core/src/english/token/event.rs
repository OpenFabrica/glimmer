//! 切词的输出事件：一个词，或一处断开。

/// [`super::tokenize`] 给出的一个事件。
///
/// 流的形状是确定的：不会连着两个断开（连续的断开只留最后一个）；开头的 [`Token::SentenceEnd`] 省掉
/// （文本开头本来就是句首）；有过词的流一定以 [`Token::SentenceEnd`] 结尾（文本结尾 = 一条消息说完）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Token {
    /// 一个词：已去掉两边标点、转小写、弯撇号 `’` 归一成 `'`（`don't`）。
    /// 调用方拿 [`glimmer_dictionary::english_code`] 算编码查词表，查不到就当 [`Token::Gap`] 处理。
    Word(String),

    /// 句子结束：句末标点（`.` `!` `?` `…`）、换行、文本结尾。下一个词从句首算。
    SentenceEnd,

    /// 断开但不是句首：中间夹了一段不是词的东西（数字、URL、带符号的标识符、非 ASCII 字母的词）。
    /// 下一个词的上文未知，不能把两边的词接起来，也不能当句首。
    Gap,
}
