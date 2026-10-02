use serde::Deserialize;
use serde_json::Value;
use std::collections::BTreeMap;
use std::sync::OnceLock;

#[derive(Debug, Deserialize)]
pub struct AppearanceField {
    pub file_kind: String,
    pub pointer: String,
    pub component: String,
    pub kind: String,
    pub default: Option<Value>,
    pub role: String,
    pub min: Option<f64>,
    pub max: Option<f64>,
    #[serde(default)]
    pub allowed: Vec<String>,
    #[serde(default)]
    pub integer: bool,
}

#[derive(Deserialize)]
struct AppearanceSchema {
    fields: Vec<AppearanceField>,
    #[serde(skip)]
    index: BTreeMap<String, BTreeMap<String, usize>>,
}

fn schema() -> &'static AppearanceSchema {
    static SCHEMA: OnceLock<AppearanceSchema> = OnceLock::new();
    SCHEMA.get_or_init(|| {
        let mut schema: AppearanceSchema =
            serde_json::from_str(include_str!("lunar_appearance.json"))
                .expect("The bundled Lunar appearance schema must be valid");
        for (index, field) in schema.fields.iter().enumerate() {
            let previous = schema
                .index
                .entry(field.file_kind.clone())
                .or_default()
                .insert(field.pointer.clone(), index);
            assert!(previous.is_none(), "Duplicate Lunar appearance field");
        }
        schema
    })
}

pub fn fields() -> &'static [AppearanceField] {
    &schema().fields
}

pub fn field(file_kind: &str, pointer: &str) -> Option<&'static AppearanceField> {
    let schema = schema();
    let index = schema.index.get(file_kind)?.get(pointer)?;
    Some(&schema.fields[*index])
}

pub fn can_insert(document: &Value, file_kind: &str, pointer: &str, value: &Value) -> bool {
    let Some(field) = field(file_kind, pointer) else {
        return false;
    };
    if !crate::safety::lunar_value_is_safe(file_kind, pointer, value)
        || document.pointer(pointer).is_some()
        || !document
            .pointer(&field.component)
            .is_some_and(Value::is_object)
        || !pointer.starts_with(&format!("{}/", field.component))
    {
        return false;
    }
    let group = if field.role == "color-value" || field.role.starts_with("chroma") {
        Some(["value", "chroma", "chromaSpeed", "chromaType"].as_slice())
    } else if field.role.starts_with("keybind") {
        Some(["value", "modifier", "alt", "shift", "control"].as_slice())
    } else {
        None
    };
    if let Some(leaves) = group {
        let Some((parent, _)) = pointer.rsplit_once('/') else {
            return false;
        };
        for leaf in leaves {
            let sibling = format!("{parent}/{leaf}");
            if field_exists(file_kind, &sibling)
                && document.pointer(&sibling).is_some_and(|existing| {
                    !crate::safety::lunar_value_is_safe(file_kind, &sibling, existing)
                })
            {
                return false;
            }
        }
    }
    let Some(parts) = pointer.strip_prefix('/') else {
        return false;
    };
    let parts: Vec<_> = parts.split('/').collect();
    let mut parent = document;
    for token in &parts[..parts.len().saturating_sub(1)] {
        let Some(object) = parent.as_object() else {
            return false;
        };
        let key = token.replace("~1", "/").replace("~0", "~");
        let Some(child) = object.get(&key) else {
            return true;
        };
        parent = child;
    }
    parent.is_object()
}

fn field_exists(file_kind: &str, pointer: &str) -> bool {
    field(file_kind, pointer).is_some()
}

pub fn defaults(document: &Value, file_kind: &str) -> Vec<(String, Value)> {
    fields()
        .iter()
        .filter(|field| field.file_kind == file_kind)
        .filter_map(|field| {
            let value = field.default.as_ref()?;
            can_insert(document, file_kind, &field.pointer, value)
                .then(|| (field.pointer.clone(), value.clone()))
        })
        .collect()
}
