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
    if !matches!(file_kind, "mods" | "general" | "controls" | "performance") {
        return Err("This Lunar settings file is unsupported.".into());
    }
    let value = parse_document(content)?;
    let mut settings = Vec::new();
    collect(&value, "", file_kind, profile, &mut settings, 0);
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
    for setting in settings {
        crate::safety::validate_setting(setting)?;
        if setting.source != "lunar" || !seen.insert(setting.pointer.as_str()) {
            return Err("The selected Lunar settings are unsupported or duplicated.".into());
        }
        let existing = document
            .pointer(&setting.pointer)
            .ok_or("A selected setting is unavailable in this Lunar profile.")?;
        if !same_value_type(existing, &setting.value) {
            return Err("A selected setting uses a different Lunar schema version.".into());
        }
    }
    let mut tokens = ScalarTokens {
        text: content,
        offset: usize::from(content.starts_with('\u{feff}')) * 3,
        selected: &seen,
        spans: BTreeMap::new(),
    };
    tokens.value("", 0)?;
    let mut replacements = Vec::new();
    for setting in settings {
        let span = tokens
            .spans
            .remove(&setting.pointer)
            .ok_or("The Lunar setting location could not be verified.")?;
        let value = serde_json::to_string(&setting.value)
            .map_err(|_| "The Lunar settings could not be prepared.".to_string())?;
        replacements.push((span, value));
    }
    replacements.sort_by_key(|replacement| std::cmp::Reverse(replacement.0.start));
    let mut output = content.to_owned();
    for (span, value) in replacements {
        output.replace_range(span, &value);
    }
    parse_document(&output)?;
    Ok(output)
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
        assert_eq!(settings.len(), 3);
        assert!(settings
            .iter()
            .all(|setting| setting.pointer.starts_with("/FPS/")));
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
}
