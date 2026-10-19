//! emoji 候选：按候选词查 emoji（笑 → 😄），跟在对应词后面（前三位留给字词）。数据表由 Unicode CLDR 的中文 annotations 生成
//! （`assets/emoji/emoji-zh.tsv`，`词\temoji emoji …`），`dict-convert emoji` 转换。

mod table;

pub use table::EmojiTable;
