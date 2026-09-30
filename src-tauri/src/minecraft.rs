use crate::model::{setting_id, Setting};
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};

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
}
