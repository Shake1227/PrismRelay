use crate::model::{setting_id, Setting};
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

#[derive(Clone, Debug, PartialEq)]
pub struct OptionsLine {
    pub raw: String,
    pub key: Option<String>,
    pub value: Option<String>,
}

#[derive(Clone, Debug)]
pub struct OptionsDocument {
    pub lines: Vec<OptionsLine>,
    pub newline: String,
    pub trailing_newline: bool,
    pub byte_order_mark: bool,
}

pub fn parse_options(content: &str) -> Result<OptionsDocument, String> {
    if content.len() > 8 * 1024 * 1024 || content.contains('\0') {
        return Err("This Minecraft settings file is too large or damaged.".into());
    }
    let byte_order_mark = content.starts_with('\u{feff}');
    let content = content.strip_prefix('\u{feff}').unwrap_or(content);
    let newline = if content.contains("\r\n") {
        "\r\n"
    } else {
        "\n"
    };
    let trailing_newline = content.ends_with('\n');
    let lines = content
        .split_terminator('\n')
        .map(|line| {
            let line = line.strip_suffix('\r').unwrap_or(line);
            let parsed = line.split_once(':').filter(|(key, _)| {
                !key.is_empty()
                    && key.len() <= 128
                    && key
                        .chars()
                        .all(|c| c.is_ascii_alphanumeric() || "_.-".contains(c))
            });
            OptionsLine {
                raw: line.into(),
                key: parsed.map(|(key, _)| key.into()),
                value: parsed.map(|(_, value)| value.into()),
            }
        })
        .collect();
    Ok(OptionsDocument {
        lines,
        newline: newline.into(),
        trailing_newline,
        byte_order_mark,
    })
}

pub fn parse_settings(
    content: &str,
    file_kind: &str,
    profile: &str,
) -> Result<Vec<Setting>, String> {
    let document = parse_options(content)?;
    let mut entries = BTreeMap::new();
    for line in document.lines {
        if let (Some(key), Some(value)) = (line.key, line.value) {
            entries.insert(key, value);
        }
    }
    let mut settings = Vec::new();
    for (key, raw) in entries {
        let Some((category, group, label)) = describe_key(&key) else {
            continue;
        };
        let setting = Setting {
            id: setting_id("minecraft", profile, file_kind, &key),
            label,
            source: "minecraft".into(),
            category: category.into(),
            group: group.into(),
            file_kind: file_kind.into(),
            profile: profile.into(),
            pointer: key,
            value: Value::String(raw),
        };
        if crate::safety::validate_setting(&setting).is_ok() {
            settings.push(setting);
        }
    }
    Ok(settings)
}

pub fn parse_file(
    content: &str,
    file_kind: &str,
    profile: &str,
    path: &Path,
) -> Result<Vec<Setting>, String> {
    match path.file_name().and_then(|name| name.to_str()) {
        Some("optionsLC.txt") if file_kind == "options" => parse_lunar_options(content, profile),
        Some("options.txt") if file_kind == "options" => {
            parse_settings(content, file_kind, profile)
        }
        _ => Err("This Minecraft settings file is unsupported.".into()),
    }
}

pub fn merge_file(content: &str, settings: &[Setting], path: &Path) -> Result<String, String> {
    match path.file_name().and_then(|name| name.to_str()) {
        Some("optionsLC.txt") => merge_lunar_options(content, settings),
        Some("options.txt") => merge_options(content, settings),
        _ => Err("This Minecraft settings file is unsupported.".into()),
    }
}

pub fn validate_file_setting(setting: &Setting, path: &Path) -> Result<(), String> {
    crate::safety::validate_setting(setting)?;
    if setting.source != "minecraft" || setting.file_kind != "options" {
        return Err("This selection is not a supported Minecraft settings change.".into());
    }
    match path.file_name().and_then(|name| name.to_str()) {
        Some("optionsLC.txt") => {
            lunar_option_value(setting)?;
            Ok(())
        }
        Some("options.txt") => Ok(()),
        _ => Err("This Minecraft settings file is unsupported.".into()),
    }
}

