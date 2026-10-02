use crate::backup::{self, BackupManifest, BackupStore, FileUpdate};
use crate::codec;
use crate::display::PixelSize;
use crate::hud::HudWindow;
use crate::model::{ScanReport, ScanRequest, Setting};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields, default)]
pub struct ImportTarget {
    pub minecraft_profile: String,
    pub lunar_profile: String,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportChange {
    pub id: String,
    pub label: String,
    pub source: String,
    pub category: String,
    pub current: Value,
    pub incoming: Value,
    pub changed: bool,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportPreview {
    pub changes: Vec<ImportChange>,
    pub warnings: Vec<String>,
    pub selected_count: usize,
    pub fingerprint: String,
    pub target_files: Vec<ImportFile>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hud_layout: Option<HudLayoutPreview>,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum HudLayoutMode {
    #[default]
    Auto,
    Preserve,
    Manual,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields, default)]
pub struct HudLayoutRequest {
    pub mode: HudLayoutMode,
    pub window_size: Option<PixelSize>,
}

impl HudLayoutRequest {
    pub fn validate(&self) -> Result<(), String> {
        let valid = match (&self.mode, self.window_size) {
            (HudLayoutMode::Auto | HudLayoutMode::Preserve, None) => true,
            (HudLayoutMode::Manual, Some(size)) => {
                (320..=32768).contains(&size.width) && (240..=32768).contains(&size.height)
            }
            _ => false,
        };
        if valid {
            Ok(())
        } else {
            Err(
                "Enter a window width from 320 to 32768 and a height from 240 to 32768 pixels."
                    .into(),
            )
        }
    }
}

#[derive(Clone, Debug, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HudLayoutContext {
    pub window_size: Option<PixelSize>,
    pub window: Option<HudWindow>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HudLayoutPreview {
    pub status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub window_size: Option<PixelSize>,
    pub adjusted_count: usize,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportFile {
    pub source: String,
    pub profile: String,
    pub file_kind: String,
    pub path: String,
}

struct PlannedFile {
    path: PathBuf,
    source: String,
    expected: Vec<u8>,
    current_settings: BTreeMap<String, Setting>,
    present_keys: BTreeSet<String>,
    lunar_document: Option<Value>,
    settings: Vec<Setting>,
}

struct ImportPlan {
    preview: ImportPreview,
    report: ScanReport,
    files: Vec<PlannedFile>,
}

fn is_gui_setting(setting: &Setting) -> bool {
    setting.source == "minecraft"
        && setting.file_kind == "options"
        && matches!(setting.pointer.as_str(), "guiScale" | "forceUnicodeFont")
}

pub fn preview(
    code: &str,
    selected_ids: &[String],
    target: &ImportTarget,
    request: &ScanRequest,
) -> Result<ImportPreview, String> {
    preview_with_layout(
        code,
        selected_ids,
        target,
        request,
        &HudLayoutRequest {
            mode: HudLayoutMode::Preserve,
            window_size: None,
        },
        &HudLayoutContext::default(),
    )
}

pub fn preview_with_layout(
    code: &str,
    selected_ids: &[String],
    target: &ImportTarget,
    request: &ScanRequest,
    layout: &HudLayoutRequest,
    context: &HudLayoutContext,
) -> Result<ImportPreview, String> {
    Ok(plan(code, selected_ids, target, request, layout, context)?.preview)
}

pub fn apply(
    store: &BackupStore,
    code: &str,
    selected_ids: &[String],
    target: &ImportTarget,
    request: &ScanRequest,
    allow_running: bool,
    preview_fingerprint: &str,
) -> Result<BackupManifest, String> {
    apply_with_layout(
        store,
        code,
        selected_ids,
        target,
        request,
        allow_running,
        preview_fingerprint,
        &HudLayoutRequest {
            mode: HudLayoutMode::Preserve,
            window_size: None,
        },
        &HudLayoutContext::default(),
    )
}

#[allow(clippy::too_many_arguments)]
pub fn apply_with_layout(
    store: &BackupStore,
    code: &str,
    selected_ids: &[String],
    target: &ImportTarget,
    request: &ScanRequest,
    allow_running: bool,
    preview_fingerprint: &str,
    layout: &HudLayoutRequest,
    context: &HudLayoutContext,
) -> Result<BackupManifest, String> {
    if preview_fingerprint.len() != 64
        || !preview_fingerprint.bytes().all(|c| c.is_ascii_hexdigit())
    {
        return Err("Review the import preview before applying these settings.".into());
    }
    let plan = plan(code, selected_ids, target, request, layout, context)?;
    if plan.preview.fingerprint != preview_fingerprint {
        return Err(
            "Settings changed since the preview. Review the differences again before importing."
                .into(),
        );
    }
    if plan
        .preview
        .hud_layout
        .as_ref()
        .is_some_and(|layout| matches!(layout.status.as_str(), "missing-source" | "unavailable"))
    {
        return Err("HUD positions could not be adjusted. Create a new share code or keep the original values.".into());
    }
    if !allow_running && !plan.report.running_processes.is_empty() {
        return Err(
            "Close Minecraft and Lunar Client before importing, or explicitly choose to continue."
                .into(),
        );
    }
    let mut updates = Vec::new();
    for file in plan.files {
        if file.settings.is_empty() {
            continue;
        }
        let original = std::str::from_utf8(&file.expected)
            .map_err(|_| "A settings file has an unsupported encoding.".to_string())?;
        let contents = if file.source == "minecraft" {
            crate::minecraft::merge_file(original, &file.settings, &file.path)?
        } else {
            crate::lunar::merge_settings(original, &file.settings)?
        };
        updates.push(FileUpdate {
            path: file.path,
            expected: file.expected,
            contents: contents.into_bytes(),
        });
    }
    if updates.is_empty() {
        return Err("The selected settings already match this profile.".into());
    }
    store.apply(&plan.report, request, "import", updates)
}

fn plan(
    code: &str,
    selected_ids: &[String],
    target: &ImportTarget,
    request: &ScanRequest,
    layout: &HudLayoutRequest,
    context: &HudLayoutContext,
) -> Result<ImportPlan, String> {
    layout.validate()?;
    let envelope = codec::decode(code)?;
    if selected_ids.is_empty() || selected_ids.len() > codec::MAX_SETTINGS {
        return Err("Select at least one setting to import.".into());
    }
    let selected: BTreeSet<&str> = selected_ids.iter().map(String::as_str).collect();
    if selected.len() != selected_ids.len() {
        return Err("The import selection contains duplicate settings.".into());
    }
    if selected
        .iter()
        .any(|id| !envelope.settings.iter().any(|setting| setting.id == *id))
    {
        return Err("The import selection contains an unknown setting.".into());
    }
    let report = crate::scanner::scan(request.clone())?;
    let mut warnings = report.warnings.clone();
    let mut selected_settings: Vec<Setting> = envelope
        .settings
        .iter()
        .filter(|setting| selected.contains(setting.id.as_str()))
        .cloned()
        .collect();
    let has_layout = selected_settings
        .iter()
        .any(crate::hud::is_adaptable_coordinate);
    let applied_gui_settings = if has_layout
        && layout.mode != HudLayoutMode::Preserve
        && selected_settings.iter().any(is_gui_setting)
    {
        let baseline = plan(
            code,
            selected_ids,
            target,
            request,
            &HudLayoutRequest {
                mode: HudLayoutMode::Preserve,
                window_size: None,
            },
            &HudLayoutContext::default(),
        )?;
        selected_settings
            .iter()
            .filter(|setting| {
                is_gui_setting(setting)
                    && baseline
                        .preview
                        .changes
                        .iter()
                        .any(|change| change.id == setting.id)
            })
            .cloned()
            .collect::<Vec<_>>()
    } else {
        Vec::new()
    };
    let mut resolved_hud_viewport = None;
    let hud_layout = if has_layout && layout.mode != HudLayoutMode::Preserve {
        let mut preview = HudLayoutPreview {
            status: "unavailable".into(),
            window_size: context.window_size,
            adjusted_count: 0,
        };
        if let Some(source) = envelope.metadata.hud_viewport.as_ref() {
            let viewport = context.window.as_ref().and_then(|window| {
                crate::hud::viewport(
                    &report,
                    &target.lunar_profile,
                    window,
                    &applied_gui_settings,
                )
                .ok()
            });
            if let Some(viewport) = viewport {
                let adjustment =
                    crate::hud::adapt(&selected_settings, Some(source), Some(&viewport))?;
                resolved_hud_viewport = Some(viewport);
                preview.adjusted_count = adjustment.adjusted_count;
                preview.status = if adjustment.adjusted_count == 0 {
                    "unchanged"
                } else {
                    "adjusted"
                }
                .into();
                selected_settings = adjustment.settings;
            }
        } else {
            preview.status = "missing-source".into();
        }
        Some(preview)
    } else {
        None
    };
    if envelope
        .metadata
        .minecraft_version
        .as_ref()
        .is_some_and(|version| {
            !report.minecraft_versions.is_empty() && !report.minecraft_versions.contains(version)
        })
    {
        warnings.push("This code comes from another Minecraft version. Only compatible settings already present in the selected profile will be imported.".into());
    }
    let mut changes = Vec::new();
    let mut mapped = BTreeSet::new();
    let mut files = BTreeMap::<PathBuf, PlannedFile>::new();
    let mut target_paths = BTreeSet::new();
    let mut unsupported = 0;
    let mut hud_coordinates = false;
    'settings: for incoming in &selected_settings {
        let profile = if incoming.source == "minecraft" {
            &target.minecraft_profile
        } else {
            &target.lunar_profile
        };
        let key = (
            &incoming.source,
            &incoming.file_kind,
            profile,
            &incoming.pointer,
        );
        if !mapped.insert(key) {
            return Err("Multiple shared profiles target the same setting. Select settings from one shared profile at a time.".into());
        }
        let current = report
            .settings
            .iter()
            .find(|setting| {
                setting.source == incoming.source
                    && setting.file_kind == incoming.file_kind
                    && setting.pointer == incoming.pointer
                    && &setting.profile == profile
            })
            .cloned()
            .or_else(|| {
                if crate::lunar::is_module_enabled_setting(incoming) {
                    let mut current = incoming.clone();
                    current.profile = profile.clone();
                    current.value = Value::Null;
                    Some(current)
                } else {
                    None
                }
            });
        let Some(current) = current else {
            unsupported += 1;
            continue;
        };
        let source_files: Vec<_> = report
            .files
            .iter()
            .filter(|file| {
                file.source == current.source
                    && file.profile == current.profile
                    && (file.file_kind == current.file_kind
                        || (current.source == "minecraft"
                            && current.file_kind == "options"
                            && file.file_kind == "options_legacy"))
            })
            .collect();
        if source_files.is_empty() {
            unsupported += 1;
            continue;
        }
        let paired_options = source_files.len() == 2
            && current.source == "minecraft"
            && source_files.iter().any(|file| {
                file.file_kind == "options"
                    && std::path::Path::new(&file.path).file_name()
                        == Some(std::ffi::OsStr::new("optionsLC.txt"))
            })
            && source_files.iter().any(|file| {
                file.file_kind == "options_legacy"
                    && std::path::Path::new(&file.path).file_name()
                        == Some(std::ffi::OsStr::new("options.txt"))
            })
            && std::path::Path::new(&source_files[0].path).parent()
                == std::path::Path::new(&source_files[1].path).parent();
        if source_files.len() != 1 && !paired_options {
            return Err(
                "The selected profile is ambiguous. Choose a more specific settings folder.".into(),
            );
        }
        let mut destinations = Vec::new();
        for source_file in source_files {
            let path = PathBuf::from(&source_file.path);
            if !files.contains_key(&path) {
                let expected = backup::read_config(&path)?;
                let content = std::str::from_utf8(&expected)
                    .map_err(|_| "A settings file has an unsupported encoding.".to_string())?;
                let captured = if current.source == "minecraft" {
                    crate::minecraft::parse_file(
                        content,
                        &current.file_kind,
                        &current.profile,
                        &path,
                    )?
                } else {
                    crate::lunar::parse_settings(content, &current.file_kind, &current.profile)?
                };
                let current_settings: BTreeMap<_, _> = captured
                    .into_iter()
                    .map(|setting| (setting.pointer.clone(), setting))
                    .collect();
                let present_keys =
                    if path.file_name() == Some(std::ffi::OsStr::new("optionsLC.txt")) {
                        crate::lunar::parse_document(content)?
                            .as_object()
                            .unwrap()
                            .keys()
                            .cloned()
                            .collect()
                    } else {
                        current_settings.keys().cloned().collect()
                    };
                let lunar_document = if current.source == "lunar" {
                    Some(crate::lunar::parse_document(content)?)
                } else {
                    None
                };
                files.insert(
                    path.clone(),
                    PlannedFile {
                        path: path.clone(),
                        source: current.source.clone(),
                        expected,
                        current_settings,
                        present_keys,
                        lunar_document,
                        settings: Vec::new(),
                    },
                );
            }
            let file = files.get(&path).unwrap();
            if path.file_name() == Some(std::ffi::OsStr::new("optionsLC.txt"))
                && file.present_keys.contains(&incoming.pointer)
                && (!file.current_settings.contains_key(&incoming.pointer)
                    || crate::minecraft::validate_file_setting(incoming, &path).is_err())
            {
                unsupported += 1;
                continue 'settings;
            }
            if let Some(captured) = file.current_settings.get(&incoming.pointer) {
                let value = if captured.source == "lunar" {
                    crate::lunar::value_for_existing_type(
                        &incoming.file_kind,
                        &incoming.pointer,
                        &captured.value,
                        &incoming.value,
                    )
                } else if crate::lunar::same_value_type(&captured.value, &incoming.value) {
                    Some(incoming.value.clone())
                } else {
                    None
                };
                let Some(value) = value else {
                    unsupported += 1;
                    continue 'settings;
                };
                destinations.push((path, captured.clone(), value));
            } else if file
                .lunar_document
                .as_ref()
                .is_some_and(|document| crate::lunar::can_insert_module_enabled(document, incoming))
            {
                let mut setting = incoming.clone();
                setting.profile = profile.clone();
                setting.value = Value::Null;
                destinations.push((path, setting, incoming.value.clone()));
            }
        }
        if destinations.is_empty() {
            unsupported += 1;
            continue;
        }
        let changed = destinations
            .iter()
            .any(|(_, setting, value)| setting.value != *value);
        let current = destinations
            .iter()
            .find(|(_, setting, value)| setting.value != *value)
            .or_else(|| {
                destinations.iter().find(|(path, _, _)| {
                    path.file_name() == Some(std::ffi::OsStr::new("optionsLC.txt"))
                })
            })
            .unwrap_or(&destinations[0])
            .1
            .clone();
        for (path, mut setting, value) in destinations {
            target_paths.insert(path.clone());
            if setting.value != value {
                setting.value = value;
                files.get_mut(&path).unwrap().settings.push(setting);
            }
        }
        if incoming.source == "lunar"
            && (incoming.pointer.ends_with("/x") || incoming.pointer.ends_with("/y"))
        {
            hud_coordinates = true;
        }
        changes.push(ImportChange {
            id: incoming.id.clone(),
            label: current.label.clone(),
            source: current.source.clone(),
            category: current.category.clone(),
            current: current.value.clone(),
            incoming: incoming.value.clone(),
            changed,
        });
    }
    if changes.is_empty() {
        return Err("None of the selected settings are available in the destination profile. Choose another profile or reduce the selection.".into());
    }
    if unsupported > 0 {
        warnings.push(format!("{unsupported} selected settings are unavailable or use a different schema. They will be skipped."));
    }
    if hud_coordinates && layout.mode == HudLayoutMode::Preserve {
        warnings.push("HUD coordinates will be copied as stored. Their layout can differ on another display; no unverified resolution scaling is applied.".into());
    }
    let mut hasher = Sha256::new();
    hasher.update(b"prism-relay-import-preview-v1");
    hasher.update(Sha256::digest(code.trim().as_bytes()));
    let identity = serde_json::to_vec(&(
        selected,
        target,
        request,
        layout,
        context,
        &hud_layout,
        &selected_settings,
        &resolved_hud_viewport,
    ))
    .map_err(|_| "The import selection could not be verified.".to_string())?;
    hasher.update((identity.len() as u64).to_be_bytes());
    hasher.update(identity);
    for (path, file) in &files {
        let path = path.to_string_lossy();
        hasher.update((path.len() as u64).to_be_bytes());
        hasher.update(path.as_bytes());
        hasher.update(Sha256::digest(&file.expected));
    }
    warnings.sort();
    warnings.dedup();
    let preview = ImportPreview {
        selected_count: changes.len(),
        changes,
        warnings,
        fingerprint: format!("{:x}", hasher.finalize()),
        hud_layout,
        target_files: target_paths
            .iter()
            .filter_map(|path| {
                report
                    .files
                    .iter()
                    .find(|file| std::path::Path::new(&file.path) == path)
            })
            .map(|file| ImportFile {
                source: file.source.clone(),
                profile: file.profile.clone(),
                file_kind: file.file_kind.clone(),
                path: file.path.clone(),
            })
            .collect(),
    };
    Ok(ImportPlan {
        preview,
        report,
        files: files.into_values().collect(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::codec::{encode, ShareMetadata};
    use std::fs;

    fn fixture() -> (tempfile::TempDir, ScanRequest, ImportTarget, BackupStore) {
        let directory = tempfile::tempdir().unwrap();
        let root = dunce::canonicalize(directory.path()).unwrap();
        let minecraft = root.join("minecraft");
        let lunar = root.join("lunar");
        let profile = lunar.join("settings/game/Destination");
        fs::create_dir_all(&minecraft).unwrap();
        fs::create_dir_all(&profile).unwrap();
        fs::write(
            minecraft.join("options.txt"),
            "fov:0.5\nguiScale:2\nfutureKey:preserved:value\n",
        )
        .unwrap();
        fs::write(profile.join("mods.json"), r#"{"FPS":{"enabled":true,"x":0.2,"unknown":{"marker":"preserved"}},"future":{"data":[1,2]}}"#).unwrap();
        let request = ScanRequest {
            minecraft_root: Some(minecraft.to_string_lossy().into()),
            lunar_root: Some(lunar.to_string_lossy().into()),
        };
        let target = ImportTarget {
            minecraft_profile: "Minecraft".into(),
            lunar_profile: "Destination".into(),
        };
        (
            directory,
            request,
            target,
            BackupStore::new(root.join("backups")),
        )
    }

    fn code(settings: Vec<Setting>) -> (String, Vec<String>) {
        code_for_platform(settings, "Windows")
    }

    fn code_for_platform(settings: Vec<Setting>, platform: &str) -> (String, Vec<String>) {
        let code = encode(
            settings,
            ShareMetadata {
                minecraft_version: Some("1.21".into()),
                lunar_version: None,
                hud_viewport: None,
                platform: platform.into(),
            },
        )
        .unwrap()
        .code;
        let ids = codec::decode(&code)
            .unwrap()
            .settings
            .iter()
            .map(|setting| setting.id.clone())
            .collect();
        (code, ids)
    }

    #[test]
    fn partial_import_remaps_profiles_and_preserves_unknown_fields() {
        let (_directory, request, target, store) = fixture();
        let mut settings =
            crate::minecraft::parse_settings("fov:0.75\nguiScale:4\n", "options", "Other PC")
                .unwrap();
        settings.extend(
            crate::lunar::parse_settings(r#"{"FPS":{"x":0.6}}"#, "mods", "Other Lunar Profile")
                .unwrap(),
        );
        let (code, ids) = code(settings);
        let envelope = codec::decode(&code).unwrap();
        let selected: Vec<String> = envelope
            .settings
            .iter()
            .filter(|setting| setting.pointer != "guiScale")
            .map(|setting| setting.id.clone())
            .collect();
        let before = preview(&code, &selected, &target, &request).unwrap();
        assert_eq!(before.selected_count, 2);
        assert_eq!(before.target_files.len(), 2);
        assert!(before.target_files.iter().any(|file| {
            file.source == "minecraft"
                && file.profile == "Minecraft"
                && file.file_kind == "options"
                && std::path::Path::new(&file.path)
                    == PathBuf::from(request.minecraft_root.as_ref().unwrap()).join("options.txt")
        }));
        assert!(before.target_files.iter().any(|file| {
            file.source == "lunar"
                && file.profile == "Destination"
                && file.file_kind == "mods"
                && std::path::Path::new(&file.path)
                    == PathBuf::from(request.lunar_root.as_ref().unwrap())
                        .join("settings/game/Destination/mods.json")
        }));
        let serialized = serde_json::to_value(&before).unwrap();
        assert!(serialized.get("targetFiles").is_some());
        assert!(serialized["targetFiles"][0].get("fileKind").is_some());
        let _lock = store.lock().unwrap();
        let backup = apply(
            &store,
            &code,
            &selected,
            &target,
            &request,
            true,
            &before.fingerprint,
        )
        .unwrap();
        assert_eq!(backup.files.len(), 2);
        let minecraft = PathBuf::from(request.minecraft_root.as_ref().unwrap()).join("options.txt");
        assert_eq!(
            fs::read_to_string(minecraft).unwrap(),
            "fov:0.75\nguiScale:2\nfutureKey:preserved:value\n"
        );
        let lunar = PathBuf::from(request.lunar_root.as_ref().unwrap())
            .join("settings/game/Destination/mods.json");
        let merged = crate::lunar::parse_document(&fs::read_to_string(lunar).unwrap()).unwrap();
        assert_eq!(merged.pointer("/FPS/x"), Some(&serde_json::json!(0.6)));
        assert_eq!(
            merged.pointer("/FPS/unknown/marker"),
            Some(&serde_json::json!("preserved"))
        );
        assert_eq!(
            merged.pointer("/future/data"),
            Some(&serde_json::json!([1, 2]))
        );
        assert_eq!(ids.len(), 3);
    }

    #[test]
    fn stale_preview_is_rejected_before_backup_or_write() {
        let (_directory, request, target, store) = fixture();
        let settings = crate::minecraft::parse_settings("fov:0.75", "options", "Other PC").unwrap();
        let (code, ids) = code(settings);
        let before = preview(&code, &ids, &target, &request).unwrap();
        let minecraft = PathBuf::from(request.minecraft_root.as_ref().unwrap()).join("options.txt");
        fs::write(&minecraft, "fov:0.6\nguiScale:2\nfutureKey:changed\n").unwrap();
        let _lock = store.lock().unwrap();
        assert!(apply(
            &store,
            &code,
            &ids,
            &target,
            &request,
            true,
            &before.fingerprint
        )
        .is_err());
        assert!(store.list().unwrap().is_empty());
        assert!(fs::read_to_string(&minecraft)
            .unwrap()
            .contains("futureKey:changed"));
    }

    #[test]
    fn duplicate_shared_profile_targets_are_rejected() {
        let (_directory, request, target, _store) = fixture();
        let mut settings =
            crate::minecraft::parse_settings("fov:0.75", "options", "First PC").unwrap();
        settings
            .extend(crate::minecraft::parse_settings("fov:0.25", "options", "Second PC").unwrap());
        let (code, ids) = code(settings);
        assert!(preview(&code, &ids, &target, &request).is_err());
    }

    #[test]
    fn unavailable_settings_are_previewed_as_skipped() {
        let (_directory, request, target, _store) = fixture();
        let settings = crate::minecraft::parse_settings(
            "fov:0.75\nmouseSensitivity:0.4",
            "options",
            "Other PC",
        )
        .unwrap();
        let (code, ids) = code(settings);
        let before = preview(&code, &ids, &target, &request).unwrap();
        assert_eq!(before.selected_count, 1);
        assert!(before
            .warnings
            .iter()
            .any(|warning| warning.contains("will be skipped")));
        assert!(preview(&code, &["unknown".into()], &target, &request).is_err());
    }

    #[test]
    fn sparse_lunar_enabled_imports_are_previewed_applied_twice_and_restored() {
        let (_directory, request, target, store) = fixture();
        let mods = PathBuf::from(request.lunar_root.as_ref().unwrap())
            .join("settings/game/Destination/mods.json");
        let original = "\u{feff}{ \"FPS\":{\"unknown\":0.123456789012345678901}, \"future\":[\"unshared\"] }\r\n";
        fs::write(&mods, original).unwrap();
        let (first, first_ids) = code(
            crate::lunar::parse_settings(r#"{"FPS":{"enabled":true}}"#, "mods", "Windows source")
                .unwrap(),
        );
        let before = preview(&first, &first_ids, &target, &request).unwrap();
        assert_eq!(before.selected_count, 1);
        assert!(before.changes[0].current.is_null());
        assert!(before.changes[0].changed);
        assert_eq!(before.target_files.len(), 1);
        let _lock = store.lock().unwrap();
        let first_backup = apply(
            &store,
            &first,
            &first_ids,
            &target,
            &request,
            true,
            &before.fingerprint,
        )
        .unwrap();
        let first_expected = original.replace(
            "0.123456789012345678901}",
            "0.123456789012345678901,\"enabled\":true}",
        );
        assert_eq!(fs::read_to_string(&mods).unwrap(), first_expected);
        assert_eq!(first_backup.files.len(), 1);
        assert!(
            !preview(&first, &first_ids, &target, &request)
                .unwrap()
                .changes[0]
                .changed
        );
        let (second, second_ids) = code(
            crate::lunar::parse_settings(r#"{"FPS":{"enabled":false}}"#, "mods", "Mac source")
                .unwrap(),
        );
        let before = preview(&second, &second_ids, &target, &request).unwrap();
        assert_eq!(before.changes[0].current, true);
        let second_backup = apply(
            &store,
            &second,
            &second_ids,
            &target,
            &request,
            true,
            &before.fingerprint,
        )
        .unwrap();
        assert_eq!(
            fs::read_to_string(&mods).unwrap(),
            first_expected.replace("\"enabled\":true", "\"enabled\":false")
        );
        let report = crate::scanner::scan(request.clone()).unwrap();
        for backup in [&second_backup, &first_backup] {
            let updates = store.verify(backup, &report).unwrap();
            store.apply(&report, &request, "restore", updates).unwrap();
        }
        assert_eq!(fs::read_to_string(&mods).unwrap(), original);
        assert_eq!(store.pending_backup_id().unwrap(), None);
    }

    #[test]
    fn sparse_lunar_enabled_preview_does_not_overwrite_invalid_existing_types() {
        let (_directory, request, target, store) = fixture();
        let mods = PathBuf::from(request.lunar_root.as_ref().unwrap())
            .join("settings/game/Destination/mods.json");
        let (code, ids) = code(
            crate::lunar::parse_settings(r#"{"FPS":{"enabled":true}}"#, "mods", "source").unwrap(),
        );
        for original in [
            r#"{"FPS":{"enabled":"true"}}"#,
            r#"{"FPS":[]}"#,
            r#"{"CPS":{}}"#,
        ] {
            fs::write(&mods, original).unwrap();
            assert!(preview(&code, &ids, &target, &request).is_err());
            assert_eq!(fs::read_to_string(&mods).unwrap(), original);
            assert!(store.list().unwrap().is_empty());
        }
        fs::write(&mods, r#"{"FPS":{}}"#).unwrap();
        let before = preview(&code, &ids, &target, &request).unwrap();
        fs::write(&mods, r#"{"FPS":{"unknown":"external-change"}}"#).unwrap();
        let _lock = store.lock().unwrap();
        assert!(apply(
            &store,
            &code,
            &ids,
            &target,
            &request,
            true,
            &before.fingerprint
        )
        .is_err());
        assert!(store.list().unwrap().is_empty());
    }

    #[test]
    fn reviewed_lunar_numeric_types_convert_in_preview_apply_and_noop() {
        let (_directory, request, target, store) = fixture();
        let mods = PathBuf::from(request.lunar_root.as_ref().unwrap())
            .join("settings/game/Destination/mods.json");
        let original = r#"{"TOGGLE_SNEAK":{"options":{"flyBoostAmount":1}},"TIME_CHANGER":{"options":{"timeChangerTime":"2"}},"future":0.123456789012345678901}"#;
        fs::write(&mods, original).unwrap();
        let (code, ids) = code(crate::lunar::parse_settings(r#"{"TOGGLE_SNEAK":{"options":{"flyBoostAmount":"3"}},"TIME_CHANGER":{"options":{"timeChangerTime":4}}}"#, "mods", "source").unwrap());
        let before = preview(&code, &ids, &target, &request).unwrap();
        assert_eq!(before.selected_count, 2);
        assert!(before.changes.iter().all(|change| change.changed));
        let _lock = store.lock().unwrap();
        let backup = apply(
            &store,
            &code,
            &ids,
            &target,
            &request,
            true,
            &before.fingerprint,
        )
        .unwrap();
        assert_eq!(
            fs::read_to_string(&mods).unwrap(),
            original
                .replace("\"flyBoostAmount\":1", "\"flyBoostAmount\":3")
                .replace("\"timeChangerTime\":\"2\"", "\"timeChangerTime\":\"4\"")
        );
        let repeated = preview(&code, &ids, &target, &request).unwrap();
        assert!(repeated.changes.iter().all(|change| !change.changed));
        let report = crate::scanner::scan(request.clone()).unwrap();
        let updates = store.verify(&backup, &report).unwrap();
        store.apply(&report, &request, "restore", updates).unwrap();
        assert_eq!(fs::read_to_string(&mods).unwrap(), original);
    }

    #[test]
    fn lunar_game_options_sync_both_formats_for_consecutive_codes_and_restore() {
        let (directory, mut request, target, store) = fixture();
        let game = PathBuf::from(request.lunar_root.as_ref().unwrap()).join("profiles/1.21");
        fs::create_dir_all(&game).unwrap();
        let text_path = game.join("options.txt");
        let lunar_path = game.join("optionsLC.txt");
        let text_original = "\u{feff}fov:0.25\r\nguiScale:2\r\nfullscreen:false\r\nkey_key.jump:key.keyboard.space\r\ngraphicsMode:2\r\nunknown:keep:value\r\n";
        let lunar_original = "\u{feff}{ \"fov\":\"80\", \"guiScale\":\"2\", \"fullscreen\":\"false\", \"key_key.jump\":\"key.keyboard.space\", \"unknown\":0.9000000000000000001 }\r\n";
        fs::write(&text_path, text_original).unwrap();
        fs::write(&lunar_path, lunar_original).unwrap();
        request.minecraft_root = Some(game.to_string_lossy().into());
        let windows = code(crate::minecraft::parse_settings(
            "fov:0.75\r\nguiScale:3\r\nfullscreen:true\r\nkey_key.jump:key.keyboard.r\r\ngraphicsMode:0\r\n",
            "options",
            "Windows game",
        ).unwrap());
        let mac = code_for_platform(crate::minecraft::parse_lunar_options(
            r#"{"fov":"90","guiScale":"4","fullscreen":"false","key_key.jump":"key.keyboard.t"}"#,
            "Mac game",
        ).unwrap(), "macOS");
        let mut backups = Vec::new();
        let mut previous = Vec::new();
        for ((code, ids), degree, ratio, scale) in
            [(&windows, "100", "0.75", "3"), (&mac, "90", "0.5", "4")]
        {
            let before = preview(code, ids, &target, &request).unwrap();
            assert_eq!(before.selected_count, ids.len());
            assert!(before.changes.iter().all(|change| change.changed));
            assert_eq!(before.target_files.len(), 2);
            assert!(before
                .target_files
                .iter()
                .all(|file| file.profile == "Minecraft"));
            previous.push([
                fs::read(&text_path).unwrap(),
                fs::read(&lunar_path).unwrap(),
            ]);
            let _lock = store.lock().unwrap();
            let backup = apply(
                &store,
                code,
                ids,
                &target,
                &request,
                true,
                &before.fingerprint,
            )
            .unwrap();
            assert_eq!(backup.files.len(), 2);
            let text = fs::read_to_string(&text_path).unwrap();
            assert!(text.contains(&format!("fov:{ratio}\r\n")));
            assert!(text.contains(&format!("guiScale:{scale}\r\n")));
            assert!(text.contains("graphicsMode:0\r\nunknown:keep:value\r\n"));
            let lunar = fs::read_to_string(&lunar_path).unwrap();
            assert!(lunar.starts_with('\u{feff}'));
            assert!(lunar.ends_with(" }\r\n"));
            assert!(lunar.contains("\"unknown\":0.9000000000000000001"));
            let document = crate::lunar::parse_document(&lunar).unwrap();
            assert_eq!(document["fov"], degree);
            assert_eq!(document["guiScale"], scale);
            assert!(document.get("graphicsMode").is_none());
            let repeated = preview(code, ids, &target, &request).unwrap();
            assert!(repeated.changes.iter().all(|change| !change.changed));
            assert_eq!(store.pending_backup_id().unwrap(), None);
            backups.push(backup);
        }
        let report = crate::scanner::scan(request.clone()).unwrap();
        for (backup, expected) in backups.iter().zip(&previous).rev() {
            let _lock = store.lock().unwrap();
            let updates = store.verify(backup, &report).unwrap();
            store.apply(&report, &request, "restore", updates).unwrap();
            assert_eq!(fs::read(&text_path).unwrap(), expected[0]);
            assert_eq!(fs::read(&lunar_path).unwrap(), expected[1]);
            assert_eq!(store.pending_backup_id().unwrap(), None);
        }
        assert_eq!(fs::read_to_string(&text_path).unwrap(), text_original);
        assert_eq!(fs::read_to_string(&lunar_path).unwrap(), lunar_original);
        let vanilla = directory.path().join("minecraft/options.txt");
        assert_eq!(
            fs::read_to_string(vanilla).unwrap(),
            "fov:0.5\nguiScale:2\nfutureKey:preserved:value\n"
        );
    }

    #[test]
    fn changed_lunar_mirror_is_previewed_and_both_files_are_guarded() {
        let (_directory, mut request, target, store) = fixture();
        let game = PathBuf::from(request.lunar_root.as_ref().unwrap()).join("profiles/1.21");
        fs::create_dir_all(&game).unwrap();
        let text = game.join("options.txt");
        let lunar = game.join("optionsLC.txt");
        fs::write(&text, "fov:0.25\n").unwrap();
        fs::write(&lunar, r#"{"fov":"100"}"#).unwrap();
        request.minecraft_root = Some(game.to_string_lossy().into());
        let (code, ids) =
            code(crate::minecraft::parse_settings("fov:0.75", "options", "Other").unwrap());
        let before = preview(&code, &ids, &target, &request).unwrap();
        assert!(before.changes[0].changed);
        assert_eq!(before.changes[0].current, "0.25");
        assert_eq!(before.target_files.len(), 2);
        fs::write(&lunar, r#"{"fov":"90"}"#).unwrap();
        let _lock = store.lock().unwrap();
        assert!(apply(
            &store,
            &code,
            &ids,
            &target,
            &request,
            true,
            &before.fingerprint
        )
        .is_err());
        assert!(store.list().unwrap().is_empty());
        let fresh = preview(&code, &ids, &target, &request).unwrap();
        let backup = apply(
            &store,
            &code,
            &ids,
            &target,
            &request,
            true,
            &fresh.fingerprint,
        )
        .unwrap();
        assert_eq!(backup.files.len(), 2);
        assert_eq!(fs::read_to_string(&text).unwrap(), "fov:0.75\n");
        assert_eq!(fs::read_to_string(&lunar).unwrap(), r#"{"fov":"100"}"#);
    }

    #[test]
    fn lunar_options_in_the_selected_game_folder_sync_with_standard_settings() {
        let (_directory, request, target, store) = fixture();
        let minecraft = PathBuf::from(request.minecraft_root.as_ref().unwrap());
        let text_path = minecraft.join("options.txt");
        let lunar = minecraft.join("optionsLC.txt");
        fs::write(&lunar, r#"{"fov":"80"}"#).unwrap();
        let (code, ids) =
            code(crate::minecraft::parse_settings("fov:0.75", "options", "Other").unwrap());
        let before = preview(&code, &ids, &target, &request).unwrap();
        assert_eq!(before.target_files.len(), 2);
        let _lock = store.lock().unwrap();
        let backup = apply(
            &store,
            &code,
            &ids,
            &target,
            &request,
            true,
            &before.fingerprint,
        )
        .unwrap();
        assert_eq!(backup.files.len(), 2);
        assert_eq!(fs::read_to_string(lunar).unwrap(), r#"{"fov":"100"}"#);
        assert_eq!(
            fs::read_to_string(text_path).unwrap(),
            "fov:0.75\nguiScale:2\nfutureKey:preserved:value\n"
        );
    }

    #[test]
    fn lunar_limits_and_unsupported_existing_types_are_skipped_without_partial_mirrors() {
        let (_directory, mut request, target, store) = fixture();
        let game = PathBuf::from(request.lunar_root.as_ref().unwrap()).join("profiles/1.21");
        fs::create_dir_all(&game).unwrap();
        let text = game.join("options.txt");
        let lunar = game.join("optionsLC.txt");
        fs::write(&text, "fov:0.5\nguiScale:2\n").unwrap();
        fs::write(&lunar, r#"{"fov":"90","guiScale":"2"}"#).unwrap();
        request.minecraft_root = Some(game.to_string_lossy().into());
        let (first_code, first_ids) = code(
            crate::minecraft::parse_settings("fov:1.5\nguiScale:3", "options", "Other").unwrap(),
        );
        let before = preview(&first_code, &first_ids, &target, &request).unwrap();
        assert_eq!(before.selected_count, 1);
        assert_eq!(before.changes[0].label, "Gui Scale");
        let _lock = store.lock().unwrap();
        apply(
            &store,
            &first_code,
            &first_ids,
            &target,
            &request,
            true,
            &before.fingerprint,
        )
        .unwrap();
        assert_eq!(fs::read_to_string(&text).unwrap(), "fov:0.5\nguiScale:3\n");
        assert_eq!(
            fs::read_to_string(&lunar).unwrap(),
            r#"{"fov":"90","guiScale":"3"}"#
        );
        fs::write(&lunar, r#"{"fov":90,"guiScale":"3"}"#).unwrap();
        let (code, ids) =
            code(crate::minecraft::parse_settings("fov:0.75", "options", "Other").unwrap());
        assert!(preview(&code, &ids, &target, &request).is_err());
    }

    fn layout_fixture() -> (
        tempfile::TempDir,
        ScanRequest,
        ImportTarget,
        BackupStore,
        HudLayoutContext,
    ) {
        let (directory, request, target, store) = fixture();
        let minecraft = PathBuf::from(request.minecraft_root.as_ref().unwrap());
        fs::write(
            minecraft.join("options.txt"),
            "fov:0.5\nguiScale:2\nforceUnicodeFont:false\nfutureKey:preserved:value\n",
        )
        .unwrap();
        fs::write(
            minecraft.join("optionsLC.txt"),
            r#"{"fov":"90","guiScale":"2","forceUnicodeFont":"false","lastLaunchedVersion":"v1_21_11"}"#,
        )
        .unwrap();
        let profile =
            PathBuf::from(request.lunar_root.as_ref().unwrap()).join("settings/game/Destination");
        fs::write(
            profile.join("general.json"),
            r#"{"useMinecraftScale":"all","highDPIScale":true,"future":123}"#,
        )
        .unwrap();
        fs::write(
            profile.join("mods.json"),
            r#"{"FPS":{"enabled":false,"x":10,"y":20,"position":"topRight","unknown":0.9000000000000000001},"future":{"data":[1,2]}}"#,
        )
        .unwrap();
        let context = HudLayoutContext {
            window_size: Some(PixelSize {
                width: 2400,
                height: 1600,
            }),
            window: Some(HudWindow {
                physical_width: 2400,
                physical_height: 1600,
                logical_height: 800,
                macos: true,
            }),
        };
        (directory, request, target, store, context)
    }

    fn layout_code(x: f64, y: f64, enabled: bool) -> (String, Vec<String>) {
        let settings = crate::lunar::parse_settings(
            &serde_json::json!({"FPS":{"enabled":enabled,"x":x,"y":y,"position":"topRight"}})
                .to_string(),
            "mods",
            "Windows source",
        )
        .unwrap();
        let code = encode(
            settings,
            ShareMetadata {
                minecraft_version: None,
                lunar_version: None,
                platform: "windows".into(),
                hud_viewport: Some(crate::hud::HudViewport {
                    width: 960.0,
                    height: 540.0,
                }),
            },
        )
        .unwrap()
        .code;
        let ids = codec::decode(&code)
            .unwrap()
            .settings
            .into_iter()
            .map(|setting| setting.id)
            .collect();
        (code, ids)
    }

    #[test]
    fn windows_hud_layout_adapts_to_mac_client_and_consecutive_imports_restore_exactly() {
        let (_directory, request, target, store, context) = layout_fixture();
        let path = PathBuf::from(request.lunar_root.as_ref().unwrap())
            .join("settings/game/Destination/mods.json");
        let layout = HudLayoutRequest::default();
        let mut before_second = Vec::new();
        let mut second_backup = None;
        for (index, (x, y, enabled, expected_x, expected_y)) in [
            (100.0, -54.0, true, 250.0, -160.0),
            (64.0, 27.0, false, 160.0, 80.0),
        ]
        .into_iter()
        .enumerate()
        {
            let (code, ids) = layout_code(x, y, enabled);
            let preview =
                preview_with_layout(&code, &ids, &target, &request, &layout, &context).unwrap();
            let preview_layout = preview.hud_layout.as_ref().unwrap();
            assert_eq!(preview_layout.status, "adjusted");
            assert_eq!(preview_layout.adjusted_count, 2);
            assert_eq!(preview_layout.window_size, context.window_size);
            assert!(preview
                .changes
                .iter()
                .any(|change| change.label == "X" && change.incoming.as_f64() == Some(expected_x)));
            if index == 1 {
                before_second = fs::read(&path).unwrap();
            }
            let _lock = store.lock().unwrap();
            let backup = apply_with_layout(
                &store,
                &code,
                &ids,
                &target,
                &request,
                true,
                &preview.fingerprint,
                &layout,
                &context,
            )
            .unwrap();
            assert_eq!(backup.files.len(), 1);
            if index == 1 {
                second_backup = Some(backup);
            }
            let content = fs::read_to_string(&path).unwrap();
            let actual = crate::lunar::parse_document(&content).unwrap();
            assert_eq!(actual["FPS"]["x"].as_f64(), Some(expected_x));
            assert_eq!(actual["FPS"]["y"].as_f64(), Some(expected_y));
            assert_eq!(actual["FPS"]["enabled"], enabled);
            assert_eq!(actual["FPS"]["position"], "topRight");
            assert!(content.contains("\"unknown\":0.9000000000000000001"));
            let repeated =
                preview_with_layout(&code, &ids, &target, &request, &layout, &context).unwrap();
            assert!(repeated.changes.iter().all(|change| !change.changed));
        }
        let report = crate::scanner::scan(request.clone()).unwrap();
        let _lock = store.lock().unwrap();
        let updates = store.verify(&second_backup.unwrap(), &report).unwrap();
        store.apply(&report, &request, "restore", updates).unwrap();
        assert_eq!(fs::read(path).unwrap(), before_second);
    }

    #[test]
    fn unselected_gui_scale_and_monitor_changes_require_a_new_layout_preview() {
        let (_directory, request, target, store, context) = layout_fixture();
        let layout = HudLayoutRequest::default();
        let (code, ids) = layout_code(100.0, -54.0, true);
        let preview =
            preview_with_layout(&code, &ids, &target, &request, &layout, &context).unwrap();
        let mut moved = context.clone();
        moved.window.as_mut().unwrap().physical_width = 2000;
        moved.window_size.as_mut().unwrap().width = 2000;
        assert!(apply_with_layout(
            &store,
            &code,
            &ids,
            &target,
            &request,
            true,
            &preview.fingerprint,
            &layout,
            &moved
        )
        .unwrap_err()
        .contains("changed since the preview"));
        let options = PathBuf::from(request.minecraft_root.as_ref().unwrap()).join("optionsLC.txt");
        fs::write(
            options,
            r#"{"guiScale":"3","forceUnicodeFont":"false","lastLaunchedVersion":"v1_21_11"}"#,
        )
        .unwrap();
        assert!(apply_with_layout(
            &store,
            &code,
            &ids,
            &target,
            &request,
            true,
            &preview.fingerprint,
            &layout,
            &context
        )
        .unwrap_err()
        .contains("changed since the preview"));
    }

    #[test]
    fn zero_offsets_still_bind_the_effective_viewport_to_the_preview() {
        let (_directory, request, target, store, context) = layout_fixture();
        let layout = HudLayoutRequest::default();
        let (code, ids) = layout_code(0.0, 0.0, true);
        let preview =
            preview_with_layout(&code, &ids, &target, &request, &layout, &context).unwrap();
        assert_eq!(preview.hud_layout.as_ref().unwrap().status, "unchanged");
        let options = PathBuf::from(request.minecraft_root.as_ref().unwrap()).join("optionsLC.txt");
        fs::write(
            options,
            r#"{"guiScale":"4","forceUnicodeFont":"false","lastLaunchedVersion":"v1_21_11"}"#,
        )
        .unwrap();
        assert!(apply_with_layout(
            &store,
            &code,
            &ids,
            &target,
            &request,
            true,
            &preview.fingerprint,
            &layout,
            &context
        )
        .unwrap_err()
        .contains("changed since the preview"));
    }

    #[test]
    fn only_applied_gui_settings_affect_hud_adjustment() {
        let (_directory, request, target, store, context) = layout_fixture();
        let layout = HudLayoutRequest::default();
        let (hud_code, _) = layout_code(96.0, 54.0, true);
        let mut envelope = codec::decode(&hud_code).unwrap();
        envelope
            .settings
            .extend(crate::minecraft::parse_settings("guiScale:4\n", "options", "Source").unwrap());
        let code = encode(envelope.settings, envelope.metadata).unwrap().code;
        let ids: Vec<_> = codec::decode(&code)
            .unwrap()
            .settings
            .into_iter()
            .map(|setting| setting.id)
            .collect();
        let mut missing_target = target.clone();
        missing_target.minecraft_profile = "Missing".into();
        let skipped =
            preview_with_layout(&code, &ids, &missing_target, &request, &layout, &context).unwrap();
        assert!(!skipped
            .changes
            .iter()
            .any(|change| change.source == "minecraft"));
        assert!(skipped
            .changes
            .iter()
            .any(|change| change.label == "X" && change.incoming.as_f64() == Some(240.0)));
        let preview =
            preview_with_layout(&code, &ids, &target, &request, &layout, &context).unwrap();
        assert!(preview
            .changes
            .iter()
            .any(|change| change.label == "X" && change.incoming.as_f64() == Some(120.0)));
        let _lock = store.lock().unwrap();
        apply_with_layout(
            &store,
            &code,
            &ids,
            &target,
            &request,
            true,
            &preview.fingerprint,
            &layout,
            &context,
        )
        .unwrap();
        let minecraft = PathBuf::from(request.minecraft_root.as_ref().unwrap());
        assert!(fs::read_to_string(minecraft.join("options.txt"))
            .unwrap()
            .contains("guiScale:4"));
        let mods = PathBuf::from(request.lunar_root.as_ref().unwrap())
            .join("settings/game/Destination/mods.json");
        let document = crate::lunar::parse_document(&fs::read_to_string(mods).unwrap()).unwrap();
        assert_eq!(document["FPS"]["x"].as_f64(), Some(120.0));
        assert_eq!(document["FPS"]["y"].as_f64(), Some(80.0));
    }

    #[test]
    fn legacy_source_and_missing_destination_geometry_need_explicit_preservation() {
        let (_directory, request, target, store, context) = layout_fixture();
        let layout = HudLayoutRequest::default();
        let settings =
            crate::lunar::parse_settings(r#"{"FPS":{"x":100}}"#, "mods", "Source").unwrap();
        let (legacy_code, legacy_ids) = code(settings);
        let preview = preview_with_layout(
            &legacy_code,
            &legacy_ids,
            &target,
            &request,
            &layout,
            &context,
        )
        .unwrap();
        assert_eq!(
            preview.hud_layout.as_ref().unwrap().status,
            "missing-source"
        );
        assert!(apply_with_layout(
            &store,
            &legacy_code,
            &legacy_ids,
            &target,
            &request,
            true,
            &preview.fingerprint,
            &layout,
            &context
        )
        .is_err());
        let preserved = super::preview(&legacy_code, &legacy_ids, &target, &request).unwrap();
        assert_eq!(preserved.changes[0].incoming.as_f64(), Some(100.0));
        assert!(preserved.hud_layout.is_none());
        let (modern_code, modern_ids) = layout_code(100.0, -54.0, true);
        let unavailable = preview_with_layout(
            &modern_code,
            &modern_ids,
            &target,
            &request,
            &layout,
            &HudLayoutContext::default(),
        )
        .unwrap();
        assert_eq!(
            unavailable.hud_layout.as_ref().unwrap().status,
            "unavailable"
        );
        assert!(apply_with_layout(
            &store,
            &modern_code,
            &modern_ids,
            &target,
            &request,
            true,
            &unavailable.fingerprint,
            &layout,
            &HudLayoutContext::default()
        )
        .is_err());
    }

    #[test]
    fn manual_layout_dimensions_and_mode_are_validated() {
        for request in [
            HudLayoutRequest {
                mode: HudLayoutMode::Manual,
                window_size: None,
            },
            HudLayoutRequest {
                mode: HudLayoutMode::Manual,
                window_size: Some(PixelSize {
                    width: 319,
                    height: 240,
                }),
            },
            HudLayoutRequest {
                mode: HudLayoutMode::Auto,
                window_size: Some(PixelSize {
                    width: 1920,
                    height: 1080,
                }),
            },
        ] {
            assert!(request.validate().is_err());
        }
        assert!(HudLayoutRequest {
            mode: HudLayoutMode::Manual,
            window_size: Some(PixelSize {
                width: 1920,
                height: 1000
            })
        }
        .validate()
        .is_ok());
    }
}
