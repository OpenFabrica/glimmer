//! `IBusLookupTable`（`src/ibuslookuptable.c`）：`(s a{sv} u u b b i av av)`，
//! 依次是 `page_size`、`cursor_pos`、`cursor_visible`、`round`、`orientation`、候选（`IBusText`）、标签（`IBusText`）。

use zbus::zvariant::Value;

use super::attribute::Attribute;
use super::text::text;
use super::{boxed, serializable};
use crate::frontend::view::{CandidateRow, CandidateView};

/// 译文注解的灰色。
const ANNOTATION_RGB: u32 = 0x80_80_80;

/// `IBUS_ORIENTATION_HORIZONTAL`。
const HORIZONTAL: i32 = 0;

/// `IBUS_ORIENTATION_VERTICAL`。
const VERTICAL: i32 = 1;

/// 当前页编码成一张候选表：页大小就是本页条数（ibus 限定 1..=16），不循环。
pub fn lookup_table(view: &CandidateView) -> Value<'static> {
    let candidates: Vec<Value<'static>> =
        view.rows.iter().map(|row| boxed(candidate(row))).collect();
    let labels: Vec<Value<'static>> = view
        .rows
        .iter()
        .map(|row| boxed(text(&row.label, &[])))
        .collect();
    let page_size = view.rows.len().clamp(1, 16) as u32;
    serializable(
        "IBusLookupTable",
        vec![
            Value::U32(page_size),
            Value::U32(view.highlight as u32),
            Value::Bool(true),
            Value::Bool(false),
            Value::I32(if view.vertical { VERTICAL } else { HORIZONTAL }),
            Value::from(candidates),
            Value::from(labels),
        ],
    )
}

/// 一个候选：有译文时拼成「文字 译文」，译文段画灰。
fn candidate(row: &CandidateRow) -> Value<'static> {
    match &row.annotation {
        Some(annotation) => {
            let start = row.text.chars().count() + 1;
            let end = start + annotation.chars().count();
            text(
                &format!("{} {annotation}", row.text),
                &[Attribute::foreground(ANNOTATION_RGB, start, end)],
            )
        }
        None => text(&row.text, &[]),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn table_signature_matches_ibus() {
        let view = CandidateView {
            rows: vec![
                CandidateRow {
                    label: "1".to_owned(),
                    text: "你好".to_owned(),
                    annotation: Some("hello".to_owned()),
                },
                CandidateRow {
                    label: "2".to_owned(),
                    text: "拟好".to_owned(),
                    annotation: None,
                },
            ],
            highlight: 1,
            vertical: true,
            page: 0,
            page_count: 1,
        };
        let value = lookup_table(&view);
        assert_eq!(value.value_signature().to_string(), "(sa{sv}uubbiavav)");
        let Value::Structure(structure) = &value else {
            panic!("IBusLookupTable 应是结构体");
        };
        assert_eq!(structure.fields()[2], Value::U32(2));
        assert_eq!(structure.fields()[3], Value::U32(1));
        assert_eq!(structure.fields()[6], Value::I32(VERTICAL));
    }

    #[test]
    fn annotation_is_appended_and_grayed() {
        let row = CandidateRow {
            label: "1".to_owned(),
            text: "你好".to_owned(),
            annotation: Some("hello".to_owned()),
        };
        let expected = text("你好 hello", &[Attribute::foreground(ANNOTATION_RGB, 3, 8)]);
        assert_eq!(candidate(&row), expected);
    }
}
