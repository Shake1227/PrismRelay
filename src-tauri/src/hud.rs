use crate::model::{ScanFile, ScanReport, Setting};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeSet;
use std::path::Path;

const UNAVAILABLE: &str = "The Lunar HUD viewport could not be determined.";
const INVALID_VIEWPORT: &str = "The HUD viewport dimensions are invalid.";
const MODERN_RENDERER_IDS: &[&str] = &[
    "v1_16_1",
    "v1_16_5",
    "v1_17_0",
    "v1_17_1",
    "v1_18_1",
    "v1_18_2",
    "v1_19_pre",
    "v1_19_0",
    "v1_19_2",
    "v1_19_3",
    "v1_19_4",
    "v1_20_0",
    "v1_20_1",
    "v1_20_2",
    "v1_20_3",
    "v1_20_4",
    "v1_20_5",
    "v1_20_6",
    "v1_21_0",
    "v1_21_1",
    "v1_21_2",
    "v1_21_3",
    "v1_21_4",
    "v1_21_5",
    "v1_21_6",
    "v1_21_7",
    "v1_21_8",
    "v1_21_9",
    "v1_21_10",
    "v1_21_11",
    "v26_1",
    "v26_1_1",
    "v26_1_2",
    "v26_2",
    "v26_3",
];
const ROOT_HUD_MODULES: &[&str] = &[
    "ARMORSTATUS",
    "BOSSBAR",
    "CLOCK",
    "COMBO",
    "COOLDOWNS",
    "COORDINATES",
    "CPS",
    "DIRECTION_HUD",
    "FPS",
    "ITEM_COUNTER",
    "ITEM_TRACKER",
    "KEYSTROKES",
    "MEMORY",
    "MOMENTUM",
    "PING",
    "PLAYTIME",
    "POTION_EFFECTS",
    "REACH_DISPLAY",
    "STOPWATCH",
];

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct HudViewport {
    pub width: f64,
    pub height: f64,
}