pub fn parse_lunar_options(content: &str, profile: &str) -> Result<Vec<Setting>, String> {
    let document = crate::lunar::parse_document(content)?;
    let mut settings = Vec::new();
    for (key, value) in document.as_object().unwrap() {
        let Some((category, group, label)) = describe_key(key) else {
            continue;
        };
        let Some(raw) = value.as_str() else {
            continue;
        };
        let raw = if key == "fov" {
            let Some(degrees) = bounded_number(raw, 30.0, 110.0) else {
                continue;
            };
            ((degrees - 70.0) / 40.0).to_string()
        } else {
            raw.to_owned()
        };
        let setting = Setting {
            id: setting_id("minecraft", profile, "options", key),
            label,
            source: "minecraft".into(),
            category: category.into(),
            group: group.into(),
            file_kind: "options".into(),
            profile: profile.into(),
            pointer: key.clone(),
            value: Value::String(raw),
        };
        if crate::safety::validate_setting(&setting).is_ok() {
            settings.push(setting);
        }
    }
    Ok(settings)
}

fn bounded_number(raw: &str, min: f64, max: f64) -> Option<f64> {
    if raw.len() > 32 {
        return None;
    }
    raw.parse::<f64>()
        .ok()
        .filter(|value| value.is_finite() && (min..=max).contains(value))
}

fn lunar_option_value(setting: &Setting) -> Result<Value, String> {
    let raw = setting
        .value
        .as_str()
        .ok_or("This Lunar game setting uses an unsupported value type.")?;
    if setting.pointer == "fov" {
        let ratio = bounded_number(raw, -1.0, 1.0)
            .ok_or("This FOV value is unavailable in this Lunar version.")?;
        let degrees = 70.0 + 40.0 * ratio;
        if !(30.0..=110.0).contains(&degrees) {
            return Err("This FOV value is unavailable in this Lunar version.".into());
        }
        Ok(Value::String(degrees.to_string()))
    } else {
        Ok(setting.value.clone())
    }
}

pub fn merge_lunar_options(content: &str, settings: &[Setting]) -> Result<String, String> {
    let document = crate::lunar::parse_document(content)?;
    let available: BTreeSet<String> = parse_lunar_options(content, "validation")?
        .into_iter()
        .map(|setting| setting.pointer)
        .collect();
    let mut changes = BTreeMap::new();
    for setting in settings {
        validate_file_setting(setting, Path::new("optionsLC.txt"))?;
        if !available.contains(&setting.pointer)
            || !document.get(&setting.pointer).is_some_and(Value::is_string)
        {
            return Err("A selected setting is unavailable in this Lunar version.".into());
        }
        let pointer = format!("/{}", setting.pointer.replace('~', "~0").replace('/', "~1"));
        if changes
            .insert(pointer, lunar_option_value(setting)?)
            .is_some()
        {
            return Err("The selected settings contain a duplicate entry.".into());
        }
    }
    crate::lunar::replace_scalar_values(content, &changes)
}

pub fn merge_options(content: &str, settings: &[Setting]) -> Result<String, String> {
    let document = parse_options(content)?;
    let present: BTreeSet<&str> = document
        .lines
        .iter()
        .filter_map(|line| line.key.as_deref())
        .collect();
    let mut changes = BTreeMap::new();
    for setting in settings {
        crate::safety::validate_setting(setting)?;
        if setting.source != "minecraft" || setting.file_kind != "options" {
            return Err("This selection is not a supported Minecraft settings change.".into());
        }
        if !present.contains(setting.pointer.as_str()) {
            return Err("A selected setting is unavailable in this Minecraft version.".into());
        }
        if changes
            .insert(setting.pointer.as_str(), setting.value.as_str().unwrap())
            .is_some()
        {
            return Err("The selected settings contain a duplicate entry.".into());
        }
    }
    let mut output = String::new();
    if document.byte_order_mark {
        output.push('\u{feff}');
    }
    for (index, line) in document.lines.iter().enumerate() {
        if index > 0 {
            output.push_str(&document.newline);
        }
        if let Some(value) = line.key.as_deref().and_then(|key| changes.get(key)) {
            output.push_str(line.key.as_ref().unwrap());
            output.push(':');
            output.push_str(value);
        } else {
            output.push_str(&line.raw);
        }
    }
    if document.trailing_newline {
        output.push_str(&document.newline);
    }
    Ok(output)
}

