use crate::model::{setting_id, Setting};
use serde::de::{self, MapAccess, SeqAccess, Visitor};
use serde::{Deserialize, Deserializer};
use serde_json::{Map, Number, Value};
use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::ops::Range;

struct StrictValue(Value);

impl<'de> Deserialize<'de> for StrictValue {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct ValueVisitor;
        impl<'de> Visitor<'de> for ValueVisitor {
            type Value = StrictValue;
            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("a JSON value with unique object keys")
            }
            fn visit_bool<E: de::Error>(self, value: bool) -> Result<StrictValue, E> {
                Ok(StrictValue(Value::Bool(value)))
            }
            fn visit_i64<E: de::Error>(self, value: i64) -> Result<StrictValue, E> {
                Ok(StrictValue(Value::Number(value.into())))
            }
            fn visit_u64<E: de::Error>(self, value: u64) -> Result<StrictValue, E> {
                Ok(StrictValue(Value::Number(value.into())))
            }
            fn visit_f64<E: de::Error>(self, value: f64) -> Result<StrictValue, E> {
                Number::from_f64(value)
                    .map(|number| StrictValue(Value::Number(number)))
                    .ok_or_else(|| E::custom("invalid number"))
            }
            fn visit_str<E: de::Error>(self, value: &str) -> Result<StrictValue, E> {
                Ok(StrictValue(Value::String(value.into())))
            }
            fn visit_string<E: de::Error>(self, value: String) -> Result<StrictValue, E> {
                Ok(StrictValue(Value::String(value)))
            }
            fn visit_unit<E: de::Error>(self) -> Result<StrictValue, E> {
                Ok(StrictValue(Value::Null))
            }
            fn visit_none<E: de::Error>(self) -> Result<StrictValue, E> {
                Ok(StrictValue(Value::Null))
            }
            fn visit_seq<A: SeqAccess<'de>>(
                self,
                mut sequence: A,
            ) -> Result<StrictValue, A::Error> {
                let mut items = Vec::new();
                while let Some(StrictValue(item)) = sequence.next_element()? {
                    if items.len() >= 10000 {
                        return Err(de::Error::custom("array limit exceeded"));
                    }
                    items.push(item);
                }
                Ok(StrictValue(Value::Array(items)))
            }
            fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<StrictValue, A::Error> {
                let mut entries = Map::new();
                while let Some((key, StrictValue(value))) =
                    map.next_entry::<String, StrictValue>()?
                {
                    if entries.len() >= 10000 || entries.insert(key, value).is_some() {
                        return Err(de::Error::custom("duplicate or excessive object keys"));
                    }
                }
                Ok(StrictValue(Value::Object(entries)))
            }
        }
        deserializer.deserialize_any(ValueVisitor)
    }
}

pub fn parse_document(content: &str) -> Result<Value, String> {
    if content.len() > 8 * 1024 * 1024 || content.contains('\0') {
        return Err("This Lunar settings file is too large or damaged.".into());
    }
    let StrictValue(value) =
        serde_json::from_str(content.strip_prefix('\u{feff}').unwrap_or(content)).map_err(
            |_| "This Lunar settings file is damaged or has an unsupported structure.".to_string(),
        )?;
    if !value.is_object() {
        return Err("This Lunar settings file has an unsupported structure.".into());
    }
    Ok(value)
}

pub fn parse_settings(
    content: &str,
    file_kind: &str,
    profile: &str,
) -> Result<Vec<Setting>, String> {
    parse_settings_inner(content, file_kind, profile, false)
}

pub fn parse_settings_with_defaults(
    content: &str,
    file_kind: &str,
    profile: &str,
) -> Result<Vec<Setting>, String> {
    parse_settings_inner(content, file_kind, profile, true)
}

fn parse_settings_inner(
    content: &str,
    file_kind: &str,
    profile: &str,
    include_defaults: bool,
) -> Result<Vec<Setting>, String> {
    if !matches!(file_kind, "mods" | "general" | "controls" | "performance") {
        return Err("This Lunar settings file is unsupported.".into());
    }
    let value = parse_document(content)?;
    let mut settings = Vec::new();
    collect(&value, "", file_kind, profile, &mut settings, 0);
    if include_defaults {
        for (pointer, value) in crate::appearance::defaults(&value, file_kind) {
            if settings.len() >= 10000 {
                break;
            }
            let (category, group, label) = describe_pointer(file_kind, &pointer);
            settings.push(Setting {
                id: setting_id("lunar", profile, file_kind, &pointer),
                label,
                source: "lunar".into(),
                category,
                group,
                file_kind: file_kind.into(),
                profile: profile.into(),
                pointer,
                value,
            });
        }
    }
    Ok(settings)
}

