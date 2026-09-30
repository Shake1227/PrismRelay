use crate::model::{file_id, ScanFile, ScanReport, ScanRequest};
use std::collections::BTreeSet;
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
        inspect_minecraft(root, &profile, &mut report, &mut seen);
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
                &mut report,
                &mut seen,
            );
        }
    }
    for root in lunar_roots {
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
            inspect_minecraft(&directory, &profile, &mut report, &mut seen);
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
    report: &mut ScanReport,
    seen: &mut BTreeSet<PathBuf>,
) {
    if !safe_directory(directory) {
        return;
    }
    for (name, kind) in [("options.txt", "options"), ("optionsof.txt", "optionsof")] {
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
                report.minecraft_detected = true;
                report.files.push(ScanFile {
                    id: file_id("minecraft", profile, kind),
                    source: "minecraft".into(),
                    path: path.to_string_lossy().into(),
                    file_kind: kind.into(),
                    profile: profile.into(),
                });
                if kind == "options" {
                    match crate::minecraft::parse_settings(&content, kind, profile) {
                        Ok(settings) => {
                            let available = 10000usize.saturating_sub(report.settings.len());
                            if settings.len() > available {
                                report.warnings.push(
                                    "The setting limit was reached. Choose a specific profile."
                                        .into(),
                                );
                            }
                            report.settings.extend(settings.into_iter().take(available));
                        }
                        Err(error) => report.warnings.push(error),
                    }
                } else {
                    report.warnings.push("OptiFine settings were detected. Their unverified fields remain read-only.".into());
                }
            }
            Err(error) => report.warnings.push(error),
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
