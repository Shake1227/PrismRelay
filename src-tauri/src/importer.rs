use crate::backup::{self, BackupManifest, BackupStore, FileUpdate};
use crate::codec;
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
}

struct PlannedFile {
    path: PathBuf,
    source: String,
    expected: Vec<u8>,
    current_settings: BTreeMap<String, Setting>,
    settings: Vec<Setting>,
}

struct ImportPlan {
    preview: ImportPreview,
    report: ScanReport,
    files: Vec<PlannedFile>,
}

pub fn preview(
    code: &str,
    selected_ids: &[String],
    target: &ImportTarget,
    request: &ScanRequest,
) -> Result<ImportPreview, String> {
    Ok(plan(code, selected_ids, target, request)?.preview)
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
    if preview_fingerprint.len() != 64
        || !preview_fingerprint.bytes().all(|c| c.is_ascii_hexdigit())
    {
        return Err("Review the import preview before applying these settings.".into());
    }
    let plan = plan(code, selected_ids, target, request)?;
    if plan.preview.fingerprint != preview_fingerprint {
        return Err(
            "Settings changed since the preview. Review the differences again before importing."
                .into(),
        );
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
            crate::minecraft::merge_options(original, &file.settings)?
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
) -> Result<ImportPlan, String> {
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
    let mut unsupported = 0;
    let mut hud_coordinates = false;
    for incoming in envelope
        .settings
        .iter()
        .filter(|setting| selected.contains(setting.id.as_str()))
    {
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
        let current = report.settings.iter().find(|setting| {
            setting.source == incoming.source
                && setting.file_kind == incoming.file_kind
                && setting.pointer == incoming.pointer
                && &setting.profile == profile
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
                    && file.file_kind == current.file_kind
                    && file.profile == current.profile
            })
            .collect();
        if source_files.len() != 1 {
            return Err(
                "The selected profile is ambiguous. Choose a more specific settings folder.".into(),
            );
        }
        let path = PathBuf::from(&source_files[0].path);
        if !files.contains_key(&path) {
            let expected = backup::read_config(&path)?;
            let content = std::str::from_utf8(&expected)
                .map_err(|_| "A settings file has an unsupported encoding.".to_string())?;
            let captured = if current.source == "minecraft" {
                crate::minecraft::parse_settings(content, &current.file_kind, &current.profile)?
            } else {
                crate::lunar::parse_settings(content, &current.file_kind, &current.profile)?
            };
            let current_settings = captured
                .into_iter()
                .map(|setting| (setting.pointer.clone(), setting))
                .collect();
            files.insert(
                path.clone(),
                PlannedFile {
                    path: path.clone(),
                    source: current.source.clone(),
                    expected,
                    current_settings,
                    settings: Vec::new(),
                },
            );
        }
        let current = files
            .get(&path)
            .and_then(|file| file.current_settings.get(&incoming.pointer))
            .cloned();
        let Some(current) = current else {
            unsupported += 1;
            continue;
        };
        if !crate::lunar::same_value_type(&current.value, &incoming.value) {
            unsupported += 1;
            continue;
        }
        let changed = current.value != incoming.value;
        if changed {
            let mut setting = current.clone();
            setting.value = incoming.value.clone();
            files.get_mut(&path).unwrap().settings.push(setting);
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
    if hud_coordinates {
        warnings.push("HUD coordinates will be copied as stored. Their layout can differ on another display; no unverified resolution scaling is applied.".into());
    }
    let mut hasher = Sha256::new();
    hasher.update(b"prism-relay-import-preview-v1");
    hasher.update(Sha256::digest(code.trim().as_bytes()));
    let identity = serde_json::to_vec(&(selected, target, request))
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
            minecraft_profile: "Vanilla".into(),
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
        let code = encode(
            settings,
            ShareMetadata {
                minecraft_version: Some("1.21".into()),
                lunar_version: None,
                platform: "Windows".into(),
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
}