fn collect(
    value: &Value,
    pointer: &str,
    file_kind: &str,
    profile: &str,
    settings: &mut Vec<Setting>,
    depth: usize,
) {
    if depth > 12 || settings.len() >= 10000 {
        return;
    }
    if let Value::Object(map) = value {
        for (key, child) in map {
            if key.len() > 128 {
                continue;
            }
            let token = key.replace('~', "~0").replace('/', "~1");
            collect(
                child,
                &format!("{pointer}/{token}"),
                file_kind,
                profile,
                settings,
                depth + 1,
            );
        }
    } else if crate::safety::lunar_value_is_safe(file_kind, pointer, value) {
        let (category, group, label) = describe_pointer(file_kind, pointer);
        settings.push(Setting {
            id: setting_id("lunar", profile, file_kind, pointer),
            label,
            source: "lunar".into(),
            category,
            group,
            file_kind: file_kind.into(),
            profile: profile.into(),
            pointer: pointer.into(),
            value: value.clone(),
        });
    }
}

pub fn merge_settings(content: &str, settings: &[Setting]) -> Result<String, String> {
    let document = parse_document(content)?;
    let mut seen = BTreeSet::new();
    let mut changes = BTreeMap::new();
    let mut additions = BTreeMap::new();
    for setting in settings {
        crate::safety::validate_setting(setting)?;
        if setting.source != "lunar" || !seen.insert(setting.pointer.as_str()) {
            return Err("The selected Lunar settings are unsupported or duplicated.".into());
        }
        if can_insert_setting(&document, setting) {
            additions.insert(setting.pointer.clone(), setting.value.clone());
        } else {
            let existing = document
                .pointer(&setting.pointer)
                .ok_or("A selected setting is unavailable in this Lunar profile.")?;
            let value = value_for_existing_type(
                &setting.file_kind,
                &setting.pointer,
                existing,
                &setting.value,
            )
            .ok_or("A selected setting uses a different Lunar schema version.")?;
            changes.insert(setting.pointer.clone(), value);
        }
    }
    patch_scalar_values(content, &changes, &additions)
}

pub(crate) fn value_for_existing_type(
    file_kind: &str,
    pointer: &str,
    existing: &Value,
    incoming: &Value,
) -> Option<Value> {
    if !crate::safety::lunar_value_is_safe(file_kind, pointer, existing)
        || !crate::safety::lunar_share_value_is_safe(file_kind, pointer, incoming)
    {
        return None;
    }
    if same_value_type(existing, incoming)
        && crate::safety::lunar_value_is_safe(file_kind, pointer, incoming)
    {
        return Some(incoming.clone());
    }
    let mut value = if existing.is_number() {
        let raw = incoming.as_str()?;
        serde_json::from_str::<Value>(raw)
            .ok()
            .filter(Value::is_number)
            .or_else(|| Number::from_f64(raw.parse::<f64>().ok()?).map(Value::Number))?
    } else if existing.is_string() && incoming.is_number() {
        Value::String(incoming.to_string())
    } else {
        return None;
    };
    if crate::appearance::field(file_kind, pointer).is_some_and(|field| field.integer) {
        if let Some(number) = value.as_f64().filter(|number| {
            number.is_finite()
                && number.fract() == 0.0
                && (-4294967296.0..=4294967295.0).contains(number)
        }) {
            value = Value::from(number as i64);
        }
    }
    crate::safety::lunar_value_is_safe(file_kind, pointer, &value).then_some(value)
}

pub(crate) fn is_module_enabled_setting(setting: &Setting) -> bool {
    setting.source == "lunar"
        && setting.file_kind == "mods"
        && setting.value.is_boolean()
        && setting.pointer.starts_with('/')
        && setting.pointer.split('/').count() == 3
        && setting.pointer.ends_with("/enabled")
        && crate::safety::validate_setting(setting).is_ok()
}

pub(crate) fn can_insert_module_enabled(document: &Value, setting: &Setting) -> bool {
    if !is_module_enabled_setting(setting) || document.pointer(&setting.pointer).is_some() {
        return false;
    }
    setting
        .pointer
        .strip_suffix("/enabled")
        .and_then(|parent| document.pointer(parent))
        .is_some_and(Value::is_object)
}

