use crate::model::Setting;
use serde_json::Value;
use std::collections::BTreeMap;
use std::sync::OnceLock;

pub fn validate_setting(setting: &Setting) -> Result<(), String> {
    if setting.pointer.is_empty()
        || setting.pointer.len() > 512
        || setting.profile.is_empty()
        || setting.profile.len() > 256
        || setting
            .profile
            .chars()
            .any(|c| c.is_control() || matches!(c, '/' | '\\'))
    {
        return Err("This setting has an unsupported identity.".into());
    }
    let allowed = match setting.source.as_str() {
        "minecraft" => {
            setting.file_kind == "options"
                && crate::minecraft::describe_key(&setting.pointer).is_some()
                && setting
                    .value
                    .as_str()
                    .is_some_and(|raw| minecraft_value_is_safe(&setting.pointer, raw))
        }
        "lunar" => lunar_value_is_safe(&setting.file_kind, &setting.pointer, &setting.value),
        _ => false,
    };
    if allowed {
        Ok(())
    } else {
        Err("This setting is not approved for sharing or import.".into())
    }
}

pub fn canonicalize_setting(setting: &Setting) -> Result<Setting, String> {
    validate_setting(setting)?;
    let (category, group, label) = if setting.source == "minecraft" {
        let (category, group, label) = crate::minecraft::describe_key(&setting.pointer).unwrap();
        (category.to_owned(), group.to_owned(), label)
    } else {
        crate::lunar::describe_pointer(&setting.file_kind, &setting.pointer)
    };
    let mut canonical = setting.clone();
    canonical.category = category;
    canonical.group = group;
    canonical.label = label;
    Ok(canonical)
}

fn finite_number(raw: &str, min: f64, max: f64) -> bool {
    raw.len() <= 32
        && raw
            .parse::<f64>()
            .is_ok_and(|n| n.is_finite() && n >= min && n <= max)
}

fn integer(raw: &str, min: i64, max: i64) -> bool {
    raw.len() <= 20 && raw.parse::<i64>().is_ok_and(|n| n >= min && n <= max)
}

fn boolean(raw: &str) -> bool {
    matches!(raw, "true" | "false")
}

fn minecraft_value_is_safe(key: &str, raw: &str) -> bool {
    if raw.len() > 8192 || raw.chars().any(|c| c.is_control()) {
        return false;
    }
    if key.starts_with("key_") {
        return keybind_is_safe(raw);
    }
    if key.starts_with("soundCategory_") {
        return finite_number(raw, 0.0, 1.0);
    }
    match key {
        "resourcePacks" | "incompatibleResourcePacks" => resource_packs_are_safe(raw),
        "lang" => {
            let parts: Vec<&str> = raw.split('_').collect();
            parts.len() == 2
                && parts.iter().all(|p| {
                    (2..=4).contains(&p.len()) && p.chars().all(|c| c.is_ascii_alphabetic())
                })
        }
        "mainHand" => matches!(raw, "left" | "right"),
        "tutorialStep" => matches!(
            raw,
            "none" | "movement" | "find_tree" | "punch_tree" | "open_inventory" | "craft_planks"
        ),
        "fullscreenResolution" => {
            raw.is_empty()
                || (raw.len() <= 40
                    && raw
                        .chars()
                        .all(|c| c.is_ascii_digit() || "x@: ".contains(c))
                    && raw.contains('x'))
        }
        "renderClouds" | "clouds" => matches!(raw, "true" | "false" | "fast" | "fancy" | "off"),
        "ao" => boolean(raw) || integer(raw, 0, 2),
        "mouseSensitivity" => finite_number(raw, 0.0, 1.0),
        "mouseWheelSensitivity" => finite_number(raw, 0.01, 100.0),
        "fov" => finite_number(raw, -1.0, 2.0),
        "gamma" => finite_number(raw, -1.0, 100.0),
        "fovEffectScale"
        | "darknessEffectScale"
        | "screenEffectScale"
        | "damageTiltStrength"
        | "glintStrength"
        | "glintSpeed"
        | "panoramaScrollSpeed"
        | "chatOpacity"
        | "chatScale"
        | "chatWidth"
        | "chatHeightFocused"
        | "chatHeightUnfocused"
        | "chatLineSpacing"
        | "chatBackgroundOpacity"
        | "textBackgroundOpacity" => finite_number(raw, 0.0, 1.0),
        "entityDistanceScaling" => finite_number(raw, 0.0, 10.0),
        "notificationDisplayTime" => finite_number(raw, 0.0, 10.0),
        "chatDelay" => finite_number(raw, 0.0, 6.0),
        "renderDistance" | "simulationDistance" => integer(raw, 2, 128),
        "maxFps" => integer(raw, 0, 10000),
        "overrideWidth" | "overrideHeight" => integer(raw, 0, 32768),
        "guiScale" => integer(raw, 0, 32),
        "mipmapLevels" => integer(raw, 0, 4),
        "graphicsMode"
        | "particles"
        | "chatVisibility"
        | "attackIndicator"
        | "chunkBuilder"
        | "prioritizeChunkUpdates" => integer(raw, 0, 3),
        "narrator" => integer(raw, 0, 3),
        "biomeBlendRadius" => integer(raw, 0, 64),
        "glDebugVerbosity" => integer(raw, 0, 4),
        "menuBackgroundBlurriness" => integer(raw, 0, 10),
        _ => boolean(raw),
    }
}