impl HudViewport {
    pub fn validate(&self) -> Result<(), String> {
        if [self.width, self.height]
            .iter()
            .all(|value| value.is_finite() && (16.0..=32768.0).contains(value))
        {
            Ok(())
        } else {
            Err(INVALID_VIEWPORT.into())
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct HudWindow {
    pub physical_width: u32,
    pub physical_height: u32,
    pub logical_height: u32,
    pub macos: bool,
}

impl HudWindow {
    pub fn validate(&self) -> Result<(), String> {
        if !(16..=65536).contains(&self.physical_width)
            || !(16..=65536).contains(&self.physical_height)
            || self.logical_height == 0
            || self.logical_height > self.physical_height
            || (self.macos && self.physical_height / self.logical_height > 8)
        {
            return Err(INVALID_VIEWPORT.into());
        }
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct HudAdjustment {
    pub settings: Vec<Setting>,
    pub adjusted_count: usize,
    pub unsupported_count: usize,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ScaleMode {
    Auto,
    Off,
    Mods,
    All,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum RendererVersion {
    Legacy,
    Modern,
}

fn renderer_version(value: Option<&Value>) -> Option<RendererVersion> {
    let id = value?.as_str()?;
    if MODERN_RENDERER_IDS.contains(&id) {
        Some(RendererVersion::Modern)
    } else if matches!(id, "v1_7" | "v1_8" | "v1_9" | "v1_10" | "v1_11" | "v1_12") {
        Some(RendererVersion::Legacy)
    } else {
        None
    }
}

pub fn viewport(
    report: &ScanReport,
    lunar_profile: &str,
    window: &HudWindow,
    selected: &[Setting],
) -> Result<HudViewport, String> {
    window.validate()?;
    if !report
        .lunar_profiles
        .iter()
        .any(|profile| profile == lunar_profile)
    {
        return Err(UNAVAILABLE.into());
    }
    let (mut gui_scale, mut unicode) = game_scale(report)?;
    for pointer in ["guiScale", "forceUnicodeFont"] {
        let incoming: Vec<_> = selected
            .iter()
            .filter(|setting| {
                setting.source == "minecraft"
                    && setting.file_kind == "options"
                    && setting.pointer == pointer
            })
            .collect();
        if incoming.len() > 1 {
            return Err(UNAVAILABLE.into());
        }
        if let [setting] = incoming.as_slice() {
            crate::safety::validate_setting(setting).map_err(|_| UNAVAILABLE)?;
            if report.settings.iter().any(|local| {
                local.source == "minecraft"
                    && local.file_kind == "options"
                    && local.pointer == pointer
                    && crate::safety::validate_setting(local).is_ok()
            }) {
                match pointer {
                    "guiScale" => gui_scale = parse_gui_scale(&setting.value)?,
                    "forceUnicodeFont" => unicode = parse_unicode(&setting.value)?,
                    _ => unreachable!(),
                }
            }
        }
    }
    let general: Vec<_> = report
        .files
        .iter()
        .filter(|file| {
            file.source == "lunar" && file.file_kind == "general" && file.profile == lunar_profile
        })
        .collect();
    let (mode, high_dpi) = match general.as_slice() {
        [] => (ScaleMode::Auto, true),
        [file] => {
            let content = read_registered(file, "general.json")?;
            general_scale(&crate::lunar::parse_document(&content).map_err(|_| UNAVAILABLE)?)?
        }
        _ => return Err(UNAVAILABLE.into()),
    };
    calculate_viewport(window, gui_scale, unicode, mode, high_dpi)
}

fn game_scale(report: &ScanReport) -> Result<(u32, bool), String> {
    let files: Vec<_> = report
        .files
        .iter()
        .filter(|file| {
            file.source == "minecraft"
                && matches!(file.file_kind.as_str(), "options" | "options_legacy")
                && matches!(
                    Path::new(&file.path)
                        .file_name()
                        .and_then(|name| name.to_str()),
                    Some("options.txt" | "optionsLC.txt")
                )
        })
        .collect();
    let profiles: BTreeSet<_> = files.iter().map(|file| file.profile.as_str()).collect();
    if profiles.len() != 1 {
        return Err(UNAVAILABLE.into());
    }
    let primary: Vec<_> = files
        .iter()
        .filter(|file| {
            Path::new(&file.path)
                .file_name()
                .is_some_and(|name| name == "optionsLC.txt")
        })
        .collect();
    let file = match primary.as_slice() {
        [file] => **file,
        [] if files.len() == 1 => files[0],
        _ => return Err(UNAVAILABLE.into()),
    };
    let name = Path::new(&file.path)
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or(UNAVAILABLE)?;
    let content = read_registered(file, name)?;
    if name != "optionsLC.txt" {
        return Err(UNAVAILABLE.into());
    }
    let document = crate::lunar::parse_document(&content).map_err(|_| UNAVAILABLE)?;
    if renderer_version(document.get("lastLaunchedVersion")) != Some(RendererVersion::Modern) {
        return Err(UNAVAILABLE.into());
    }
    let (gui, unicode) = game_fields(&content, name)?;
    if name == "optionsLC.txt" && (gui.is_none() || unicode.is_none()) {
        let mirror: Vec<_> = files
            .iter()
            .filter(|candidate| {
                Path::new(&candidate.path)
                    .file_name()
                    .is_some_and(|name| name == "options.txt")
                    && Path::new(&candidate.path).parent() == Path::new(&file.path).parent()
            })
            .collect();
        if mirror.len() > 1 {
            return Err(UNAVAILABLE.into());
        }
        if let [mirror] = mirror.as_slice() {
            let content = read_registered(mirror, "options.txt")?;
            let (mirror_gui, mirror_unicode) = game_fields(&content, "options.txt")?;
            if gui.is_none() && mirror_gui.is_some()
                || unicode.is_none()
                    && mirror_unicode
                        .as_ref()
                        .map(parse_unicode)
                        .transpose()?
                        .unwrap_or(false)
            {
                return Err(UNAVAILABLE.into());
            }
        }
    }
    Ok((
        gui.as_ref().map(parse_gui_scale).transpose()?.unwrap_or(0),
        unicode
            .as_ref()
            .map(parse_unicode)
            .transpose()?
            .unwrap_or(false),
    ))
}

fn game_fields(content: &str, name: &str) -> Result<(Option<Value>, Option<Value>), String> {
    if name == "optionsLC.txt" {
        let document = crate::lunar::parse_document(content).map_err(|_| UNAVAILABLE)?;
        Ok((
            document.get("guiScale").cloned(),
            document.get("forceUnicodeFont").cloned(),
        ))
    } else {
        let document = crate::minecraft::parse_options(content).map_err(|_| UNAVAILABLE)?;
        let read = |key: &str| -> Result<Option<Value>, String> {
            let values: Vec<_> = document
                .lines
                .iter()
                .filter(|line| line.key.as_deref() == Some(key))
                .map(|line| line.value.clone())
                .collect();
            match values.as_slice() {
                [] => Ok(None),
                [Some(value)] => Ok(Some(Value::String(value.clone()))),
                _ => Err(UNAVAILABLE.into()),
            }
        };
        Ok((read("guiScale")?, read("forceUnicodeFont")?))
    }
}

fn read_registered(file: &ScanFile, name: &str) -> Result<String, String> {
    if Path::new(&file.path)
        .file_name()
        .and_then(|name| name.to_str())
        != Some(name)
        || !matches!(name, "options.txt" | "optionsLC.txt" | "general.json")
    {
        return Err(UNAVAILABLE.into());
    }
    String::from_utf8(crate::backup::read_config(Path::new(&file.path)).map_err(|_| UNAVAILABLE)?)
        .map_err(|_| UNAVAILABLE.into())
}

fn parse_gui_scale(value: &Value) -> Result<u32, String> {
    value
        .as_str()
        .and_then(|value| value.parse::<u32>().ok())
        .filter(|value| *value <= 32)
        .ok_or_else(|| UNAVAILABLE.into())
}

fn parse_unicode(value: &Value) -> Result<bool, String> {
    match value.as_str() {
        Some("true") => Ok(true),
        Some("false") => Ok(false),
        _ => Err(UNAVAILABLE.into()),
    }
}

fn general_scale(document: &Value) -> Result<(ScaleMode, bool), String> {
    let mode = match document.get("useMinecraftScale") {
        None => ScaleMode::Auto,
        Some(Value::String(value)) => match value.as_str() {
            "auto" => ScaleMode::Auto,
            "off" => ScaleMode::Off,
            "mods" => ScaleMode::Mods,
            "all" => ScaleMode::All,
            _ => return Err(UNAVAILABLE.into()),
        },
        _ => return Err(UNAVAILABLE.into()),
    };
    let high_dpi = match document.get("highDPIScale") {
        None => true,
        Some(Value::Bool(value)) => *value,
        _ => return Err(UNAVAILABLE.into()),
    };
    Ok((mode, high_dpi))
}

fn calculate_viewport(
    window: &HudWindow,
    requested: u32,
    unicode: bool,
    mode: ScaleMode,
    high_dpi: bool,
) -> Result<HudViewport, String> {
    window.validate()?;
    if requested > 32 {
        return Err(UNAVAILABLE.into());
    }
    let mut gui = 1;
    while (requested == 0 || gui < requested)
        && window.physical_width / (gui + 1) >= 320
        && window.physical_height / (gui + 1) >= 240
    {
        gui += 1;
    }
    if unicode && gui % 2 != 0 {
        gui += 1;
    }
    let dpi = if window.macos {
        (window.physical_height / window.logical_height).max(1)
    } else {
        1
    };
    let high_dpi_factor = if high_dpi { dpi } else { 1 };
    let ratio = gui as f32 / high_dpi_factor as f32;
    let minecraft = ratio.trunc().max(1.0);
    let scale = match mode {
        ScaleMode::All | ScaleMode::Mods => 1.0,
        ScaleMode::Auto if ratio > 4.0 => 4.0 / minecraft,
        ScaleMode::Auto | ScaleMode::Off => 2.0 / minecraft,
    };
    let pixels_per_hud_unit = (gui as f32 * scale).trunc() as f64 / dpi as f64;
    let viewport = HudViewport {
        width: window.physical_width as f64 / pixels_per_hud_unit,
        height: window.physical_height as f64 / pixels_per_hud_unit,
    };
    viewport.validate()?;
    Ok(viewport)
}

pub fn is_adaptable_coordinate(setting: &Setting) -> bool {
    if setting.source != "lunar"
        || setting.file_kind != "mods"
        || !setting.value.is_number()
        || crate::safety::validate_setting(setting).is_err()
    {
        return false;
    }
    let parts: Vec<_> = setting.pointer.split('/').collect();
    matches!(parts.as_slice(), ["", module, "x" | "y"] if ROOT_HUD_MODULES.contains(module))
}

pub fn adapt(
    settings: &[Setting],
    source: Option<&HudViewport>,
    target: Option<&HudViewport>,
) -> Result<HudAdjustment, String> {
    if let Some(source) = source {
        source.validate()?;
    }
    if let Some(target) = target {
        target.validate()?;
    }
    let mut adjusted = settings.to_vec();
    let mut adjusted_count = 0;
    let mut unsupported_count = 0;
    for setting in &mut adjusted {
        if !is_adaptable_coordinate(setting) {
            continue;
        }
        let (Some(source), Some(target)) = (source, target) else {
            unsupported_count += 1;
            continue;
        };
        let ratio = if setting.pointer.ends_with("/x") {
            target.width / source.width
        } else {
            target.height / source.height
        };
        let incoming = setting.value.as_f64().ok_or(UNAVAILABLE)?;
        let value = serde_json::Number::from_f64(incoming * ratio).ok_or(INVALID_VIEWPORT)?;
        setting.value = Value::Number(value);
        crate::safety::validate_setting(setting).map_err(|_| INVALID_VIEWPORT)?;
        if setting.value.as_f64() != Some(incoming) {
            adjusted_count += 1;
        }
    }
    Ok(HudAdjustment {
        settings: adjusted,
        adjusted_count,
        unsupported_count,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::fs;

    fn setting(pointer: &str, value: Value) -> Setting {
        Setting {
            id: pointer.into(),
            label: String::new(),
            source: "lunar".into(),
            category: String::new(),
            group: String::new(),
            file_kind: "mods".into(),
            profile: "Default".into(),
            pointer: pointer.into(),
            value,
        }
    }

    fn window(width: u32, height: u32) -> HudWindow {
        HudWindow {
            physical_width: width,
            physical_height: height,
            logical_height: height,
            macos: false,
        }
    }

    fn report(root: &Path, options: &str, general: &str) -> ScanReport {
        fs::write(root.join("options.txt"), options).unwrap();
        fs::write(root.join("general.json"), general).unwrap();
        let (gui, unicode) = game_fields(options, "options.txt").unwrap();
        let mut lunar_options = json!({"lastLaunchedVersion":"v1_21_11"});
        if let Some(gui) = gui {
            lunar_options["guiScale"] = gui;
        }
        if let Some(unicode) = unicode {
            lunar_options["forceUnicodeFont"] = unicode;
        }
        fs::write(
            root.join("optionsLC.txt"),
            serde_json::to_string(&lunar_options).unwrap(),
        )
        .unwrap();
        let report = ScanReport {
            files: vec![
                ScanFile {
                    id: "minecraft:options".into(),
                    source: "minecraft".into(),
                    path: root.join("options.txt").to_string_lossy().into(),
                    file_kind: "options".into(),
                    profile: "Minecraft".into(),
                },
                ScanFile {
                    id: "minecraft:lunar-options".into(),
                    source: "minecraft".into(),
                    path: root.join("optionsLC.txt").to_string_lossy().into(),
                    file_kind: "options".into(),
                    profile: "Minecraft".into(),
                },
                ScanFile {
                    id: "lunar:general".into(),
                    source: "lunar".into(),
                    path: root.join("general.json").to_string_lossy().into(),
                    file_kind: "general".into(),
                    profile: "Default".into(),
                },
            ],
            settings: crate::minecraft::parse_settings(options, "options", "Minecraft").unwrap(),
            lunar_profiles: vec!["Default".into()],
            ..ScanReport::default()
        };
        for file in &report.files {
            assert!(crate::backup::read_config(Path::new(&file.path)).is_ok());
        }
        report
    }

    #[test]
    fn root_offsets_follow_each_viewport_axis_and_preserve_anchors() {
        let source = HudViewport {
            width: 400.0,
            height: 300.0,
        };
        let target = HudViewport {
            width: 800.0,
            height: 900.0,
        };
        let selected = vec![
            setting("/FPS/x", json!(20.0)),
            setting("/FPS/y", json!(-10.0)),
            setting("/FPS/position", json!("bottomCenterRight")),
            setting("/CPS/x", json!(0.25)),
        ];
        let result = adapt(&selected, Some(&source), Some(&target)).unwrap();
        assert_eq!(result.adjusted_count, 3);
        assert_eq!(result.unsupported_count, 0);
        assert_eq!(result.settings[0].value, json!(40.0));
        assert_eq!(result.settings[1].value, json!(-30.0));
        assert_eq!(result.settings[2], selected[2]);
        assert_eq!(result.settings[3].value, json!(0.5));
    }

    #[test]
    fn nested_offsets_and_component_scales_remain_exact() {
        let viewport = HudViewport {
            width: 800.0,
            height: 600.0,
        };
        let target = HudViewport {
            width: 1600.0,
            height: 1200.0,
        };
        let selected = vec![
            setting("/F3_DISPLAY/default_pie_chart/y", json!(-0.375)),
            setting("/F3_DISPLAY/default_pie_chart/options/scale", json!(1.25)),
            setting("/FPS/options/background", json!(true)),
        ];
        for setting in &selected {
            assert!(crate::safety::validate_setting(setting).is_ok());
            assert!(!is_adaptable_coordinate(setting));
        }
        let result = adapt(&selected, Some(&viewport), Some(&target)).unwrap();
        assert_eq!(result.settings, selected);
        assert_eq!((result.adjusted_count, result.unsupported_count), (0, 0));
        let result = adapt(&[setting("/FPS/x", json!(10))], None, Some(&target)).unwrap();
        assert_eq!((result.adjusted_count, result.unsupported_count), (0, 1));
        assert_eq!(result.settings[0].value, json!(10));
    }

    #[test]
    fn gui_scale_modes_use_primary_float_and_integer_units() {
        let standard = window(1920, 1080);
        let off = calculate_viewport(&standard, 4, false, ScaleMode::Off, true).unwrap();
        assert_eq!(
            off,
            HudViewport {
                width: 960.0,
                height: 540.0
            }
        );
        for mode in [ScaleMode::All, ScaleMode::Mods] {
            assert_eq!(
                calculate_viewport(&standard, 4, false, mode, true).unwrap(),
                HudViewport {
                    width: 480.0,
                    height: 270.0
                },
            );
        }
        let large = window(3840, 2160);
        assert_eq!(
            calculate_viewport(&large, 0, false, ScaleMode::Auto, true).unwrap(),
            HudViewport {
                width: 960.0,
                height: 540.0
            },
        );
        assert_eq!(
            calculate_viewport(&standard, 3, true, ScaleMode::All, true).unwrap(),
            HudViewport {
                width: 480.0,
                height: 270.0
            },
        );
    }

    #[test]
    fn retina_uses_framebuffer_to_logical_height_and_high_dpi_flag() {
        let retina = HudWindow {
            logical_height: 540,
            macos: true,
            ..window(1920, 1080)
        };
        assert_eq!(
            calculate_viewport(&retina, 3, false, ScaleMode::Off, true).unwrap(),
            HudViewport {
                width: 640.0,
                height: 360.0
            },
        );
        assert_eq!(
            calculate_viewport(&retina, 3, false, ScaleMode::Off, false).unwrap(),
            HudViewport {
                width: 1920.0,
                height: 1080.0
            },
        );
        assert_eq!(
            calculate_viewport(&retina, 3, false, ScaleMode::All, true).unwrap(),
            HudViewport {
                width: 1280.0,
                height: 720.0
            },
        );
    }

    #[test]
    fn scale_options_require_exact_primary_types_and_vocabulary() {
        assert_eq!(general_scale(&json!({})).unwrap(), (ScaleMode::Auto, true));
        for mode in ["auto", "off", "mods", "all"] {
            assert!(general_scale(&json!({"useMinecraftScale":mode,"highDPIScale":false})).is_ok());
        }
        for document in [
            json!({"useMinecraftScale":true}),
            json!({"useMinecraftScale":"future-mode"}),
            json!({"highDPIScale":"true"}),
        ] {
            assert!(general_scale(&document).is_err());
        }
        assert!(parse_gui_scale(&json!(2)).is_err());
        assert!(parse_gui_scale(&json!("33")).is_err());
        assert!(parse_unicode(&json!(false)).is_err());
    }

    #[test]
    fn prospective_gui_scale_only_uses_a_supported_target_field() {
        let directory = tempfile::tempdir().unwrap();
        let root = dunce::canonicalize(directory.path()).unwrap();
        let mut local = report(&root, "guiScale:2\n", r#"{"useMinecraftScale":"all"}"#);
        let incoming =
            crate::minecraft::parse_settings("guiScale:4\n", "options", "Other").unwrap();
        let size = window(1920, 1080);
        assert_eq!(
            viewport(&local, "Default", &size, &[]).unwrap().width,
            960.0
        );
        assert_eq!(
            viewport(&local, "Default", &size, &incoming).unwrap().width,
            480.0
        );
        local.settings.clear();
        assert_eq!(
            viewport(&local, "Default", &size, &incoming).unwrap().width,
            960.0
        );
        assert!(viewport(&local, "Other", &size, &[]).is_err());
        fs::write(
            root.join("general.json"),
            r#"{"useMinecraftScale":"all","useMinecraftScale":"off"}"#,
        )
        .unwrap();
        assert!(viewport(&local, "Default", &size, &[]).is_err());
    }

    #[test]
    fn missing_lunar_gui_field_does_not_guess_a_saved_text_value() {
        let directory = tempfile::tempdir().unwrap();
        let root = dunce::canonicalize(directory.path()).unwrap();
        let local = report(&root, "guiScale:2\n", "{}");
        fs::write(
            root.join("optionsLC.txt"),
            r#"{"fov":"70","lastLaunchedVersion":"v1_21_11"}"#,
        )
        .unwrap();
        let size = window(1920, 1080);
        assert!(viewport(&local, "Default", &size, &[]).is_err());
        fs::write(
            root.join("optionsLC.txt"),
            r#"{"guiScale":"3","lastLaunchedVersion":"v1_21_11"}"#,
        )
        .unwrap();
        assert_eq!(
            viewport(&local, "Default", &size, &[]).unwrap().width,
            960.0
        );
        fs::write(
            root.join("optionsLC.txt"),
            r#"{"guiScale":3,"lastLaunchedVersion":"v1_21_11"}"#,
        )
        .unwrap();
        assert!(viewport(&local, "Default", &size, &[]).is_err());
    }

    #[test]
    fn only_exact_known_modern_version_ids_allow_automatic_geometry() {
        assert_eq!(MODERN_RENDERER_IDS.len(), 35);
        for id in ["v1_16_1", "v1_19_pre", "v1_21_11", "v26_3"] {
            assert_eq!(
                renderer_version(Some(&json!(id))),
                Some(RendererVersion::Modern)
            );
        }
        for id in ["v1_7", "v1_8", "v1_12"] {
            assert_eq!(
                renderer_version(Some(&json!(id))),
                Some(RendererVersion::Legacy)
            );
        }
        for value in [
            json!("1.21.11"),
            json!("V1_21_11"),
            json!("v1_21_99"),
            json!("v27_1"),
            json!(35),
            json!(true),
            Value::Null,
        ] {
            assert!(renderer_version(Some(&value)).is_none());
        }
        assert!(renderer_version(None).is_none());
        let directory = tempfile::tempdir().unwrap();
        let root = dunce::canonicalize(directory.path()).unwrap();
        let mut local = report(&root, "guiScale:2\n", "{}");
        let size = window(1920, 1080);
        for version in [
            json!("v1_8"),
            json!("v1_12"),
            json!("v1_21_99"),
            json!("1.21.11"),
            json!(35),
            Value::Null,
        ] {
            fs::write(
                root.join("optionsLC.txt"),
                serde_json::to_string(&json!({"guiScale":"2","lastLaunchedVersion":version}))
                    .unwrap(),
            )
            .unwrap();
            assert!(viewport(&local, "Default", &size, &[]).is_err());
        }
        fs::write(root.join("optionsLC.txt"), r#"{"guiScale":"2"}"#).unwrap();
        assert!(viewport(&local, "Default", &size, &[]).is_err());
        local
            .files
            .retain(|file| !file.path.ends_with("optionsLC.txt"));
        assert!(viewport(&local, "Default", &size, &[]).is_err());
        let original = vec![setting("/FPS/x", json!(12.5))];
        assert_eq!(adapt(&original, None, None).unwrap().settings, original);
    }

    #[test]
    fn geometry_and_transformed_offsets_have_finite_bounds() {
        for width in [0.0, 15.0, 32769.0, f64::INFINITY, f64::NAN] {
            let viewport = HudViewport {
                width,
                height: 600.0,
            };
            assert!(viewport.validate().is_err());
            assert!(adapt(&[], Some(&viewport), None).is_err());
        }
        assert!(HudWindow {
            logical_height: 0,
            ..window(1920, 1080)
        }
        .validate()
        .is_err());
        let source = HudViewport {
            width: 16.0,
            height: 16.0,
        };
        let target = HudViewport {
            width: 32768.0,
            height: 32768.0,
        };
        assert!(adapt(
            &[setting("/FPS/x", json!(4294967295.0))],
            Some(&source),
            Some(&target)
        )
        .is_err());
        assert!(!is_adaptable_coordinate(&setting(
            "/WAYPOINTS/x",
            json!(0.5)
        )));
    }
}
