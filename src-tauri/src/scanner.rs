use crate::model::{file_id, ScanFile, ScanReport, ScanRequest};
use std::collections::{BTreeMap, BTreeSet};
use std::fs::{self, File};
use std::io::Read;
use std::path::{Path, PathBuf};

const MAX_FILES: usize = 512;
const MAX_DIRECTORIES: usize = 256;
const MAX_FILE_SIZE: u64 = 8 * 1024 * 1024;
const LUNAR_FILES: &[(&str, &str)] = &[
    ("mods.json", "mods"),
    ("general.json", "general"),
    ("controls.json", "controls"),
    ("performance.json", "performance"),
];

pub fn platform_name() -> String {
    if cfg!(target_os = "windows") {
        "Windows"
    } else if cfg!(target_os = "macos") {
        "macOS"
    } else {
        "Linux"
    }
    .into()
}

pub fn default_candidate_paths(
    platform: &str,
    home: &Path,
    app_data: Option<&Path>,
) -> (Vec<PathBuf>, Vec<PathBuf>) {
    let mut minecraft = Vec::new();
    if platform == "Windows" {
        if let Some(app_data) = app_data {
            minecraft.push(app_data.join(".minecraft"));
        }
        minecraft.push(home.join("AppData").join("Roaming").join(".minecraft"));
    } else if platform == "macOS" {
        minecraft.push(
            home.join("Library")
                .join("Application Support")
                .join("minecraft"),
        );
    }
    minecraft.push(home.join(".minecraft"));
    let mut lunar = vec![home.join(".lunarclient")];
    if let Some(app_data) = app_data {
        lunar.push(app_data.join(".lunarclient"));
        lunar.push(app_data.join("Lunar Client"));
    }
    (minecraft, lunar)
}

pub fn scan(request: ScanRequest) -> Result<ScanReport, String> {
    let home = dirs::home_dir().ok_or("The home folder could not be located.")?;
    let platform = platform_name();
    let app_data = if platform == "Windows" {
        std::env::var_os("APPDATA")
            .map(PathBuf::from)
            .or_else(dirs::data_dir)
    } else {
        dirs::data_dir()
    };
    let (default_minecraft, default_lunar) =
        default_candidate_paths(&platform, &home, app_data.as_deref());
    let minecraft_roots = roots(request.minecraft_root.as_deref(), default_minecraft)?;
    let lunar_roots = roots(request.lunar_root.as_deref(), default_lunar)?;
    let mut report = ScanReport {
        application_icons: crate::icons::detect_application_icons(&platform, &home),
        platform,
        ..Default::default()
    };
    let mut seen = BTreeSet::new();
    let mut versions = BTreeSet::new();
    let mut lunar_profiles = BTreeSet::new();
    for (index, root) in minecraft_roots.iter().enumerate() {
        let profile = if index == 0 {
            "Vanilla".to_owned()
        } else {
            format!("Vanilla {}", index + 1)
        };
        inspect_minecraft(root, &profile, false, &mut report, &mut seen);
        for directory in child_directories(&root.join("versions"), &mut report.warnings) {
            if let Some(version) = directory
                .file_name()
                .and_then(|name| name.to_str())
                .filter(|name| is_version_name(name))
            {
                versions.insert(version.split('-').next().unwrap_or(version).into());
            }
        }
        let mut remaining = MAX_DIRECTORIES;
        for directory in descendant_directories(
            &root.join("versions"),
            2,
            &mut remaining,
            &mut report.warnings,
        ) {
            let label = directory
                .file_name()
                .and_then(|name| name.to_str())
                .unwrap_or("Version");
            inspect_minecraft(
                &directory,
                &format!("Vanilla {label}"),
                false,
                &mut report,
                &mut seen,
            );
        }
    }
    for root in lunar_roots {
        inspect_minecraft(&root, "Lunar Default", true, &mut report, &mut seen);
        let game = if root.join("settings").join("game").is_dir() {
            root.join("settings").join("game")
        } else {
            root.clone()
        };
        inspect_lunar(
            &game,
            "Default",
            &mut report,
            &mut seen,
            &mut lunar_profiles,
        );
        for directory in child_directories(&game, &mut report.warnings) {
            let profile = directory
                .file_name()
                .and_then(|name| name.to_str())
                .unwrap_or("Profile");
            inspect_lunar(
                &directory,
                profile,
                &mut report,
                &mut seen,
                &mut lunar_profiles,
            );
        }
        let profile_root = root.join("profiles");
        let mut remaining = MAX_DIRECTORIES;
        for directory in
            descendant_directories(&profile_root, 3, &mut remaining, &mut report.warnings)
        {
            let name = directory
                .file_name()
                .and_then(|name| name.to_str())
                .unwrap_or("Profile");
            let relative = directory.strip_prefix(&profile_root).unwrap_or(&directory);
            let profile = format!(
                "Lunar {}",
                relative.to_string_lossy().replace(['/', '\\'], " · ")
            );
            inspect_minecraft(&directory, &profile, true, &mut report, &mut seen);
            if is_version_name(name) {
                versions.insert(name.split('-').next().unwrap_or(name).into());
            }
        }
    }
    report.minecraft_versions = versions.into_iter().collect();
    report.lunar_profiles = lunar_profiles.into_iter().collect();
    report.running_processes = detect_running_processes();
    if !report.running_processes.is_empty() {
        report.warnings.push("Minecraft, Lunar Client, or a Java game may be running. Close the game before changing settings.".into());
    }
    report.warnings.sort();
    report.warnings.dedup();
    report.settings.sort_by(|left, right| {
        (
            &left.source,
            &left.profile,
            &left.category,
            &left.group,
            &left.pointer,
        )
            .cmp(&(
                &right.source,
                &right.profile,
                &right.category,
                &right.group,
                &right.pointer,
            ))
    });
    Ok(report)
}