pub(crate) fn can_insert_setting(document: &Value, setting: &Setting) -> bool {
    can_insert_module_enabled(document, setting)
        || (setting.source == "lunar"
            && crate::appearance::can_insert(
                document,
                &setting.file_kind,
                &setting.pointer,
                &setting.value,
            ))
}

pub(crate) fn replace_scalar_values(
    content: &str,
    changes: &BTreeMap<String, Value>,
) -> Result<String, String> {
    patch_scalar_values(content, changes, &BTreeMap::new())
}

fn patch_scalar_values(
    content: &str,
    changes: &BTreeMap<String, Value>,
    additions: &BTreeMap<String, Value>,
) -> Result<String, String> {
    let document = parse_document(content)?;
    if changes.len() + additions.len() > 10000 {
        return Err("The selected Lunar settings exceed the supported limit.".into());
    }
    for (pointer, value) in changes {
        let existing = document
            .pointer(pointer)
            .ok_or("A selected setting is unavailable in this Lunar profile.")?;
        if !same_value_type(existing, value) {
            return Err("A selected setting uses a different Lunar schema version.".into());
        }
    }
    let mut insertions = BTreeMap::<String, Value>::new();
    for (pointer, value) in additions {
        if !(value.is_boolean() || value.is_number() || value.is_string())
            || document.pointer(pointer).is_some()
            || changes.contains_key(pointer)
        {
            return Err("This Lunar field cannot be added.".into());
        }
        let (parent, missing) = missing_path(&document, pointer)?;
        let fields = insertions
            .entry(parent)
            .or_insert_with(|| Value::Object(Map::new()));
        insert_missing(fields, &missing, value)?;
    }
    let mut selected: BTreeSet<&str> = changes.keys().map(String::as_str).collect();
    selected.extend(insertions.keys().map(String::as_str));
    let mut tokens = ScalarTokens {
        text: content,
        offset: usize::from(content.starts_with('\u{feff}')) * 3,
        selected: &selected,
        spans: BTreeMap::new(),
    };
    tokens.value("", 0)?;
    let mut replacements = Vec::new();
    for (pointer, value) in changes {
        let span = tokens
            .spans
            .remove(pointer)
            .ok_or("The Lunar setting location could not be verified.")?;
        let value = serde_json::to_string(value)
            .map_err(|_| "The Lunar settings could not be prepared.".to_string())?;
        replacements.push((span, value));
    }
    for (parent, fields) in &insertions {
        let span = tokens
            .spans
            .remove(parent.as_str())
            .ok_or("The Lunar setting location could not be verified.")?;
        let comma = if document
            .pointer(parent)
            .unwrap()
            .as_object()
            .unwrap()
            .is_empty()
        {
            ""
        } else {
            ","
        };
        let value = serde_json::to_string(fields)
            .map_err(|_| "The Lunar settings could not be prepared.".to_string())?;
        let offset = span.end - 1;
        replacements.push((
            offset..offset,
            format!("{comma}{}", &value[1..value.len() - 1]),
        ));
    }
    replacements.sort_by_key(|replacement| std::cmp::Reverse(replacement.0.start));
    let mut output = content.to_owned();
    for (span, value) in replacements {
        output.replace_range(span, &value);
    }
    parse_document(&output)?;
    Ok(output)
}

fn missing_path(document: &Value, pointer: &str) -> Result<(String, Vec<String>), String> {
    let tokens: Vec<_> = pointer
        .strip_prefix('/')
        .ok_or("This Lunar field cannot be added.")?
        .split('/')
        .collect();
    let mut current = document;
    let mut parent = String::new();
    for (index, token) in tokens.iter().enumerate() {
        let object = current
            .as_object()
            .ok_or("This Lunar field cannot be added.")?;
        let key = token.replace("~1", "/").replace("~0", "~");
        let Some(child) = object.get(&key) else {
            let missing = tokens[index..]
                .iter()
                .map(|token| token.replace("~1", "/").replace("~0", "~"))
                .collect();
            return Ok((parent, missing));
        };
        current = child;
        parent.push('/');
        parent.push_str(token);
    }
    Err("This Lunar field cannot be added.".into())
}