fn resource_packs_are_safe(raw: &str) -> bool {
    let Ok(Value::Array(packs)) = serde_json::from_str::<Value>(raw) else {
        return false;
    };
    packs.len() <= 64
        && packs.iter().all(|pack| {
            let Some(pack) = pack.as_str() else {
                return false;
            };
            let identifier = pack
                .strip_prefix("file/")
                .or_else(|| pack.strip_prefix("mod/"))
                .unwrap_or(pack);
            !identifier.is_empty()
                && identifier.len() <= 160
                && !identifier.starts_with('.')
                && !identifier.contains("..")
                && identifier
                    .chars()
                    .all(|c| c.is_alphanumeric() || "_-. ()".contains(c))
        })
}

pub fn keybind_is_safe(raw: &str) -> bool {
    if integer(raw, -100, 512) || raw == "NONE" || raw == "KEY_NONE" {
        return true;
    }
    let key = raw
        .strip_prefix("key.keyboard.")
        .or_else(|| raw.strip_prefix("KEY_"));
    if let Some(key) = key {
        let normalized = key.to_ascii_lowercase().replace('_', ".");
        return (normalized.len() == 1 && normalized.chars().all(|c| c.is_ascii_alphanumeric()))
            || matches!(
                normalized.as_str(),
                "unknown"
                    | "space"
                    | "escape"
                    | "enter"
                    | "tab"
                    | "backspace"
                    | "insert"
                    | "delete"
                    | "right"
                    | "left"
                    | "down"
                    | "up"
                    | "page.up"
                    | "page.down"
                    | "home"
                    | "end"
                    | "caps.lock"
                    | "scroll.lock"
                    | "num.lock"
                    | "print.screen"
                    | "pause"
                    | "left.shift"
                    | "right.shift"
                    | "left.control"
                    | "right.control"
                    | "left.alt"
                    | "right.alt"
                    | "left.win"
                    | "right.win"
                    | "menu"
                    | "lshift"
                    | "rshift"
                    | "lcontrol"
                    | "rcontrol"
                    | "lmenu"
                    | "rmenu"
                    | "return"
                    | "minus"
                    | "equal"
                    | "apostrophe"
                    | "comma"
                    | "period"
                    | "slash"
                    | "semicolon"
                    | "left.bracket"
                    | "right.bracket"
                    | "backslash"
                    | "grave.accent"
            )
            || normalized
                .strip_prefix('f')
                .is_some_and(|v| integer(v, 1, 25))
            || normalized.strip_prefix("keypad.").is_some_and(|v| {
                integer(v, 0, 9)
                    || matches!(
                        v,
                        "decimal" | "divide" | "multiply" | "subtract" | "add" | "enter" | "equal"
                    )
            });
    }
    if let Some(mouse) = raw.strip_prefix("key.mouse.") {
        return matches!(mouse, "left" | "right" | "middle") || integer(mouse, 1, 8);
    }
    raw.strip_prefix("MOUSE_")
        .is_some_and(|mouse| integer(mouse, 0, 8))
}

