//! 模型 token 变回词表写法：语言模型里的词都是小写（`i`、`iphone`），给用户看的要用随包词表里的写法（`I`、`iPhone`）。

use glimmer_dictionary::{WordEntry, WordList};

/// 词表里与模型 token 只差大小写的写法（`i` → `I`）。`code` 是 token 的检索编码（[`super::english_code`]），
/// 调用方多半已经算好，这里不再算一遍。词表里没有的 token（句末标记之类）一条都没有。
pub fn token_forms<'a>(
    list: &'a WordList,
    code: &str,
    token: &'a str,
) -> impl Iterator<Item = &'a WordEntry> + 'a {
    list.get_all(code)
        .iter()
        .filter(move |form| form.word().eq_ignore_ascii_case(token))
}