fn insert_missing(target: &mut Value, tokens: &[String], value: &Value) -> Result<(), String> {
    let (key, children) = tokens
        .split_first()
        .ok_or("This Lunar field cannot be added.")?;
    let object = target
        .as_object_mut()
        .ok_or("This Lunar field cannot be added.")?;
    if children.is_empty() {
        if object.insert(key.clone(), value.clone()).is_some() {
            return Err("This Lunar field cannot be added.".into());
        }
        return Ok(());
    }
    let child = object
        .entry(key.clone())
        .or_insert_with(|| Value::Object(Map::new()));
    insert_missing(child, children, value)
}

struct ScalarTokens<'a> {
    text: &'a str,
    offset: usize,
    selected: &'a BTreeSet<&'a str>,
    spans: BTreeMap<String, Range<usize>>,
}

impl ScalarTokens<'_> {
    fn whitespace(&mut self) {
        while self
            .text
            .as_bytes()
            .get(self.offset)
            .is_some_and(u8::is_ascii_whitespace)
        {
            self.offset += 1;
        }
    }

    fn string(&mut self) -> Result<Range<usize>, String> {
        let start = self.offset;
        if self.text.as_bytes().get(start) != Some(&b'"') {
            return Err("The Lunar setting location could not be verified.".into());
        }
        self.offset += 1;
        while let Some(byte) = self.text.as_bytes().get(self.offset) {
            match byte {
                b'\\' => self.offset += 2,
                b'"' => {
                    self.offset += 1;
                    return Ok(start..self.offset);
                }
                _ => self.offset += 1,
            }
        }
        Err("The Lunar setting location could not be verified.".into())
    }

    fn value(&mut self, pointer: &str, depth: usize) -> Result<(), String> {
        if depth > 128 {
            return Err("The Lunar settings structure is too deep.".into());
        }
        self.whitespace();
        let start = self.offset;
        match self.text.as_bytes().get(self.offset) {
            Some(b'{') => {
                self.offset += 1;
                self.whitespace();
                while self.text.as_bytes().get(self.offset) != Some(&b'}') {
                    let span = self.string()?;
                    let key: String = serde_json::from_str(&self.text[span]).map_err(|_| {
                        "The Lunar setting location could not be verified.".to_string()
                    })?;
                    self.whitespace();
                    if self.text.as_bytes().get(self.offset) != Some(&b':') {
                        return Err("The Lunar setting location could not be verified.".into());
                    }
                    self.offset += 1;
                    let token = key.replace('~', "~0").replace('/', "~1");
                    self.value(&format!("{pointer}/{token}"), depth + 1)?;
                    self.whitespace();
                    match self.text.as_bytes().get(self.offset) {
                        Some(b',') => {
                            self.offset += 1;
                            self.whitespace();
                        }
                        Some(b'}') => break,
                        _ => return Err("The Lunar setting location could not be verified.".into()),
                    }
                }
                self.offset += 1;
                if self.selected.contains(pointer) {
                    self.spans.insert(pointer.into(), start..self.offset);
                }
            }
            Some(b'[') => {
                self.offset += 1;
                self.whitespace();
                let mut index = 0;
                while self.text.as_bytes().get(self.offset) != Some(&b']') {
                    self.value(&format!("{pointer}/{index}"), depth + 1)?;
                    index += 1;
                    self.whitespace();
                    match self.text.as_bytes().get(self.offset) {
                        Some(b',') => {
                            self.offset += 1;
                            self.whitespace();
                        }
                        Some(b']') => break,
                        _ => return Err("The Lunar setting location could not be verified.".into()),
                    }
                }
                self.offset += 1;
            }
            Some(b'"') => {
                let span = self.string()?;
                if self.selected.contains(pointer) {
                    self.spans.insert(pointer.into(), span);
                }
            }
            Some(_) => {
                while self
                    .text
                    .as_bytes()
                    .get(self.offset)
                    .is_some_and(|byte| !byte.is_ascii_whitespace() && !b",}]".contains(byte))
                {
                    self.offset += 1;
                }
                if start == self.offset {
                    return Err("The Lunar setting location could not be verified.".into());
                }
                if self.selected.contains(pointer) {
                    self.spans.insert(pointer.into(), start..self.offset);
                }
            }
            None => return Err("The Lunar setting location could not be verified.".into()),
        }
        Ok(())
    }
}

