//! `IBusProperty`（`src/ibusproperty.c`）：`(s a{sv} s u v s v b b u v v)`，依次是 key、type、label、icon、tooltip、
//! sensitive、visible、state、sub_props（`IBusPropList`）、symbol；`IBusPropList`（`src/ibusproplist.c`）是 `(s a{sv} av)`。

use zbus::zvariant::Value;

use super::text::text;
use super::{boxed, serializable};
use crate::frontend::{ASSIST_PROPERTY, MODE_PROPERTY};

/// `PROP_TYPE_TOGGLE`。
const PROP_TYPE_TOGGLE: u32 = 1;

/// `PROP_STATE_UNCHECKED`。
const PROP_STATE_UNCHECKED: u32 = 0;

/// `PROP_STATE_CHECKED`。
const PROP_STATE_CHECKED: u32 = 1;

/// 中英模式属性：切换型，英文模式时 checked；label 与面板图标位的 symbol 显示「中」/「英」，
/// 英文模式带补全、纠错、预测时显示「英+」。
pub fn mode_property(english: bool, assist: bool) -> Value<'static> {
    let (label, state) = match (english, assist) {
        (true, true) => ("英+", PROP_STATE_CHECKED),
        (true, false) => ("英", PROP_STATE_CHECKED),
        (false, _) => ("中", PROP_STATE_UNCHECKED),
    };
    toggle_property(MODE_PROPERTY, label, "切换中 / 英（单击 Shift）", state)
}

/// 英文辅助属性：切换型，开着时 checked，点了开关英文模式的补全、纠错、预测。
pub fn assist_property(assist: bool) -> Value<'static> {
    let state = if assist {
        PROP_STATE_CHECKED
    } else {
        PROP_STATE_UNCHECKED
    };
    toggle_property(ASSIST_PROPERTY, "英文补全、纠错、预测", "", state)
}

/// 一个切换型属性；面板图标位的 symbol 与 label 同字。
fn toggle_property(key: &str, label: &str, tooltip: &str, state: u32) -> Value<'static> {
    serializable(
        "IBusProperty",
        vec![
            Value::from(key.to_owned()),
            Value::U32(PROP_TYPE_TOGGLE),
            boxed(text(label, &[])),
            Value::from(String::new()),
            boxed(text(tooltip, &[])),
            Value::Bool(true),
            Value::Bool(true),
            Value::U32(state),
            boxed(property_list(Vec::new())),
            boxed(text(label, &[])),
        ],
    )
}

/// 属性表。
pub fn property_list(properties: Vec<Value<'static>>) -> Value<'static> {
    let properties: Vec<Value<'static>> = properties.into_iter().map(boxed).collect();
    serializable("IBusPropList", vec![Value::from(properties)])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn property_signature_matches_ibus() {
        let value = mode_property(true, false);
        assert_eq!(value.value_signature().to_string(), "(sa{sv}suvsvbbuvv)");
        let Value::Structure(structure) = &value else {
            panic!("IBusProperty 应是结构体");
        };
        assert_eq!(structure.fields()[9], Value::U32(PROP_STATE_CHECKED));
    }

    #[test]
    fn property_list_signature_matches_ibus() {
        let value = property_list(vec![mode_property(false, true), assist_property(true)]);
        assert_eq!(value.value_signature().to_string(), "(sa{sv}av)");
    }

    #[test]
    fn english_with_assist_reads_english_plus() {
        let label = |value: Value<'static>| {
            let Value::Structure(structure) = value else {
                panic!("IBusProperty 应是结构体");
            };
            structure.fields()[4].clone()
        };
        assert_eq!(label(mode_property(true, true)), boxed(text("英+", &[])));
        assert_eq!(label(mode_property(true, false)), boxed(text("英", &[])));
        assert_eq!(label(mode_property(false, true)), boxed(text("中", &[])));
        let Value::Structure(assist) = assist_property(false) else {
            panic!("IBusProperty 应是结构体");
        };
        assert_eq!(assist.fields()[9], Value::U32(PROP_STATE_UNCHECKED));
    }
}