fn roots(override_path: Option<&str>, candidates: Vec<PathBuf>) -> Result<Vec<PathBuf>, String> {
    let requested = if let Some(path) = override_path.filter(|path| !path.trim().is_empty()) {
        if path.len() > 4096 || path.contains('\0') {
            return Err("Choose a valid settings folder.".into());
        }
        vec![PathBuf::from(path)]
    } else {
        candidates
    };
    let mut found = BTreeSet::new();
    for path in requested {
        let Ok(metadata) = fs::symlink_metadata(&path) else {
            continue;
        };
        if metadata.is_dir() && !metadata.file_type().is_symlink() {
            if let Ok(path) = dunce::canonicalize(&path) {
                found.insert(path);
            }
        }
    }
    Ok(found.into_iter().collect())
}

fn safe_directory(path: &Path) -> bool {
    fs::symlink_metadata(path)
        .is_ok_and(|metadata| metadata.is_dir() && !metadata.file_type().is_symlink())
}

fn child_directories(path: &Path, warnings: &mut Vec<String>) -> Vec<PathBuf> {
    if !safe_directory(path) {
        return Vec::new();
    }
    let Ok(entries) = fs::read_dir(path) else {
        return Vec::new();
    };
    let mut children = Vec::new();
    for (index, entry) in entries.flatten().enumerate() {
        if index >= MAX_DIRECTORIES {
            warnings.push("A folder contains too many profiles. Choose a specific settings folder to scan it.".into());
            break;
        }
        if entry
            .file_type()
            .is_ok_and(|kind| kind.is_dir() && !kind.is_symlink())
        {
            children.push(entry.path());
        }
    }
    children.sort();
    children
}

fn descendant_directories(
    path: &Path,
    depth: usize,
    remaining: &mut usize,
    warnings: &mut Vec<String>,
) -> Vec<PathBuf> {
    if depth == 0 || *remaining == 0 {
        return Vec::new();
    }
    let mut found = Vec::new();
    for directory in child_directories(path, warnings) {
        if *remaining == 0 {
            warnings.push(
                "The scan limit was reached. Choose a specific settings folder to continue.".into(),
            );
            break;
        }
        *remaining -= 1;
        found.push(directory.clone());
        found.extend(descendant_directories(
            &directory,
            depth - 1,
            remaining,
            warnings,
        ));
    }
    found
}

