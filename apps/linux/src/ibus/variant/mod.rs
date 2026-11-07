//! `IBusSerializable` 的 D-Bus 编码。对照 ibus `src/ibusserializable.c`：每个对象序列化成一个结构体，
//! 头两个字段固定是 GType 名（`s`）与附件表（`a{sv}`，我们总是空），其后是各子类按父类在前的顺序追加的字段；
//! 嵌套对象一律包一层 variant（`v`）。信号参数的 `v` 里装的就是这个结构体。

mod attribute;
mod lookup_table;
mod property;
mod text;

use std::collections::HashMap;

use zbus::zvariant::{StructureBuilder, Value};

pub use attribute::Attribute;
pub use lookup_table::lookup_table;
pub use property::{assist_property, mode_property, property_list};
pub use text::text;

/// 拼一个 `IBusSerializable`：`(s a{sv} …fields)`。
fn serializable(type_name: &str, fields: Vec<Value<'static>>) -> Value<'static> {
    let attachments: HashMap<String, Value<'static>> = HashMap::new();
    let mut builder = StructureBuilder::new()
        .add_field(type_name.to_owned())
        .add_field(attachments);
    for field in fields {
        builder = builder.append_field(field);
    }
    Value::Structure(
        builder
            .build()
            .expect("IBusSerializable 至少有两个字段，构造不会失败"),
    )
}

/// 嵌套对象包一层 variant。
fn boxed(value: Value<'static>) -> Value<'static> {
    Value::Value(Box::new(value))
}