pub fn describe_key(key: &str) -> Option<(&'static str, &'static str, String)> {
    let (category, group) = if let Some(binding) = key.strip_prefix("key_") {
        let group = match binding {
            "key.forward" | "key.back" | "key.left" | "key.right" | "key.jump" | "key.sneak"
            | "key.sprint" => "Movement",
            "key.attack" | "key.use" | "key.pickItem" | "key.drop" | "key.swapOffhand" => {
                "Gameplay"
            }
            "key.inventory"
            | "key.hotbar.1"
            | "key.hotbar.2"
            | "key.hotbar.3"
            | "key.hotbar.4"
            | "key.hotbar.5"
            | "key.hotbar.6"
            | "key.hotbar.7"
            | "key.hotbar.8"
            | "key.hotbar.9"
            | "key.saveToolbarActivator"
            | "key.loadToolbarActivator" => "Inventory",
            "key.chat" | "key.playerlist" | "key.command" | "key.socialInteractions" => {
                "Multiplayer"
            }
            "key.screenshot"
            | "key.togglePerspective"
            | "key.smoothCamera"
            | "key.fullscreen"
            | "key.spectatorOutlines"
            | "key.advancements" => "Miscellaneous",
            "Freelook"
            | "of.key.zoom"
            | "key.lunarclient.zoom"
            | "key.lunarclient.freelook"
            | "key.lunarclient.menu"
            | "key.lunarclient.toggleSprint"
            | "key.lunarclient.waypoint" => "Lunar keybinds",
            _ => return None,
        };
        return Some((
            "Controls",
            group,
            humanize(binding.trim_start_matches("key.")),
        ));
    } else if MOUSE_KEYS.contains(&key) {
        ("Mouse", "Mouse")
    } else if VIDEO_KEYS.contains(&key) {
        ("Video", "Display")
    } else if AUDIO_KEYS.contains(&key) {
        ("Audio", "Sound")
    } else if CHAT_KEYS.contains(&key) {
        ("Chat", "Chat")
    } else if ACCESSIBILITY_KEYS.contains(&key) {
        ("Accessibility", "Accessibility")
    } else if matches!(key, "resourcePacks" | "incompatibleResourcePacks") {
        ("Resource Packs", "Resource Packs")
    } else if key == "lang" {
        ("Language", "Language")
    } else if GENERAL_KEYS.contains(&key) {
        ("General", "General")
    } else {
        return None;
    };
    Some((
        category,
        group,
        humanize(key.trim_start_matches("soundCategory_")),
    ))
}

pub fn humanize(text: &str) -> String {
    let mut output = String::new();
    for (index, ch) in text.chars().enumerate() {
        if ch == '_' || ch == '.' || ch == '-' {
            if !output.ends_with(' ') {
                output.push(' ');
            }
        } else {
            if ch.is_ascii_uppercase() && index > 0 && !output.ends_with(' ') {
                output.push(' ');
            }
            output.push(ch);
        }
    }
    let mut chars = output.chars();
    match chars.next() {
        Some(ch) => ch.to_uppercase().collect::<String>() + chars.as_str(),
        None => output,
    }
}