fn read_allowed_file(path: &Path) -> Result<(PathBuf, String), String> {
    let metadata =
        fs::symlink_metadata(path).map_err(|_| "A settings file could not be read.".to_string())?;
    if !metadata.is_file() || metadata.file_type().is_symlink() || metadata.len() > MAX_FILE_SIZE {
        return Err("An unsafe or oversized settings file was skipped.".into());
    }
    let canonical = dunce::canonicalize(path)
        .map_err(|_| "A settings file could not be located.".to_string())?;
    if canonical != path {
        return Err("A linked settings file was skipped.".into());
    }
    let mut contents = String::new();
    File::open(&canonical)
        .and_then(|file| file.take(MAX_FILE_SIZE + 1).read_to_string(&mut contents))
        .map_err(|_| "A settings file has an unsupported encoding.".to_string())?;
    if contents.len() as u64 > MAX_FILE_SIZE {
        return Err("An oversized settings file was skipped.".into());
    }
    Ok((canonical, contents))
}

fn inspect_minecraft(
    directory: &Path,
    profile: &str,
    lunar_profile: bool,
    report: &mut ScanReport,
    seen: &mut BTreeSet<PathBuf>,
) {
    if !safe_directory(directory) {
        return;
    }
    let lunar_options = match fs::symlink_metadata(directory.join("optionsLC.txt")) {
        Ok(_) => true,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => false,
        Err(_) => {
            report.warnings.push("Lunar game settings could not be accessed. Check the settings folder and try scanning again.".into());
            true
        }
    };
    if lunar_profile && lunar_options {
        let primary =
            inspect_minecraft_file(directory, profile, "optionsLC.txt", "options", report, seen);
        let legacy = inspect_minecraft_file(
            directory,
            profile,
            "options.txt",
            "options_legacy",
            report,
            seen,
        );
        if let Some((path, content)) = primary {
            let primary = crate::minecraft::parse_file(&content, "options", profile, &path);
            let document = crate::lunar::parse_document(&content);
            match (primary, document) {
                (Ok(settings), Ok(document)) => {
                    let mut settings: BTreeMap<_, _> = settings
                        .into_iter()
                        .map(|setting| (setting.pointer.clone(), setting))
                        .collect();
                    if let Some((path, content)) = legacy {
                        match crate::minecraft::parse_file(&content, "options", profile, &path) {
                            Ok(legacy) => {
                                for setting in legacy {
                                    if !document.as_object().unwrap().contains_key(&setting.pointer)
                                    {
                                        settings.insert(setting.pointer.clone(), setting);
                                    }
                                }
                            }
                            Err(error) => report.warnings.push(error),
                        }
                    }
                    append_minecraft_settings(settings.into_values(), report);
                }
                (Err(error), _) | (_, Err(error)) => report.warnings.push(error),
            }
        }
    } else if let Some((path, content)) =
        inspect_minecraft_file(directory, profile, "options.txt", "options", report, seen)
    {
        match crate::minecraft::parse_file(&content, "options", profile, &path) {
            Ok(settings) => append_minecraft_settings(settings, report),
            Err(error) => report.warnings.push(error),
        }
    }
    let _ = inspect_minecraft_file(
        directory,
        profile,
        "optionsof.txt",
        "optionsof",
        report,
        seen,
    );
    if !lunar_profile && lunar_options {
        if let Some((path, content)) = inspect_minecraft_file(
            directory,
            &format!("Lunar {profile}"),
            "optionsLC.txt",
            "options",
            report,
            seen,
        ) {
            match crate::minecraft::parse_file(
                &content,
                "options",
                &format!("Lunar {profile}"),
                &path,
            ) {
                Ok(settings) => append_minecraft_settings(settings, report),
                Err(error) => report.warnings.push(error),
            }
        }
    }
}

fn append_minecraft_settings(
    settings: impl IntoIterator<Item = crate::model::Setting>,
    report: &mut ScanReport,
) {
    let available = 10000usize.saturating_sub(report.settings.len());
    let settings: Vec<_> = settings.into_iter().collect();
    if settings.len() > available {
        report
            .warnings
            .push("The setting limit was reached. Choose a specific profile.".into());
    }
    report.settings.extend(settings.into_iter().take(available));
}