pub fn lunar_value_is_safe(file_kind: &str, pointer: &str, value: &Value) -> bool {
    static SCHEMA: OnceLock<BTreeMap<String, Vec<String>>> = OnceLock::new();
    let schema = SCHEMA.get_or_init(|| {
        serde_json::from_str(include_str!("lunar_schema.json"))
            .expect("The bundled Lunar schema must be valid")
    });
    let key = format!("{file_kind}:{pointer}");
    let Some(types) = schema.get(&key) else {
        return false;
    };
    types.iter().any(|kind| match kind.as_str() {
        "bool" => value.is_boolean(),
        "number" => value
            .as_f64()
            .is_some_and(|n| n.is_finite() && (-4294967296.0..=4294967295.0).contains(&n)),
        "numericString" => value
            .as_str()
            .is_some_and(|raw| finite_number(raw, -32768.0, 32768.0)),
        "keybind" => value.as_str().is_some_and(keybind_is_safe),
        "position" => value.as_str().is_some_and(|raw| {
            if raw.len() > 32
                || !raw
                    .chars()
                    .all(|c| c.is_ascii_alphabetic() || c == '_' || c == '-')
            {
                return false;
            }
            let normalized = raw.to_ascii_lowercase().replace(['_', '-'], "");
            matches!(
                normalized.as_str(),
                "topleft"
                    | "topright"
                    | "bottomleft"
                    | "bottomright"
                    | "center"
                    | "middle"
                    | "topcenter"
                    | "bottomcenter"
                    | "centerleft"
                    | "centerright"
                    | "middleleft"
                    | "middleright"
                    | "middlecenter"
                    | "bottomcenterleft"
                    | "bottomcenterright"
                    | "topmiddle"
                    | "bottommiddle"
                    | "left"
                    | "right"
                    | "top"
                    | "bottom"
            )
        }),
        _ => false,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::minecraft::parse_settings;
    use serde_json::json;

    #[test]
    fn secrets_and_unknown_keys_are_never_shared() {
        let settings = parse_settings("fov:0.5\nlastServer:private.example\naccessToken:secret\nmouseSensitivity:0.4\nfutureOption:1\n", "options", "default").unwrap();
        assert_eq!(settings.len(), 2);
        assert!(lunar_value_is_safe(
            "mods",
            "/WAYPOINTS/enabled",
            &json!(true)
        ));
        assert!(!lunar_value_is_safe(
            "mods",
            "/WAYPOINTS/waypoints",
            &json!([])
        ));
        assert!(!lunar_value_is_safe("mods", "/WAYPOINTS/x", &json!(0.5)));
        assert!(!lunar_value_is_safe(
            "mods",
            "/SERVER_ADDRESS/x",
            &json!(0.5)
        ));
        assert!(!lunar_value_is_safe(
            "mods",
            "/FPS/options/token",
            &json!("secret")
        ));
        assert!(!lunar_value_is_safe(
            "mods",
            "/FUTURE_MOD/enabled",
            &json!(true)
        ));
    }

    #[test]
    fn approved_values_have_bounded_grammars() {
        assert!(keybind_is_safe("key.keyboard.left.shift"));
        assert!(keybind_is_safe("KEY_R"));
        assert!(!keybind_is_safe("key.keyboard.accessToken"));
        assert!(!lunar_value_is_safe(
            "mods",
            "/FPS/x",
            &json!("path/to/file")
        ));
        assert!(minecraft_value_is_safe(
            "resourcePacks",
            "[\"vanilla\",\"file/Example.zip\"]"
        ));
        assert!(!minecraft_value_is_safe(
            "resourcePacks",
            "[\"file/../accounts.json\"]"
        ));
    }

    #[test]
    fn all_primary_hud_anchor_ids_are_supported_without_opening_the_grammar() {
        for anchor in [
            "topLeft",
            "topCenter",
            "topRight",
            "middleLeft",
            "middleCenter",
            "middleRight",
            "bottomLeft",
            "bottomCenterLeft",
            "bottomCenterRight",
            "bottomRight",
        ] {
            assert!(lunar_value_is_safe("mods", "/FPS/position", &json!(anchor)));
        }
        for anchor in [
            "futureAnchor",
            "bottomCenterRight/private",
            "server.example",
        ] {
            assert!(!lunar_value_is_safe(
                "mods",
                "/FPS/position",
                &json!(anchor)
            ));
        }
    }
}