pub fn same_value_type(left: &Value, right: &Value) -> bool {
    (left.is_boolean() && right.is_boolean())
        || (left.is_number() && right.is_number())
        || (left.is_string() && right.is_string())
}

pub fn describe_pointer(file_kind: &str, pointer: &str) -> (String, String, String) {
    let parts: Vec<&str> = pointer.trim_start_matches('/').split('/').collect();
    let leaf = parts.last().copied().unwrap_or("");
    let label = if leaf == "value" && parts.len() > 1 {
        crate::minecraft::humanize(parts[parts.len() - 2])
    } else {
        crate::minecraft::humanize(leaf)
    };
    if file_kind != "mods" {
        let category = if file_kind == "controls" {
            "Keybinds"
        } else if file_kind == "performance" {
            "Performance"
        } else {
            "General"
        };
        return (category.into(), category.into(), label);
    }
    let module = parts.first().copied().unwrap_or("");
    let hud_modules = [
        "FPS",
        "CPS",
        "ARMORSTATUS",
        "KEYSTROKES",
        "COORDINATES",
        "DAY_COUNTER",
        "POTION_EFFECTS",
        "DIRECTION_HUD",
        "SCOREBOARD",
        "PING",
        "CLOCK",
        "STOPWATCH",
        "PLAYTIME",
        "MEMORY",
        "COMBO",
        "REACH_DISPLAY",
        "ITEM_COUNTER",
        "ITEM_TRACKER",
        "TNT_COUNTDOWN",
        "BOSSBAR",
        "PVP_INFO",
        "PACK_DISPLAY",
        "SATURATION",
        "TOTEM_COUNTER",
        "HORSE_STATS",
        "AUDIO_SUBTITLES",
    ];
    let category = if leaf.to_ascii_lowercase().contains("keybind") {
        "Keybinds"
    } else if hud_modules.contains(&module) {
        "HUD"
    } else {
        "Mods"
    };
    let group = match module {
        "FPS" | "CPS" | "FOV" | "SBA" => module.to_owned(),
        "ARMORSTATUS" => "Armor status".into(),
        "KEYSTROKES" => "Keystrokes".into(),
        "BOSSBAR" => "Boss bar".into(),
        _ => module
            .split('_')
            .map(|word| {
                let lower = word.to_ascii_lowercase();
                crate::minecraft::humanize(&lower)
            })
            .collect::<Vec<_>>()
            .join(" "),
    };
    let label = if parts.len() > 2 && parts[1] != "options" {
        format!(
            "{} · {}",
            crate::minecraft::humanize(&parts[1].to_ascii_lowercase()),
            label
        )
    } else {
        label
    };
    (category.into(), group, label)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn only_verified_scalar_fields_are_exposed() {
        let data = r#"{"FPS":{"enabled":true,"x":0.2,"y":0.4,"futureData":{"token":"private"}},"WAYPOINTS":{"enabled":true},"SERVER_ADDRESS":{"x":0.1},"version":99}"#;
        let settings = parse_settings(data, "mods", "default").unwrap();
        assert_eq!(settings.len(), 4);
        assert!(settings
            .iter()
            .all(|setting| setting.pointer.starts_with("/FPS/")
                || setting.pointer == "/WAYPOINTS/enabled"));
    }

    #[test]
    fn partial_merge_preserves_every_unknown_field() {
        let data = r#"{"FPS":{"enabled":true,"x":0.2,"unknown":[1,"keep"]},"future":{"marker":"preserved"}}"#;
        let mut settings = parse_settings(data, "mods", "default").unwrap();
        settings.retain(|setting| setting.pointer == "/FPS/x");
        settings[0].value = json!(0.6);
        let merged = parse_document(&merge_settings(data, &settings).unwrap()).unwrap();
        assert_eq!(merged.pointer("/FPS/x"), Some(&json!(0.6)));
        assert_eq!(merged.pointer("/FPS/unknown"), Some(&json!([1, "keep"])));
        assert_eq!(merged.pointer("/future/marker"), Some(&json!("preserved")));
        assert!(merge_settings(r#"{"FPS":{"enabled":false}}"#, &settings).is_err());
    }

    #[test]
    fn scalar_patch_preserves_precision_escapes_and_whitespace() {
        let data = "\u{feff} {\n  \"F\\u0050S\": {\"x\":0.2, \"enabled\":true},\n  \"unknown/key~\": [18446744073709551617, 0.12345678901234567890123456789, {\"text\":\"} ] \\\" escaped\"}]\n}\n";
        let mut settings = parse_settings(data, "mods", "default").unwrap();
        settings.retain(|setting| setting.pointer == "/FPS/x" || setting.pointer == "/FPS/enabled");
        for setting in &mut settings {
            setting.value = if setting.pointer.ends_with("/x") {
                json!(0.625)
            } else {
                json!(false)
            };
        }
        let output = merge_settings(data, &settings).unwrap();
        assert_eq!(
            output,
            data.replace("\"x\":0.2", "\"x\":0.625")
                .replace("\"enabled\":true", "\"enabled\":false")
        );
    }

    #[test]
    fn duplicate_and_corrupt_json_are_rejected() {
        assert!(parse_document(r#"{"FPS":{"x":0.2,"x":0.3}}"#).is_err());
        assert!(parse_document("").is_err());
        assert!(parse_document("[]").is_err());
    }

    #[test]
    fn verified_missing_module_enabled_fields_are_inserted_without_reformatting() {
        let data = "\u{feff}{\r\n \"F\\u0050S\": {\"x\":0.2, \"unknown\":0.123456789012345678901},\r\n \"CPS\": {  }, \"future\":{\"token\":\"unshared\"}\r\n}\r\n";
        let settings = parse_settings(
            r#"{"FPS":{"enabled":true,"x":0.6},"CPS":{"enabled":false}}"#,
            "mods",
            "source",
        )
        .unwrap();
        assert!(settings
            .iter()
            .filter(|setting| setting.pointer.ends_with("/enabled"))
            .all(|setting| can_insert_module_enabled(&parse_document(data).unwrap(), setting)));
        assert_eq!(
            merge_settings(data, &settings).unwrap(),
            "\u{feff}{\r\n \"F\\u0050S\": {\"x\":0.6, \"unknown\":0.123456789012345678901,\"enabled\":true},\r\n \"CPS\": {  \"enabled\":false}, \"future\":{\"token\":\"unshared\"}\r\n}\r\n"
        );
    }

    #[test]
    fn sparse_module_updates_reject_unknown_modules_types_and_missing_parents() {
        let enabled = parse_settings(r#"{"FPS":{"enabled":true}}"#, "mods", "source").unwrap();
        for data in [
            r#"{}"#,
            r#"{"FPS":null}"#,
            r#"{"FPS":[]}"#,
            r#"{"FPS":{"enabled":"true"}}"#,
        ] {
            assert!(!can_insert_module_enabled(
                &parse_document(data).unwrap(),
                &enabled[0]
            ));
            assert!(merge_settings(data, &enabled).is_err());
        }
        let mut unknown = enabled[0].clone();
        unknown.pointer = "/UNVERIFIED_MOD/enabled".into();
        assert!(!can_insert_module_enabled(
            &parse_document(r#"{"UNVERIFIED_MOD":{}}"#).unwrap(),
            &unknown
        ));
        assert!(merge_settings(r#"{"UNVERIFIED_MOD":{}}"#, &[unknown]).is_err());
        let scalar = parse_settings(r#"{"FPS":{"x":0.6}}"#, "mods", "source").unwrap();
        assert!(merge_settings(r#"{"FPS":{}}"#, &scalar).is_err());
        assert!(
            merge_settings(r#"{"FPS":{}}"#, &[enabled[0].clone(), enabled[0].clone()]).is_err()
        );
    }

    #[test]
    fn appearance_import_creates_nested_fields_and_preserves_unrelated_tokens() {
        let incoming = parse_settings(
            r#"{"KEYSTROKES":{"options":{"backgroundColor":{"value":1074864691,"chroma":true},"backgroundPressedColor":{"value":-16711936}}}}"#,
            "mods",
            "source",
        )
        .unwrap();
        assert_eq!(incoming.len(), 3);
        let original = "\u{feff}{\r\n\"KEYSTROKES\": { \"enabled\": false, \"x\":42.25, \"unknown\":0.9000000000000000001 },\r\n\"outside\":{\"data\":[1,true]}\r\n}\r\n";
        let output = merge_settings(original, &incoming).unwrap();
        assert!(output.starts_with('\u{feff}'));
        assert!(output.ends_with("}\r\n"));
        assert!(
            output.contains("\"enabled\": false, \"x\":42.25, \"unknown\":0.9000000000000000001 ")
        );
        assert!(output.contains("\"outside\":{\"data\":[1,true]}"));
        let document = parse_document(&output).unwrap();
        for setting in &incoming {
            assert_eq!(document.pointer(&setting.pointer), Some(&setting.value));
        }
        assert_eq!(merge_settings(&output, &incoming).unwrap(), output);
        let next = parse_settings(
            r#"{"KEYSTROKES":{"options":{"backgroundColor":{"value":0,"chroma":false},"backgroundPressedColor":{"value":-1}}}}"#,
            "mods",
            "source",
        )
        .unwrap();
        let second = merge_settings(&output, &next).unwrap();
        let document = parse_document(&second).unwrap();
        for setting in &next {
            assert_eq!(document.pointer(&setting.pointer), Some(&setting.value));
        }
        assert!(second.contains("\"unknown\":0.9000000000000000001"));
    }

    #[test]
    fn appearance_insert_rejects_missing_components_and_damaged_parents() {
        let incoming = parse_settings(
            r#"{"KEYSTROKES":{"options":{"backgroundColor":{"value":-1}}}}"#,
            "mods",
            "source",
        )
        .unwrap();
        for original in [
            r#"{}"#,
            r#"{"KEYSTROKES":null}"#,
            r#"{"KEYSTROKES":{"options":null}}"#,
            r#"{"KEYSTROKES":{"options":[]}}"#,
            r#"{"KEYSTROKES":{"options":{"backgroundColor":"broken"}}}"#,
            r#"{"KEYSTROKES":{"options":{"backgroundColor":{"value":"broken"}}}}"#,
        ] {
            assert!(merge_settings(original, &incoming).is_err());
        }
        let mut unknown = incoming[0].clone();
        unknown.pointer = "/KEYSTROKES/options/accountColor/value".into();
        assert!(merge_settings(r#"{"KEYSTROKES":{}}"#, &[unknown]).is_err());
        assert!(merge_settings(
            r#"{"KEYSTROKES":{}}"#,
            &[incoming[0].clone(), incoming[0].clone()],
        )
        .is_err());
    }

    #[test]
    fn appearance_export_includes_defaults_without_guessing_damaged_values() {
        let source = r#"{"KEYSTROKES":{"options":{"backgroundColor":{"value":0,"chroma":true}}}}"#;
        let settings = parse_settings_with_defaults(source, "mods", "source").unwrap();
        let values: BTreeMap<_, _> = settings
            .iter()
            .map(|setting| (setting.pointer.as_str(), &setting.value))
            .collect();
        assert_eq!(
            values["/KEYSTROKES/options/backgroundColor/value"],
            &json!(0)
        );
        assert_eq!(
            values["/KEYSTROKES/options/backgroundColor/chroma"],
            &json!(true)
        );
        assert_eq!(values["/KEYSTROKES/options/textColor/value"], &json!(-1));
        assert_eq!(
            values["/KEYSTROKES/options/textPressedColor/value"],
            &json!(-16777216)
        );
        assert_eq!(
            values["/KEYSTROKES/options/textColor/chromaSpeed"],
            &json!(40)
        );
        assert_eq!(
            values["/KEYSTROKES/options/textColor/chromaType"],
            &json!("wave")
        );
        assert_eq!(values.len(), settings.len());
        for damaged in [
            r#"{"KEYSTROKES":{"options":{"textColor":{"value":"invalid"}}}}"#,
            r#"{"KEYSTROKES":{"options":{"textColor":{"value":2147483648}}}}"#,
            r#"{"KEYSTROKES":{"options":{"textColor":{"chromaType":"unknown"}}}}"#,
            r#"{"KEYSTROKES":{"options":{"textColor":null}}}"#,
        ] {
            let settings = parse_settings_with_defaults(damaged, "mods", "source").unwrap();
            assert!(!settings.iter().any(|setting| {
                setting
                    .pointer
                    .starts_with("/KEYSTROKES/options/textColor/")
                    && parse_document(damaged)
                        .unwrap()
                        .pointer(&setting.pointer)
                        .is_none()
            }));
        }
        assert!(parse_settings_with_defaults("{}", "mods", "source")
            .unwrap()
            .is_empty());
    }

    #[test]
    fn incomplete_keybinds_only_fill_verified_defaults_without_masking_damaged_members() {
        let source =
            r#"{"COORDINATES":{"options":{"copyCoords":{"value":"KEY_C","control":true}}}}"#;
        let settings = parse_settings_with_defaults(source, "mods", "source").unwrap();
        let values: BTreeMap<_, _> = settings
            .iter()
            .map(|setting| (setting.pointer.as_str(), &setting.value))
            .collect();
        assert_eq!(
            values["/COORDINATES/options/copyCoords/value"],
            &json!("KEY_C")
        );
        assert_eq!(
            values["/COORDINATES/options/copyCoords/modifier"],
            &json!("KEY_NONE")
        );
        assert_eq!(values["/COORDINATES/options/copyCoords/alt"], &json!(false));
        assert_eq!(
            values["/COORDINATES/options/copyCoords/shift"],
            &json!(false)
        );
        assert_eq!(
            values["/COORDINATES/options/copyCoords/control"],
            &json!(true)
        );
        for damaged in [
            r#"{"COORDINATES":{"options":{"copyCoords":{"value":"private command"}}}}"#,
            r#"{"COORDINATES":{"options":{"copyCoords":{"modifier":7}}}}"#,
            r#"{"COORDINATES":{"options":{"copyCoords":{"alt":"false"}}}}"#,
            r#"{"COORDINATES":{"options":{"copyCoords":null}}}"#,
        ] {
            let document = parse_document(damaged).unwrap();
            let settings = parse_settings_with_defaults(damaged, "mods", "source").unwrap();
            assert!(!settings.iter().any(|setting| {
                setting
                    .pointer
                    .starts_with("/COORDINATES/options/copyCoords/")
                    && document.pointer(&setting.pointer).is_none()
            }));
            let incoming = parse_settings(source, "mods", "source").unwrap();
            assert!(merge_settings(damaged, &incoming).is_err());
        }
    }

    #[test]
    fn primary_numeric_options_preserve_numbers_and_reject_string_coercion() {
        let source = parse_settings(r#"{"TOGGLE_SNEAK":{"options":{"flyBoostAmount":3}},"TIME_CHANGER":{"options":{"timeChangerTime":4}}}"#, "mods", "source").unwrap();
        assert_eq!(source.len(), 2);
        let target = r#"{"TOGGLE_SNEAK":{"options":{"flyBoostAmount":2}},"TIME_CHANGER":{"options":{"timeChangerTime":2}},"unknown":0.123456789012345678901}"#;
        let output = merge_settings(target, &source).unwrap();
        assert_eq!(
            output,
            target
                .replace("\"flyBoostAmount\":2", "\"flyBoostAmount\":3")
                .replace("\"timeChangerTime\":2", "\"timeChangerTime\":4")
        );
        let invalid = r#"{"TOGGLE_SNEAK":{"options":{"flyBoostAmount":"3"}},"TIME_CHANGER":{"options":{"timeChangerTime":"4"}}}"#;
        assert!(parse_settings(invalid, "mods", "source")
            .unwrap()
            .is_empty());
        assert!(merge_settings(invalid, &source).is_err());
        assert_eq!(
            value_for_existing_type("mods", "/FPS/x", &json!(0.2), &json!("0.3")),
            None
        );
        assert_eq!(
            value_for_existing_type(
                "mods",
                "/TIME_CHANGER/options/timeChangerTime",
                &json!("2"),
                &json!(1000000)
            ),
            None
        );
        assert_eq!(
            value_for_existing_type(
                "mods",
                "/TIME_CHANGER/options/timeChangerTime",
                &json!(2),
                &json!("NaN")
            ),
            None
        );
    }

    #[test]
    fn reviewed_module_toggles_share_no_macro_location_or_server_payload() {
        let settings = parse_settings(r#"{"WAYPOINTS":{"enabled":true,"waypoints":[{"x":1,"server":"unshared.invalid"}]},"AUTO_TEXT_ACTIONS":{"enabled":false,"actions":["unshared"]},"SERVER_ADDRESS":{"enabled":true,"address":"unshared.invalid"},"OVERLAY_MOD":{"enabled":true},"ITEM_CUSTOMIZER":{"enabled":false},"UNVERIFIED_MOD":{"enabled":true}}"#, "mods", "source").unwrap();
        assert_eq!(settings.len(), 5);
        assert!(settings
            .iter()
            .all(|setting| setting.value.is_boolean() && setting.pointer.ends_with("/enabled")));
        assert!(!settings
            .iter()
            .any(|setting| setting.pointer.starts_with("/UNVERIFIED_MOD/")));
    }
}