fn inspect_minecraft_file(
    directory: &Path,
    profile: &str,
    name: &str,
    kind: &str,
    report: &mut ScanReport,
    seen: &mut BTreeSet<PathBuf>,
) -> Option<(PathBuf, String)> {
    let path = directory.join(name);
    if seen.contains(&path) {
        return None;
    }
    if fs::symlink_metadata(&path).is_err_and(|error| error.kind() == std::io::ErrorKind::NotFound)
    {
        return None;
    }
    if report.files.len() >= MAX_FILES {
        report
            .warnings
            .push("The scan file limit was reached. Choose a specific settings folder.".into());
        return None;
    }
    match read_allowed_file(&path) {
        Ok((path, content)) => {
            seen.insert(path.clone());
            report.minecraft_detected = true;
            report.files.push(ScanFile {
                id: file_id("minecraft", profile, kind),
                source: "minecraft".into(),
                path: path.to_string_lossy().into(),
                file_kind: kind.into(),
                profile: profile.into(),
            });
            if kind == "optionsof" {
                report.warnings.push(
                    "OptiFine settings were detected. Their unverified fields remain read-only."
                        .into(),
                );
            }
            Some((path, content))
        }
        Err(error) => {
            report.warnings.push(error);
            None
        }
    }
}

fn inspect_lunar(
    directory: &Path,
    profile: &str,
    report: &mut ScanReport,
    seen: &mut BTreeSet<PathBuf>,
    profiles: &mut BTreeSet<String>,
) {
    if !safe_directory(directory) {
        return;
    }
    for (name, kind) in LUNAR_FILES {
        let path = directory.join(name);
        if !path.exists() || seen.contains(&path) {
            continue;
        }
        if report.files.len() >= MAX_FILES {
            report
                .warnings
                .push("The scan file limit was reached. Choose a specific settings folder.".into());
            return;
        }
        match read_allowed_file(&path) {
            Ok((path, content)) => {
                seen.insert(path.clone());
                report.lunar_detected = true;
                profiles.insert(profile.into());
                report.files.push(ScanFile {
                    id: file_id("lunar", profile, kind),
                    source: "lunar".into(),
                    path: path.to_string_lossy().into(),
                    file_kind: (*kind).into(),
                    profile: profile.into(),
                });
                match crate::lunar::parse_settings(&content, kind, profile) {
                    Ok(settings) => {
                        let available = 10000usize.saturating_sub(report.settings.len());
                        if settings.len() > available {
                            report.warnings.push(
                                "The setting limit was reached. Choose a specific profile.".into(),
                            );
                        }
                        report.settings.extend(settings.into_iter().take(available));
                    }
                    Err(error) => report.warnings.push(error),
                }
            }
            Err(error) => report.warnings.push(error),
        }
    }
}

fn is_version_name(name: &str) -> bool {
    let core = name.split('-').next().unwrap_or(name);
    let parts: Vec<&str> = core.split('.').collect();
    (1..=4).contains(&parts.len())
        && parts.iter().all(|part| {
            !part.is_empty() && part.len() <= 3 && part.chars().all(|c| c.is_ascii_digit())
        })
        && name.len() <= 64
}