pub const MOUSE_KEYS: &[&str] = &[
    "mouseSensitivity",
    "rawMouseInput",
    "mouseWheelSensitivity",
    "discrete_mouse_scroll",
    "invertYMouse",
    "touchscreen",
];
pub const VIDEO_KEYS: &[&str] = &[
    "fov",
    "fovEffectScale",
    "gamma",
    "renderDistance",
    "simulationDistance",
    "maxFps",
    "enableVsync",
    "fullscreen",
    "fullscreenResolution",
    "overrideWidth",
    "overrideHeight",
    "guiScale",
    "graphicsMode",
    "fancyGraphics",
    "renderClouds",
    "clouds",
    "particles",
    "entityShadows",
    "biomeBlendRadius",
    "mipmapLevels",
    "ao",
    "bobView",
    "entityDistanceScaling",
    "prioritizeChunkUpdates",
    "chunkBuilder",
    "useVbo",
    "fastRender",
    "useNativeTransport",
    "glDebugVerbosity",
];
pub const AUDIO_KEYS: &[&str] = &[
    "soundCategory_master",
    "soundCategory_music",
    "soundCategory_record",
    "soundCategory_weather",
    "soundCategory_block",
    "soundCategory_hostile",
    "soundCategory_neutral",
    "soundCategory_player",
    "soundCategory_ambient",
    "soundCategory_voice",
    "soundCategory_ui",
    "directionalAudio",
];
pub const CHAT_KEYS: &[&str] = &[
    "chatVisibility",
    "chatColors",
    "chatLinks",
    "chatLinksPrompt",
    "chatOpacity",
    "chatScale",
    "chatWidth",
    "chatHeightFocused",
    "chatHeightUnfocused",
    "chatLineSpacing",
    "chatDelay",
    "chatBackgroundOpacity",
    "textBackgroundOpacity",
    "backgroundForChatOnly",
    "autoSuggestions",
    "hideMatchedNames",
    "onlyShowSecureChat",
];
pub const ACCESSIBILITY_KEYS: &[&str] = &[
    "narrator",
    "showSubtitles",
    "highContrast",
    "autoJump",
    "toggleCrouch",
    "toggleSprint",
    "darknessEffectScale",
    "screenEffectScale",
    "damageTiltStrength",
    "glintStrength",
    "glintSpeed",
    "notificationDisplayTime",
    "panoramaScrollSpeed",
    "hideLightningFlashes",
    "monochromeLogo",
    "menuBackgroundBlurriness",
    "reducedDebugInfo",
    "accessibilityOnboarded",
];
pub const GENERAL_KEYS: &[&str] = &[
    "attackIndicator",
    "mainHand",
    "heldItemTooltips",
    "advancedItemTooltips",
    "pauseOnLostFocus",
    "hideServerAddress",
    "realmsNotifications",
    "showAutosaveIndicator",
    "syncChunkWrites",
    "modelPart_cape",
    "modelPart_jacket",
    "modelPart_left_sleeve",
    "modelPart_right_sleeve",
    "modelPart_left_pants_leg",
    "modelPart_right_pants_leg",
    "modelPart_hat",
    "tutorialStep",
    "skipMultiplayerWarning",
    "skipRealms32bitWarning",
    "operatorItemsTab",
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn preserves_unknown_lines_and_first_separator() {
        let parsed = parse_options("fov:0.25\nfutureKey:abc:def\nmalformed\n\n").unwrap();
        assert_eq!(parsed.lines[1].value.as_deref(), Some("abc:def"));
        assert_eq!(parsed.lines[2].key, None);
        let mut selected = parse_settings("fov:0.25\n", "options", "default").unwrap();
        selected[0].value = Value::String("0.5".into());
        assert_eq!(
            merge_options("fov:0.25\nfutureKey:abc:def\nmalformed\n\n", &selected).unwrap(),
            "fov:0.5\nfutureKey:abc:def\nmalformed\n\n"
        );
    }

    #[test]
    fn empty_and_damaged_files() {
        assert!(parse_options("").unwrap().lines.is_empty());
        assert!(parse_options("fov:\0").is_err());
        assert!(parse_settings(
            "lastServer:private.example\nfutureSecret:1\n",
            "options",
            "default"
        )
        .unwrap()
        .is_empty());
    }

    #[test]
    fn duplicate_keys_change_together_and_crlf_is_retained() {
        let settings = parse_settings("guiScale:2", "options", "default").unwrap();
        assert_eq!(
            merge_options(
                "\u{feff}guiScale:0\r\nfuture:keep\r\nguiScale:1\r\n",
                &settings
            )
            .unwrap(),
            "\u{feff}guiScale:2\r\nfuture:keep\r\nguiScale:2\r\n"
        );
    }

    #[test]
    fn legacy_and_modern_keybinds_use_recognized_key_grammars() {
        let settings = parse_settings("key_key.forward:17\nkey_key.jump:key.keyboard.space\nkey_Freelook:key.keyboard.r\nkey_of.key.zoom:key.keyboard.c\nkey_key.futureSecret:key.keyboard.password\n", "options", "default").unwrap();
        assert_eq!(settings.len(), 4);
        assert_eq!(
            settings
                .iter()
                .filter(|setting| setting.group == "Lunar keybinds")
                .count(),
            2
        );
    }

    #[test]
    fn imports_only_existing_known_keys() {
        let settings = parse_settings("fov:0.5", "options", "default").unwrap();
        assert!(merge_options("guiScale:1\n", &settings).is_err());
    }

    #[test]
    fn lunar_game_options_keep_the_existing_share_contract() {
        let settings = parse_lunar_options(
            r#"{"fov":"80.0","guiScale":"2","fullscreen":"true","key_key.jump":"key.keyboard.space","lang":"en_us","lastServer":"unshared.invalid","future":{"key":"unshared"}}"#,
            "Lunar 1.21",
        )
        .unwrap();
        assert_eq!(settings.len(), 5);
        assert!(settings.iter().all(|setting| {
            setting.source == "minecraft"
                && setting.file_kind == "options"
                && setting.value.is_string()
                && crate::safety::validate_setting(setting).is_ok()
        }));
        let fov = settings
            .iter()
            .find(|setting| setting.pointer == "fov")
            .unwrap();
        assert_eq!(fov.value, Value::String("0.25".into()));
        assert_eq!(
            fov.id,
            parse_settings("fov:0.25", "options", "Lunar 1.21").unwrap()[0].id
        );
    }

    #[test]
    fn lunar_fov_endpoints_and_consecutive_imports_round_trip() {
        for (degrees, normalized) in [("30", "-1"), ("70", "0"), ("80", "0.25"), ("110", "1")] {
            let source = format!(r#"{{"fov":"{degrees}","guiScale":"2"}}"#);
            let settings = parse_lunar_options(&source, "default").unwrap();
            let fov = settings
                .iter()
                .find(|setting| setting.pointer == "fov")
                .unwrap();
            assert_eq!(fov.value.as_str(), Some(normalized));
            assert_eq!(merge_lunar_options(&source, &settings).unwrap(), source);
        }
        let mut content = r#"{"fov":"70","guiScale":"0","future":[1,2]}"#.to_owned();
        for (raw, expected_degree) in [("0.25", "80"), ("0.75", "100")] {
            let settings = parse_settings(
                &format!("fov:{raw}\r\nguiScale:3\r\n"),
                "options",
                "Windows profile",
            )
            .unwrap();
            content = merge_file(&content, &settings, Path::new("optionsLC.txt")).unwrap();
            let parsed = crate::lunar::parse_document(&content).unwrap();
            assert_eq!(
                parsed.get("fov").and_then(Value::as_str),
                Some(expected_degree)
            );
            assert_eq!(parsed.get("guiScale").and_then(Value::as_str), Some("3"));
            assert_eq!(
                parse_lunar_options(&content, "Windows profile").unwrap(),
                settings
            );
        }
    }

    #[test]
    fn lunar_scalar_updates_preserve_unknown_json_bytes() {
        let data = "\u{feff} {\r\n  \"fov\":\"70.0\", \"full\\u0073creen\":\"false\",\r\n  \"unknown/key~\": [18446744073709551617, 0.12345678901234567890123456789, {\"text\":\"} ] \\\" escaped\"}],\r\n  \"resourcePacks\": [\"unshared\"]\r\n}\r\n";
        let settings = parse_settings("fov:0.5\nfullscreen:true\n", "options", "default").unwrap();
        let output = merge_lunar_options(data, &settings).unwrap();
        assert_eq!(
            output,
            data.replace("\"70.0\"", "\"90\"")
                .replace("\"false\"", "\"true\"")
        );
        assert_eq!(parse_lunar_options(&output, "default").unwrap(), settings);
        assert_eq!(merge_lunar_options(data, &[]).unwrap(), data);
    }

    #[test]
    fn lunar_options_reject_duplicates_and_skip_unverified_values() {
        for data in [
            r#"{"fov":"70","fov":"80"}"#,
            r#"{"future":{"duplicate":1,"duplicate":2}}"#,
            "[]",
            "fov:0.25\n",
            "",
        ] {
            assert!(parse_lunar_options(data, "default").is_err());
        }
        assert!(parse_lunar_options("{}", "default").unwrap().is_empty());
        assert!(parse_lunar_options(
            r#"{"fov":"29","guiScale":2,"fullscreen":true,"key_key.jump":"key.keyboard.unverified","future":"unshared"}"#,
            "default"
        )
        .unwrap()
        .is_empty());
        for raw in ["111", "NaN", "inf", "-1", "0.5"] {
            assert!(
                parse_lunar_options(&format!(r#"{{"fov":"{raw}"}}"#), "default")
                    .unwrap()
                    .is_empty()
            );
        }
    }

    #[test]
    fn lunar_imports_require_existing_valid_string_fields() {
        let fov = parse_settings("fov:0.5", "options", "default").unwrap();
        for data in [r#"{}"#, r#"{"fov":70}"#, r#"{"fov":"invalid"}"#] {
            assert!(merge_lunar_options(data, &fov).is_err());
        }
        assert!(merge_lunar_options(r#"{"fov":"70"}"#, &[fov[0].clone(), fov[0].clone()]).is_err());
        let unsupported_fov = parse_settings("fov:1.5", "options", "default").unwrap();
        assert_eq!(unsupported_fov.len(), 1);
        assert!(validate_file_setting(&unsupported_fov[0], Path::new("options.txt")).is_ok());
        assert!(validate_file_setting(&unsupported_fov[0], Path::new("optionsLC.txt")).is_err());
        assert!(merge_lunar_options(r#"{"fov":"70"}"#, &unsupported_fov).is_err());
        let absent = parse_settings("graphicsMode:2", "options", "default").unwrap();
        assert!(merge_lunar_options(r#"{"fov":"70"}"#, &absent).is_err());
    }

    #[test]
    fn game_format_dispatch_uses_exact_known_filenames() {
        let json = r#"{"fov":"80"}"#;
        let settings = parse_file(json, "options", "default", Path::new("optionsLC.txt")).unwrap();
        assert_eq!(settings[0].value.as_str(), Some("0.25"));
        assert!(parse_file(json, "mods", "default", Path::new("optionsLC.txt")).is_err());
        assert!(parse_file("fov:0.25", "options", "default", Path::new("optionsLC.txt")).is_err());
        for filename in ["OPTIONSLC.TXT", "future.txt", "optionsof.txt"] {
            let path = Path::new(filename);
            assert!(parse_file(json, "options", "default", path).is_err());
            assert!(merge_file(json, &settings, path).is_err());
            assert!(validate_file_setting(&settings[0], path).is_err());
        }
        assert_eq!(
            parse_file("fov:0.25", "options", "default", Path::new("options.txt")).unwrap(),
            settings
        );
    }
}