pub fn detect_running_processes() -> Vec<String> {
    let system = sysinfo::System::new_all();
    let mut found = BTreeSet::new();
    for process in system.processes().values() {
        let name = process.name().to_string_lossy().to_ascii_lowercase();
        if name.contains("lunar") {
            found.insert("Lunar Client".into());
        }
        if name.contains("minecraft") {
            found.insert("Minecraft".into());
        }
        if matches!(name.as_str(), "java" | "javaw" | "java.exe" | "javaw.exe") {
            found.insert("Java (possible Minecraft)".into());
        }
    }
    found.into_iter().collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn inspect_profile(path: &Path, lunar: bool) -> ScanReport {
        let mut report = ScanReport::default();
        inspect_minecraft(
            path,
            if lunar { "Lunar Test" } else { "Vanilla" },
            lunar,
            &mut report,
            &mut BTreeSet::new(),
        );
        report
    }

    #[test]
    fn lunar_json_values_take_priority_and_legacy_only_keys_remain_available() {
        let temporary = tempfile::tempdir().unwrap();
        let root = dunce::canonicalize(temporary.path()).unwrap();
        let json = "\u{feff}{\r\n\"fov\":\"80.0\",\"guiScale\":\"2\",\"future\":{\"raw\":[1,true]}\r\n}\r\n";
        let text = "fov:-0.5\r\nguiScale:1\r\ngraphicsMode:1\r\nresourcePacks:[\"vanilla\"]\r\n";
        fs::write(root.join("optionsLC.txt"), json).unwrap();
        fs::write(root.join("options.txt"), text).unwrap();
        fs::write(root.join("accounts.json"), [0xff, 0x00]).unwrap();
        let report = inspect_profile(&root, true);
        assert_eq!(report.files.len(), 2);
        assert_eq!(report.files[0].file_kind, "options");
        assert!(report.files[0].path.ends_with("optionsLC.txt"));
        assert_eq!(report.files[1].file_kind, "options_legacy");
        assert!(report.files[1].path.ends_with("options.txt"));
        assert_ne!(report.files[0].id, report.files[1].id);
        let values: BTreeMap<_, _> = report
            .settings
            .iter()
            .map(|setting| (setting.pointer.as_str(), setting.value.as_str().unwrap()))
            .collect();
        assert_eq!(values.len(), report.settings.len());
        assert_eq!(values.get("fov"), Some(&"0.25"));
        assert_eq!(values.get("guiScale"), Some(&"2"));
        assert_eq!(values.get("graphicsMode"), Some(&"1"));
        assert_eq!(values.get("resourcePacks"), Some(&"[\"vanilla\"]"));
        assert!(report.settings.iter().all(|setting| {
            setting.source == "minecraft"
                && setting.file_kind == "options"
                && setting.profile == "Lunar Test"
        }));
        assert_eq!(
            fs::read(root.join("optionsLC.txt")).unwrap(),
            json.as_bytes()
        );
        assert_eq!(fs::read(root.join("options.txt")).unwrap(), text.as_bytes());
        assert_eq!(fs::read(root.join("accounts.json")).unwrap(), [0xff, 0x00]);
    }

    #[test]
    fn legacy_lunar_options_remain_primary_when_json_file_is_absent() {
        let temporary = tempfile::tempdir().unwrap();
        let root = dunce::canonicalize(temporary.path()).unwrap();
        fs::write(root.join("options.txt"), "fov:0.5\r\nguiScale:2\r\n").unwrap();
        let report = inspect_profile(&root, true);
        assert_eq!(report.files.len(), 1);
        assert_eq!(report.files[0].file_kind, "options");
        assert!(report.files[0].path.ends_with("options.txt"));
        assert_eq!(report.settings.len(), 2);
    }

    #[test]
    fn damaged_lunar_json_never_uses_legacy_settings_as_fallback() {
        let temporary = tempfile::tempdir().unwrap();
        let root = dunce::canonicalize(temporary.path()).unwrap();
        let legacy = "fov:0.5\nguiScale:2\n";
        fs::write(root.join("options.txt"), legacy).unwrap();
        for damaged in [
            b"{\"fov\":\"80\"".as_slice(),
            b"[]".as_slice(),
            b"{\"fov\":\"80\",\"fov\":\"90\"}".as_slice(),
            &[0xff],
        ] {
            fs::write(root.join("optionsLC.txt"), damaged).unwrap();
            let report = inspect_profile(&root, true);
            assert!(report.settings.is_empty());
            assert!(!report.warnings.is_empty());
            assert!(report
                .files
                .iter()
                .any(|file| file.file_kind == "options_legacy"));
            assert!(!report
                .files
                .iter()
                .any(|file| file.path.ends_with("options.txt") && file.file_kind == "options"));
            assert_eq!(fs::read(root.join("optionsLC.txt")).unwrap(), damaged);
            assert_eq!(
                fs::read(root.join("options.txt")).unwrap(),
                legacy.as_bytes()
            );
        }
    }

    #[test]
    fn invalid_lunar_values_block_per_key_legacy_fallback() {
        let temporary = tempfile::tempdir().unwrap();
        let root = dunce::canonicalize(temporary.path()).unwrap();
        fs::write(
            root.join("optionsLC.txt"),
            r#"{"fov":"80","guiScale":null,"gamma":"private","future":true}"#,
        )
        .unwrap();
        fs::write(
            root.join("options.txt"),
            "fov:0.5\nguiScale:2\ngamma:1\ngraphicsMode:1\n",
        )
        .unwrap();
        let report = inspect_profile(&root, true);
        let pointers: BTreeSet<_> = report
            .settings
            .iter()
            .map(|setting| setting.pointer.as_str())
            .collect();
        assert_eq!(pointers, BTreeSet::from(["fov", "graphicsMode"]));
    }

    #[test]
    fn lunar_json_in_vanilla_folder_uses_an_independent_profile() {
        let temporary = tempfile::tempdir().unwrap();
        let root = dunce::canonicalize(temporary.path()).unwrap();
        fs::write(root.join("optionsLC.txt"), r#"{"fov":"80"}"#).unwrap();
        fs::write(root.join("options.txt"), "fov:-0.5\n").unwrap();
        let report = inspect_profile(&root, false);
        assert_eq!(report.files.len(), 2);
        assert_eq!(report.settings.len(), 2);
        assert!(report.files.iter().all(|file| file.file_kind == "options"));
        let profiles: BTreeMap<_, _> = report
            .settings
            .iter()
            .map(|setting| (setting.profile.as_str(), setting.value.as_str().unwrap()))
            .collect();
        assert_eq!(profiles.get("Vanilla"), Some(&"-0.5"));
        assert_eq!(profiles.get("Lunar Vanilla"), Some(&"0.25"));
        assert_eq!(
            report
                .files
                .iter()
                .filter(|file| file.path.ends_with("options.txt"))
                .count(),
            1
        );
    }

    #[test]
    fn explicit_lunar_game_folder_is_scanned_without_a_profiles_parent() {
        let temporary = tempfile::tempdir().unwrap();
        let root = dunce::canonicalize(temporary.path()).unwrap();
        fs::write(root.join("optionsLC.txt"), r#"{"guiScale":"2"}"#).unwrap();
        fs::write(root.join("options.txt"), "guiScale:1\n").unwrap();
        let report = scan(ScanRequest {
            minecraft_root: Some(root.join("absent").to_string_lossy().into()),
            lunar_root: Some(root.to_string_lossy().into()),
        })
        .unwrap();
        assert_eq!(report.files.len(), 2);
        assert_eq!(report.settings.len(), 1);
        assert_eq!(report.settings[0].profile, "Lunar Default");
        assert_eq!(report.settings[0].value, serde_json::json!("2"));
    }

    #[cfg(unix)]
    #[test]
    fn linked_and_dangling_lunar_json_files_prevent_legacy_fallback() {
        use std::os::unix::fs::symlink;
        let temporary = tempfile::tempdir().unwrap();
        let root = dunce::canonicalize(temporary.path()).unwrap();
        let external = root.join("external");
        let game = root.join("game");
        fs::create_dir(&game).unwrap();
        fs::write(&external, r#"{"fov":"80"}"#).unwrap();
        fs::write(game.join("options.txt"), "fov:0.5\n").unwrap();
        symlink(&external, game.join("optionsLC.txt")).unwrap();
        for exists in [true, false] {
            if !exists {
                fs::remove_file(&external).unwrap();
            }
            let report = inspect_profile(&game, true);
            assert!(report.settings.is_empty());
            assert!(!report.warnings.is_empty());
            assert_eq!(report.files.len(), 1);
            assert_eq!(report.files[0].file_kind, "options_legacy");
            assert_eq!(fs::read(game.join("options.txt")).unwrap(), b"fov:0.5\n");
        }
    }

    #[test]
    fn version_names_support_year_versions_without_private_suffixes() {
        assert!(is_version_name("26"));
        assert!(is_version_name("26.1"));
        assert!(is_version_name("1.21-PrivateProfile"));
        assert_eq!("1.21-PrivateProfile".split('-').next().unwrap(), "1.21");
        assert!(!is_version_name("privateProfile"));
    }

    #[test]
    fn windows_and_macos_candidates_are_distinct() {
        let home = Path::new("Users/Player");
        let roaming = Path::new("Users/Player/AppData/Roaming");
        let (windows, lunar) = default_candidate_paths("Windows", home, Some(roaming));
        assert!(windows.contains(&roaming.join(".minecraft")));
        assert!(lunar.contains(&home.join(".lunarclient")));
        let (mac, _) = default_candidate_paths("macOS", home, None);
        assert!(mac.contains(&home.join("Library/Application Support/minecraft")));
    }

    #[test]
    fn explicit_unicode_root_reports_native_canonical_file_paths() {
        let directory = tempfile::tempdir().unwrap();
        let root = dunce::canonicalize(directory.path()).unwrap();
        let minecraft = root.join("プレイヤー Settings");
        fs::create_dir(&minecraft).unwrap();
        let path = minecraft.join("options.txt");
        fs::write(&path, "fov:0.5\r\nguiScale:2\r\n").unwrap();
        let report = scan(ScanRequest {
            minecraft_root: Some(minecraft.to_string_lossy().into()),
            lunar_root: Some(root.join("absent").to_string_lossy().into()),
        })
        .unwrap();
        assert_eq!(report.files.len(), 1);
        assert_eq!(
            PathBuf::from(&report.files[0].path),
            dunce::canonicalize(&path).unwrap()
        );
        assert_eq!(
            crate::backup::read_config(Path::new(&report.files[0].path)).unwrap(),
            b"fov:0.5\r\nguiScale:2\r\n"
        );
        assert_eq!(report.settings.len(), 2);
    }

    #[test]
    fn synthetic_roots_are_read_only_and_secrets_are_ignored() {
        let temporary = tempfile::tempdir().unwrap();
        let root = dunce::canonicalize(temporary.path()).unwrap();
        let minecraft = root.join("minecraft");
        let lunar = root.join("lunar");
        let settings = lunar.join("settings/game/Example");
        let game = lunar.join("profiles/1.21");
        fs::create_dir_all(&minecraft).unwrap();
        fs::create_dir_all(&settings).unwrap();
        fs::create_dir_all(&game).unwrap();
        let original = "fov:0.5\nunknownSecret:private\n";
        fs::write(minecraft.join("options.txt"), original).unwrap();
        fs::write(game.join("options.txt"), "guiScale:2\n").unwrap();
        fs::write(
            settings.join("mods.json"),
            r#"{"FPS":{"enabled":true,"x":0.25},"WAYPOINTS":{"enabled":true}}"#,
        )
        .unwrap();
        fs::write(
            lunar.join("accounts.json"),
            "not readable JSON and must be ignored",
        )
        .unwrap();
        let report = scan(ScanRequest {
            minecraft_root: Some(minecraft.to_string_lossy().into()),
            lunar_root: Some(lunar.to_string_lossy().into()),
        })
        .unwrap();
        assert!(report.minecraft_detected && report.lunar_detected);
        assert!(report.minecraft_versions.contains(&"1.21".into()));
        assert_eq!(report.files.len(), 3);
        assert_eq!(report.settings.len(), 4);
        assert_eq!(
            fs::read_to_string(minecraft.join("options.txt")).unwrap(),
            original
        );
        assert!(report
            .files
            .iter()
            .all(|file| !file.path.contains("accounts.json")));
    }

    #[cfg(unix)]
    #[test]
    fn linked_files_and_directories_are_denied() {
        use std::os::unix::fs::symlink;
        let temporary = tempfile::tempdir().unwrap();
        let root = dunce::canonicalize(temporary.path()).unwrap();
        let outside = root.join("outside");
        let minecraft = root.join("minecraft");
        fs::create_dir(&outside).unwrap();
        fs::create_dir(&minecraft).unwrap();
        fs::write(outside.join("options.txt"), "fov:0.5").unwrap();
        symlink(outside.join("options.txt"), minecraft.join("options.txt")).unwrap();
        let report = scan(ScanRequest {
            minecraft_root: Some(minecraft.to_string_lossy().into()),
            lunar_root: Some(root.join("missing").to_string_lossy().into()),
        })
        .unwrap();
        assert!(!report.minecraft_detected);
        assert!(report.settings.is_empty());
        symlink(&outside, root.join("linked-root")).unwrap();
        assert!(
            roots(Some(root.join("linked-root").to_str().unwrap()), Vec::new())
                .unwrap()
                .is_empty()
        );
    }
}
