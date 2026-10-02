use crate::model::{ScanReport, ScanRequest};
use chrono::Utc;
use fs2::FileExt;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

const MAX_FILE_SIZE: u64 = 16 * 1024 * 1024;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct BackupFile {
    pub relative_name: String,
    pub original_path: String,
    pub checksum: String,
    pub size: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct BackupManifest {
    pub id: String,
    pub created_at: String,
    pub platform: String,
    pub minecraft_versions: Vec<String>,
    pub lunar_profiles: Vec<String>,
    pub reason: String,
    pub files: Vec<BackupFile>,
    pub status: String,
    pub request: ScanRequest,
}

#[derive(Clone)]
pub struct BackupStore {
    pub root: PathBuf,
}

pub struct FileUpdate {
    pub path: PathBuf,
    pub expected: Vec<u8>,
    pub contents: Vec<u8>,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct TransactionFile {
    original_path: String,
    before_checksum: String,
    after_checksum: String,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Transaction {
    backup_id: String,
    files: Vec<TransactionFile>,
}

pub fn checksum(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

fn verified_config_path(path: &Path) -> Result<PathBuf, String> {
    let metadata =
        fs::symlink_metadata(path).map_err(|_| "設定ファイルを読み取れません。".to_string())?;
    if !metadata.is_file() || metadata.file_type().is_symlink() || metadata.len() > MAX_FILE_SIZE {
        return Err("設定ファイルの形式またはサイズを確認してください。".into());
    }
    let canonical = dunce::canonicalize(path)
        .map_err(|_| "設定ファイルの場所を確認できません。".to_string())?;
    let absolute = if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::env::current_dir()
            .map_err(|e| e.to_string())?
            .join(path)
    };
    if canonical != absolute {
        return Err("リンクを経由する設定ファイルは変更できません。".into());
    }
    for ancestor in absolute.ancestors() {
        if fs::symlink_metadata(ancestor)
            .map(|m| m.file_type().is_symlink())
            .unwrap_or(true)
        {
            return Err("リンクを経由する設定ファイルは変更できません。".into());
        }
    }
    Ok(canonical)
}

pub fn read_config(path: &Path) -> Result<Vec<u8>, String> {
    let path = verified_config_path(path)?;
    let mut bytes = Vec::new();
    File::open(&path)
        .and_then(|f| f.take(MAX_FILE_SIZE + 1).read_to_end(&mut bytes))
        .map_err(|_| "設定ファイルを読み取れません。".to_string())?;
    if bytes.len() as u64 > MAX_FILE_SIZE {
        return Err("設定ファイルが大きすぎます。".into());
    }
    Ok(bytes)
}

fn private_permissions(path: &Path, directory: bool) -> Result<(), String> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(
            path,
            fs::Permissions::from_mode(if directory { 0o700 } else { 0o600 }),
        )
        .map_err(|e| e.to_string())?;
    }
    #[cfg(not(unix))]
    let _ = (path, directory);
    Ok(())
}

pub fn atomic_write(path: &Path, bytes: &[u8]) -> Result<(), String> {
    let parent = path.parent().ok_or("ファイルの保存先が無効です。")?;
    let mut temporary = tempfile::NamedTempFile::new_in(parent).map_err(|e| e.to_string())?;
    temporary.write_all(bytes).map_err(|e| e.to_string())?;
    temporary.as_file().sync_all().map_err(|e| e.to_string())?;
    if let Ok(metadata) = fs::metadata(path) {
        temporary
            .as_file()
            .set_permissions(metadata.permissions())
            .map_err(|e| e.to_string())?;
    }
    let written = fs::read(temporary.path()).map_err(|e| e.to_string())?;
    if checksum(&written) != checksum(bytes) {
        return Err("一時ファイルの検証に失敗しました。".into());
    }
    temporary.persist(path).map_err(|e| e.to_string())?;
    #[cfg(unix)]
    File::open(parent)
        .and_then(|file| file.sync_all())
        .map_err(|e| e.to_string())?;
    Ok(())
}

impl BackupStore {
    pub fn new(root: PathBuf) -> Self {
        Self { root }
    }

    pub fn lock(&self) -> Result<File, String> {
        fs::create_dir_all(&self.root).map_err(|e| e.to_string())?;
        if fs::symlink_metadata(&self.root)
            .map_err(|e| e.to_string())?
            .file_type()
            .is_symlink()
        {
            return Err("バックアップ先にリンクは使用できません。".into());
        }
        private_permissions(&self.root, true)?;
        let file = OpenOptions::new()
            .create(true)
            .truncate(false)
            .read(true)
            .write(true)
            .open(self.root.join("operation.lock"))
            .map_err(|e| e.to_string())?;
        for _ in 0..300 {
            match file.try_lock_exclusive() {
                Ok(()) => return Ok(file),
                Err(error)
                    if error.kind() == std::io::ErrorKind::WouldBlock
                        || error.raw_os_error() == fs2::lock_contended_error().raw_os_error() =>
                {
                    std::thread::sleep(std::time::Duration::from_millis(50))
                }
                Err(_) => return Err("設定処理を保護するロックを取得できません。".into()),
            }
        }
        Err("別の処理が実行中です。完了してから再試行してください。".into())
    }

    fn directory(&self, id: &str) -> Result<PathBuf, String> {
        if id.is_empty()
            || id.len() > 100
            || !id.bytes().all(|c| c.is_ascii_alphanumeric() || c == b'-')
        {
            return Err("バックアップIDが無効です。".into());
        }
        let path = self.root.join(id);
        if let Ok(metadata) = fs::symlink_metadata(&path) {
            if !metadata.is_dir() || metadata.file_type().is_symlink() {
                return Err("バックアップの場所が無効です。".into());
            }
        }
        Ok(path)
    }

    pub fn load(&self, id: &str) -> Result<BackupManifest, String> {
        let directory = self.directory(id)?;
        let bytes = read_config(&directory.join("manifest.json"))?;
        let manifest: BackupManifest = serde_json::from_slice(&bytes)
            .map_err(|_| "バックアップの情報が破損しています。".to_string())?;
        if manifest.id != id || manifest.files.is_empty() || manifest.files.len() > 512 {
            return Err("バックアップの情報が無効です。".into());
        }
        Ok(manifest)
    }

    pub fn list(&self) -> Result<Vec<BackupManifest>, String> {
        if !self.root.exists() {
            return Ok(Vec::new());
        }
        let mut manifests = Vec::new();
        for entry in fs::read_dir(&self.root)
            .map_err(|e| e.to_string())?
            .flatten()
        {
            if entry.file_type().map(|t| t.is_dir()).unwrap_or(false) {
                if let Some(id) = entry.file_name().to_str() {
                    if let Ok(manifest) = self.load(id) {
                        manifests.push(manifest);
                    }
                }
            }
        }
        if let Ok(Some(id)) = self.pending_backup_id() {
            if let Some(manifest) = manifests.iter_mut().find(|m| m.id == id) {
                manifest.status = "recovery-required".into();
            }
        }
        manifests.sort_by(|a, b| b.created_at.cmp(&a.created_at));
        Ok(manifests)
    }

    pub fn snapshot(
        &self,
        report: &ScanReport,
        request: &ScanRequest,
        reason: &str,
        paths: &[PathBuf],
    ) -> Result<BackupManifest, String> {
        if paths.is_empty() || paths.len() > 512 {
            return Err("バックアップする設定がありません。".into());
        }
        let id = format!(
            "{}-{}",
            Utc::now().format("%Y%m%dT%H%M%S%f"),
            std::process::id()
        );
        let directory = self.directory(&id)?;
        fs::create_dir(&directory).map_err(|e| e.to_string())?;
        private_permissions(&directory, true)?;
        let mut files = Vec::new();
        let mut seen = std::collections::HashSet::new();
        for (index, path) in paths.iter().enumerate() {
            let canonical = verified_config_path(path)?;
            if !report.files.iter().any(|f| Path::new(&f.path) == canonical)
                || !seen.insert(canonical.clone())
            {
                return Err("検証されていない設定または重複した設定は保存できません。".into());
            }
            let bytes = read_config(&canonical)?;
            let relative_name = format!("{index:04}.snapshot");
            let snapshot = directory.join(&relative_name);
            atomic_write(&snapshot, &bytes)?;
            private_permissions(&snapshot, false)?;
            files.push(BackupFile {
                relative_name,
                original_path: canonical.to_string_lossy().into_owned(),
                checksum: checksum(&bytes),
                size: bytes.len() as u64,
            });
        }
        let manifest = BackupManifest {
            id,
            created_at: Utc::now().to_rfc3339(),
            platform: report.platform.clone(),
            minecraft_versions: report.minecraft_versions.clone(),
            lunar_profiles: report.lunar_profiles.clone(),
            reason: reason.into(),
            files,
            status: "ready".into(),
            request: request.clone(),
        };
        atomic_write(
            &directory.join("manifest.json"),
            &serde_json::to_vec_pretty(&manifest).map_err(|e| e.to_string())?,
        )?;
        private_permissions(&directory.join("manifest.json"), false)?;
        Ok(manifest)
    }

    pub fn verify(
        &self,
        manifest: &BackupManifest,
        report: &ScanReport,
    ) -> Result<Vec<FileUpdate>, String> {
        let directory = self.directory(&manifest.id)?;
        let mut updates = Vec::new();
        let mut seen = std::collections::HashSet::new();
        for file in &manifest.files {
            if file.relative_name.len() != 13
                || !file.relative_name.ends_with(".snapshot")
                || !file.relative_name[..4].bytes().all(|c| c.is_ascii_digit())
            {
                return Err("バックアップのファイル名が無効です。".into());
            }
            let original = PathBuf::from(&file.original_path);
            let path = verified_config_path(&original)?;
            if !report.files.iter().any(|f| Path::new(&f.path) == path)
                || !seen.insert(path.clone())
            {
                return Err("復元先を検出できません。設定フォルダを確認してください。".into());
            }
            let expected = read_config(&path)?;
            let contents = read_config(&directory.join(&file.relative_name))?;
            if contents.len() as u64 != file.size || checksum(&contents) != file.checksum {
                return Err("バックアップのチェックサムが一致しません。".into());
            }
            updates.push(FileUpdate {
                path,
                expected,
                contents,
            });
        }
        Ok(updates)
    }

    pub fn apply(
        &self,
        report: &ScanReport,
        request: &ScanRequest,
        reason: &str,
        updates: Vec<FileUpdate>,
    ) -> Result<BackupManifest, String> {
        let paths: Vec<PathBuf> = updates.iter().map(|u| u.path.clone()).collect();
        for update in &updates {
            if read_config(&update.path)? != update.expected {
                return Err(
                    "プレビュー後に設定が変更されました。もう一度確認してください。".into(),
                );
            }
            validate_content(&update.path, &update.contents)?;
        }
        let manifest = self.snapshot(report, request, reason, &paths)?;
        let journal = self.root.join("transaction.json");
        let transaction = Transaction {
            backup_id: manifest.id.clone(),
            files: updates
                .iter()
                .zip(&manifest.files)
                .map(|(u, saved)| TransactionFile {
                    original_path: saved.original_path.clone(),
                    before_checksum: checksum(&u.expected),
                    after_checksum: checksum(&u.contents),
                })
                .collect(),
        };
        atomic_write(
            &journal,
            &serde_json::to_vec(&transaction).map_err(|e| e.to_string())?,
        )?;
        for update in &updates {
            let result = read_config(&update.path).and_then(|current| {
                if current != update.expected {
                    return Err("設定が別のアプリで変更されました。".into());
                }
                atomic_write(&update.path, &update.contents)
            });
            if let Err(error) = result {
                let rollback = self.rollback(&manifest, report, &transaction);
                if rollback.is_ok() {
                    let _ = fs::remove_file(&journal);
                }
                return Err(if rollback.is_ok() {
                    format!("変更を取り消しました。{error}")
                } else {
                    "処理が中断されました。バックアップを保護しました。再起動して復旧してください。"
                        .into()
                });
            }
        }
        fs::remove_file(&journal).map_err(|e| e.to_string())?;
        Ok(manifest)
    }

    fn rollback(
        &self,
        manifest: &BackupManifest,
        report: &ScanReport,
        transaction: &Transaction,
    ) -> Result<(), String> {
        if transaction.backup_id != manifest.id || transaction.files.len() != manifest.files.len() {
            return Err("復旧情報を検証できません。".into());
        }
        let mut conflict = false;
        for update in self.verify(manifest, report)? {
            let entry = transaction
                .files
                .iter()
                .find(|f| Path::new(&f.original_path) == update.path)
                .ok_or("復旧先を確認できません。")?;
            if checksum(&update.contents) != entry.before_checksum {
                return Err("復旧ファイルのチェックサムが一致しません。".into());
            }
            let current = checksum(&read_config(&update.path)?);
            if current == entry.before_checksum {
                continue;
            }
            if current != entry.after_checksum {
                conflict = true;
                continue;
            }
            validate_content(&update.path, &update.contents)?;
            atomic_write(&update.path, &update.contents)?;
        }
        if conflict {
            Err("中断後に設定が変更されたため、自動復旧を停止しました。バックアップと現在の設定を保護しています。".into())
        } else {
            Ok(())
        }
    }

    pub fn recover(&self) -> Result<bool, String> {
        let journal = self.root.join("transaction.json");
        if !journal.exists() {
            return Ok(false);
        }
        let transaction: Transaction = serde_json::from_slice(&read_config(&journal)?)
            .map_err(|_| "中断した処理の情報が破損しています。".to_string())?;
        let manifest = self.load(&transaction.backup_id)?;
        let report = crate::scanner::scan(manifest.request.clone())?;
        if !report.running_processes.is_empty() {
            return Err(
                "中断した処理を復旧するため、MinecraftとLunar Clientを終了して再起動してください。"
                    .into(),
            );
        }
        self.rollback(&manifest, &report, &transaction)?;
        fs::remove_file(&journal).map_err(|e| e.to_string())?;
        Ok(true)
    }

    pub fn pending_backup_id(&self) -> Result<Option<String>, String> {
        let journal = self.root.join("transaction.json");
        if !journal.exists() {
            return Ok(None);
        }
        let transaction: Transaction = serde_json::from_slice(&read_config(&journal)?)
            .map_err(|_| "中断した処理の情報が破損しています。".to_string())?;
        Ok(Some(transaction.backup_id))
    }

    pub fn delete(&self, id: &str) -> Result<(), String> {
        if self.root.join("transaction.json").exists() {
            return Err("復旧が必要な処理があるため削除できません。".into());
        }
        self.load(id)?;
        fs::remove_dir_all(self.directory(id)?).map_err(|e| e.to_string())
    }
}

pub fn validate_content(path: &Path, bytes: &[u8]) -> Result<(), String> {
    let content = std::str::from_utf8(bytes)
        .map_err(|_| "設定ファイルの文字コードを確認してください。".to_string())?;
    let name = path
        .file_name()
        .and_then(|s| s.to_str())
        .ok_or("設定ファイル名が無効です。")?;
    if name == "options.txt" || name == "optionsof.txt" {
        if content.contains('\0') {
            return Err("設定ファイルが破損しています。".into());
        }
    } else if name == "optionsLC.txt" {
        crate::minecraft::parse_lunar_options(content, "validation")?;
    } else if matches!(
        name,
        "mods.json" | "general.json" | "controls.json" | "performance.json"
    ) {
        crate::lunar::parse_document(content)?;
    } else {
        return Err("このファイルは設定の書き込みに対応していません。".into());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::ScanFile;

    #[test]
    fn lunar_backup_validation_matches_the_strict_parser_and_preserves_bom() {
        let original = "\u{feff}{\r\n\"FPS\":{\"enabled\":true}\r\n}\r\n";
        for name in [
            "mods.json",
            "general.json",
            "controls.json",
            "performance.json",
        ] {
            assert!(validate_content(Path::new(name), original.as_bytes()).is_ok());
            for invalid in ["[]", "{\"x\":1,\"x\":2}", "{\"x\":{\"y\":1,\"y\":2}}"] {
                assert!(validate_content(Path::new(name), invalid.as_bytes()).is_err());
            }
        }
    }

    #[test]
    fn lunar_game_backups_accept_only_strict_json_objects() {
        let path = Path::new("optionsLC.txt");
        for content in ["{}", r#"{"fov":"80","unknown":[1,2]}"#] {
            assert!(validate_content(path, content.as_bytes()).is_ok());
        }
        for content in [
            "[]",
            "fov:0.25\n",
            r#"{"fov":"70","fov":"80"}"#,
            r#"{"unknown":{"x":1,"x":2}}"#,
            "{\"fov\":\"70\"}\0",
        ] {
            assert!(validate_content(path, content.as_bytes()).is_err());
        }
    }

    struct RecoveryFixture {
        _directory: tempfile::TempDir,
        store: BackupStore,
        request: ScanRequest,
        report: ScanReport,
        paths: Vec<PathBuf>,
        original: Vec<Vec<u8>>,
        imported: Vec<Vec<u8>>,
    }

    impl RecoveryFixture {
        fn new() -> Self {
            let directory = tempfile::tempdir().unwrap();
            let root = dunce::canonicalize(directory.path()).unwrap();
            let minecraft = root.join("minecraft");
            let lunar = root.join("lunar");
            let preset = lunar.join("settings/game/Default");
            fs::create_dir_all(&minecraft).unwrap();
            fs::create_dir_all(&preset).unwrap();
            let paths = vec![minecraft.join("options.txt"), preset.join("mods.json")];
            let original = vec![
                b"fov:0.5\nunknownKey:preserved\n".to_vec(),
                b"{\"FPS\":{\"enabled\":false},\"unknown\":\"preserved\"}\n".to_vec(),
            ];
            let imported = vec![
                b"fov:0.75\nunknownKey:preserved\n".to_vec(),
                b"{\"FPS\":{\"enabled\":true},\"unknown\":\"preserved\"}\n".to_vec(),
            ];
            for (path, contents) in paths.iter().zip(&original) {
                fs::write(path, contents).unwrap();
            }
            let request = ScanRequest {
                minecraft_root: Some(minecraft.to_string_lossy().into()),
                lunar_root: Some(lunar.to_string_lossy().into()),
            };
            let report = crate::scanner::scan(request.clone()).unwrap();
            assert_eq!(report.files.len(), 2);
            Self {
                _directory: directory,
                store: BackupStore::new(root.join("backups")),
                request,
                report,
                paths,
                original,
                imported,
            }
        }

        fn persist_interrupted_import(&self) -> BackupManifest {
            let manifest = self
                .store
                .snapshot(&self.report, &self.request, "import", &self.paths)
                .unwrap();
            let transaction = Transaction {
                backup_id: manifest.id.clone(),
                files: self
                    .paths
                    .iter()
                    .zip(&self.original)
                    .zip(&self.imported)
                    .map(|((path, before), after)| TransactionFile {
                        original_path: path.to_string_lossy().into(),
                        before_checksum: checksum(before),
                        after_checksum: checksum(after),
                    })
                    .collect(),
            };
            atomic_write(
                &self.store.root.join("transaction.json"),
                &serde_json::to_vec(&transaction).unwrap(),
            )
            .unwrap();
            atomic_write(&self.paths[0], &self.imported[0]).unwrap();
            manifest
        }
    }

    fn fixture() -> (tempfile::TempDir, BackupStore, ScanReport, PathBuf) {
        let directory = tempfile::tempdir().unwrap();
        let canonical = dunce::canonicalize(directory.path()).unwrap();
        let path = canonical.join("options.txt");
        fs::write(&path, "fov:0.5\nunknownKey:preserved\n").unwrap();
        let store = BackupStore::new(canonical.join("backups"));
        let report = ScanReport {
            files: vec![ScanFile {
                id: "test".into(),
                source: "minecraft".into(),
                path: path.to_string_lossy().into(),
                file_kind: "options".into(),
                profile: "vanilla".into(),
            }],
            ..Default::default()
        };
        (directory, store, report, path)
    }

    #[test]
    fn contended_operation_lock_waits_until_the_owner_releases_it() {
        let directory = tempfile::tempdir().unwrap();
        let root = dunce::canonicalize(directory.path()).unwrap();
        let store = BackupStore::new(root.join("backups"));
        let held = store.lock().unwrap();
        let (sender, receiver) = std::sync::mpsc::channel();
        let waiting_store = store.clone();
        let waiter = std::thread::spawn(move || {
            let result = waiting_store.lock().map(drop);
            sender.send(result).unwrap();
        });
        assert!(matches!(
            receiver.recv_timeout(std::time::Duration::from_millis(100)),
            Err(std::sync::mpsc::RecvTimeoutError::Timeout)
        ));
        drop(held);
        receiver
            .recv_timeout(std::time::Duration::from_secs(3))
            .unwrap()
            .unwrap();
        waiter.join().unwrap();
    }

    #[test]
    fn atomic_replacement_handles_unicode_paths_and_leaves_no_temporary_files() {
        let directory = tempfile::tempdir().unwrap();
        let root = dunce::canonicalize(directory.path()).unwrap();
        let folder = root.join("プレイヤー Settings");
        fs::create_dir(&folder).unwrap();
        let path = folder.join("options.txt");
        atomic_write(&path, b"fov:0.5\r\nunknown:preserved\r\n").unwrap();
        atomic_write(&path, b"fov:0.75\r\nunknown:preserved\r\n").unwrap();
        assert_eq!(
            read_config(&path).unwrap(),
            b"fov:0.75\r\nunknown:preserved\r\n"
        );
        let files = fs::read_dir(&folder)
            .unwrap()
            .collect::<Result<Vec<_>, _>>()
            .unwrap();
        assert_eq!(files.len(), 1);
        assert_eq!(files[0].file_name(), "options.txt");
    }

    #[cfg(any(target_os = "macos", windows))]
    #[test]
    fn failed_second_replacement_restores_first_and_preserves_backup() {
        let fixture = RecoveryFixture::new();
        let _lock = fixture.store.lock().unwrap();
        #[cfg(target_os = "macos")]
        let permissions = fs::metadata(fixture.paths[1].parent().unwrap())
            .unwrap()
            .permissions();
        #[cfg(target_os = "macos")]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(
                fixture.paths[1].parent().unwrap(),
                fs::Permissions::from_mode(0o500),
            )
            .unwrap();
        }
        #[cfg(windows)]
        let blocked = {
            use std::os::windows::fs::OpenOptionsExt;
            OpenOptions::new()
                .read(true)
                .share_mode(3)
                .open(&fixture.paths[1])
                .unwrap()
        };
        let updates = fixture
            .paths
            .iter()
            .zip(&fixture.original)
            .zip(&fixture.imported)
            .map(|((path, before), after)| FileUpdate {
                path: path.clone(),
                expected: before.clone(),
                contents: after.clone(),
            })
            .collect();
        let result = fixture
            .store
            .apply(&fixture.report, &fixture.request, "import", updates);
        #[cfg(target_os = "macos")]
        fs::set_permissions(fixture.paths[1].parent().unwrap(), permissions).unwrap();
        #[cfg(windows)]
        drop(blocked);
        let error = result.unwrap_err();
        assert!(error.contains("変更を取り消しました"), "{error}");
        for (path, original) in fixture.paths.iter().zip(&fixture.original) {
            assert_eq!(read_config(path).unwrap(), *original);
        }
        assert_eq!(fixture.store.pending_backup_id().unwrap(), None);
        assert!(!fixture.store.root.join("transaction.json").exists());
        let backups = fixture.store.list().unwrap();
        assert_eq!(backups.len(), 1);
        let updates = fixture.store.verify(&backups[0], &fixture.report).unwrap();
        for (update, original) in updates.iter().zip(&fixture.original) {
            assert_eq!(update.contents, *original);
        }
        for path in &fixture.paths {
            let parent = path.parent().unwrap();
            assert!(!fs::read_dir(parent).unwrap().any(|entry| entry
                .unwrap()
                .file_name()
                .to_string_lossy()
                .starts_with(".tmp")));
        }
    }

    #[test]
    fn consecutive_windows_codes_update_default_game_settings_and_restore() {
        let directory = tempfile::tempdir().unwrap();
        let root = dunce::canonicalize(directory.path()).unwrap();
        let minecraft = root.join("Library/Application Support/minecraft");
        let lunar = root.join("プレイヤー Settings/.lunarclient");
        let game = lunar.join("profiles/1.21.11");
        let chosen_lunar = lunar.join("settings/game/Zulu Selected");
        let other_lunar = lunar.join("settings/game/Alpha Other");
        for folder in [&minecraft, &game, &chosen_lunar, &other_lunar] {
            fs::create_dir_all(folder).unwrap();
        }
        let paths = [
            minecraft.join("options.txt"),
            chosen_lunar.join("mods.json"),
        ];
        let original = [
            b"fov:0.5\r\nguiScale:2\r\nunknownKey:preserved\r\n".to_vec(),
            b"{\"FPS\":{\"enabled\":false,\"x\":0.25},\"unshared\":{\"value\":7}}\r\n".to_vec(),
        ];
        let other_options = game.join("options.txt");
        let other_mods = other_lunar.join("mods.json");
        fs::write(&other_options, &original[0]).unwrap();
        fs::write(&other_mods, &original[1]).unwrap();
        for (path, bytes) in paths.iter().zip(&original) {
            fs::write(path, bytes).unwrap();
        }
        let request = ScanRequest {
            minecraft_root: Some(minecraft.to_string_lossy().into()),
            lunar_root: Some(lunar.to_string_lossy().into()),
        };
        let target = crate::importer::ImportTarget {
            minecraft_profile: "Minecraft".into(),
            lunar_profile: "Zulu Selected".into(),
        };
        let store = BackupStore::new(root.join("backups"));
        let mut backups = Vec::new();
        let mut imported = Vec::new();
        for (fov, scale, enabled, x) in [("0.75", "3", true, 0.5), ("0.9", "4", false, 0.75)] {
            let mut settings = crate::minecraft::parse_settings(
                &format!("fov:{fov}\r\nguiScale:{scale}\r\n"),
                "options",
                "Windows game profile",
            )
            .unwrap();
            settings.extend(
                crate::lunar::parse_settings(
                    &format!(r#"{{"FPS":{{"enabled":{enabled},"x":{x}}}}}"#),
                    "mods",
                    "Windows Lunar profile",
                )
                .unwrap(),
            );
            let code = crate::codec::encode(
                settings,
                crate::codec::ShareMetadata {
                    hud_viewport: None,
                    minecraft_version: Some("1.21.11".into()),
                    lunar_version: None,
                    platform: "Windows".into(),
                },
            )
            .unwrap()
            .code;
            let envelope = crate::codec::decode(&code).unwrap();
            assert_eq!(envelope.metadata.platform, "windows");
            let ids = envelope
                .settings
                .iter()
                .map(|setting| setting.id.clone())
                .collect::<Vec<_>>();
            let preview = crate::importer::preview(&code, &ids, &target, &request).unwrap();
            assert_eq!(preview.selected_count, 4);
            assert!(preview.changes.iter().all(|change| change.changed));
            let _lock = store.lock().unwrap();
            let backup = crate::importer::apply(
                &store,
                &code,
                &ids,
                &target,
                &request,
                true,
                &preview.fingerprint,
            )
            .unwrap();
            assert_eq!(backup.files.len(), 2);
            let options = String::from_utf8(read_config(&paths[0]).unwrap()).unwrap();
            assert_eq!(
                options,
                format!("fov:{fov}\r\nguiScale:{scale}\r\nunknownKey:preserved\r\n")
            );
            let mods = crate::lunar::parse_document(
                &String::from_utf8(read_config(&paths[1]).unwrap()).unwrap(),
            )
            .unwrap();
            assert_eq!(
                mods.pointer("/FPS/enabled"),
                Some(&serde_json::json!(enabled))
            );
            assert_eq!(mods.pointer("/FPS/x"), Some(&serde_json::json!(x)));
            assert_eq!(mods.pointer("/unshared/value"), Some(&serde_json::json!(7)));
            assert_eq!(read_config(&other_options).unwrap(), original[0]);
            assert_eq!(read_config(&other_mods).unwrap(), original[1]);
            let repeated = crate::importer::preview(&code, &ids, &target, &request).unwrap();
            assert_eq!(repeated.selected_count, 4);
            assert!(repeated.changes.iter().all(|change| !change.changed));
            assert_eq!(store.list().unwrap().len(), backups.len() + 1);
            assert_eq!(store.pending_backup_id().unwrap(), None);
            imported.push(
                paths
                    .iter()
                    .map(|path| read_config(path).unwrap())
                    .collect::<Vec<_>>(),
            );
            backups.push(backup);
        }
        let report = crate::scanner::scan(request.clone()).unwrap();
        let first_expected = original.to_vec();
        for (backup, expected) in [(&backups[1], &imported[0]), (&backups[0], &first_expected)] {
            let _lock = store.lock().unwrap();
            let updates = store.verify(backup, &report).unwrap();
            store.apply(&report, &request, "restore", updates).unwrap();
            for (path, bytes) in paths.iter().zip(expected) {
                assert_eq!(read_config(path).unwrap(), *bytes);
            }
            assert_eq!(read_config(&other_options).unwrap(), original[0]);
            assert_eq!(read_config(&other_mods).unwrap(), original[1]);
            assert_eq!(store.pending_backup_id().unwrap(), None);
        }
        assert_eq!(store.list().unwrap().len(), 4);
    }

    #[cfg(windows)]
    #[test]
    fn windows_locked_replacement_keeps_original_and_can_be_retried_twice() {
        use std::os::windows::fs::OpenOptionsExt;
        let directory = tempfile::tempdir().unwrap();
        let root = dunce::canonicalize(directory.path()).unwrap();
        let path = root.join("options.txt");
        let mut current = b"fov:0.5\r\nunknown:preserved\r\n".to_vec();
        fs::write(&path, &current).unwrap();
        for contents in [
            b"fov:0.75\r\nunknown:preserved\r\n".as_slice(),
            b"fov:1.0\r\nunknown:preserved\r\n".as_slice(),
        ] {
            let held = OpenOptions::new()
                .read(true)
                .share_mode(3)
                .open(&path)
                .unwrap();
            assert!(atomic_write(&path, contents).is_err());
            assert_eq!(read_config(&path).unwrap(), current);
            drop(held);
            atomic_write(&path, contents).unwrap();
            assert_eq!(read_config(&path).unwrap(), contents);
            assert_eq!(fs::read_dir(&root).unwrap().count(), 1);
            current = contents.to_vec();
        }
    }

    #[test]
    fn backup_paths_remain_canonical_across_saved_manifests_and_restore() {
        let fixture = RecoveryFixture::new();
        let _lock = fixture.store.lock().unwrap();
        let mut paths = fixture.paths.clone();
        #[cfg(windows)]
        for path in &mut paths {
            *path = PathBuf::from(path.to_string_lossy().replace('\\', "/"));
        }
        #[cfg(not(windows))]
        let _ = &mut paths;
        let manifest = fixture
            .store
            .snapshot(&fixture.report, &fixture.request, "manual", &paths)
            .unwrap();
        for (saved, original) in manifest.files.iter().zip(&paths) {
            assert_eq!(
                saved.original_path,
                dunce::canonicalize(original).unwrap().to_string_lossy()
            );
        }
        let mut saved = fixture.store.load(&manifest.id).unwrap();
        #[cfg(windows)]
        for file in &mut saved.files {
            file.original_path = file.original_path.replace('\\', "/");
        }
        #[cfg(not(windows))]
        let _ = &mut saved;
        let report = crate::scanner::scan(fixture.request.clone()).unwrap();
        let updates = fixture.store.verify(&saved, &report).unwrap();
        assert_eq!(updates.len(), 2);
        assert!(updates
            .iter()
            .all(|update| update.path == dunce::canonicalize(&update.path).unwrap()));
    }

    #[cfg(unix)]
    #[test]
    fn restore_canonicalization_still_rejects_links_to_approved_files() {
        let (_directory, store, report, path) = fixture();
        let _lock = store.lock().unwrap();
        let mut manifest = store
            .snapshot(
                &report,
                &ScanRequest::default(),
                "manual",
                std::slice::from_ref(&path),
            )
            .unwrap();
        let linked = path.parent().unwrap().join("linked-options.txt");
        std::os::unix::fs::symlink(&path, &linked).unwrap();
        manifest.files[0].original_path = linked.to_string_lossy().into();
        assert!(store.verify(&manifest, &report).is_err());
        assert!(store
            .snapshot(&report, &ScanRequest::default(), "manual", &[linked])
            .is_err());
        assert_eq!(
            read_config(&path).unwrap(),
            b"fov:0.5\nunknownKey:preserved\n"
        );
    }

    #[test]
    fn backup_restore_and_checksum() {
        let (_directory, store, report, path) = fixture();
        let _lock = store.lock().unwrap();
        let original = read_config(&path).unwrap();
        let backup = store
            .apply(
                &report,
                &ScanRequest::default(),
                "import",
                vec![FileUpdate {
                    path: path.clone(),
                    expected: original.clone(),
                    contents: b"fov:0.75\nunknownKey:preserved\n".to_vec(),
                }],
            )
            .unwrap();
        assert_ne!(read_config(&path).unwrap(), original);
        let updates = store.verify(&backup, &report).unwrap();
        store
            .apply(&report, &ScanRequest::default(), "restore", updates)
            .unwrap();
        assert_eq!(read_config(&path).unwrap(), original);
        assert_eq!(store.list().unwrap().len(), 2);
        fs::write(
            store.directory(&backup.id).unwrap().join("0000.snapshot"),
            "broken",
        )
        .unwrap();
        assert!(store.verify(&backup, &report).is_err());
    }

    #[test]
    fn refuses_stale_or_invalid_writes() {
        let (_directory, store, report, path) = fixture();
        let _lock = store.lock().unwrap();
        assert!(store
            .apply(
                &report,
                &ScanRequest::default(),
                "import",
                vec![FileUpdate {
                    path: path.clone(),
                    expected: vec![],
                    contents: b"fov:1".to_vec()
                }]
            )
            .is_err());
        assert!(store.directory("../../settings").is_err());
        assert!(validate_content(Path::new("accounts.json"), b"{}").is_err());
        assert!(store
            .verify(
                &BackupManifest {
                    id: "valid".into(),
                    created_at: "".into(),
                    platform: "".into(),
                    minecraft_versions: vec![],
                    lunar_profiles: vec![],
                    reason: "".into(),
                    status: "ready".into(),
                    request: ScanRequest::default(),
                    files: vec![BackupFile {
                        relative_name: "../../../data".into(),
                        original_path: path.to_string_lossy().into(),
                        checksum: "".into(),
                        size: 0
                    }]
                },
                &report
            )
            .is_err());
    }

    #[test]
    fn rollback_preserves_external_changes() {
        let (_directory, store, report, path) = fixture();
        let _lock = store.lock().unwrap();
        let original = read_config(&path).unwrap();
        let backup = store
            .snapshot(
                &report,
                &ScanRequest::default(),
                "import",
                std::slice::from_ref(&path),
            )
            .unwrap();
        let imported = b"fov:0.75\nunknownKey:preserved\n";
        let transaction = Transaction {
            backup_id: backup.id.clone(),
            files: vec![TransactionFile {
                original_path: path.to_string_lossy().into(),
                before_checksum: checksum(&original),
                after_checksum: checksum(imported),
            }],
        };
        fs::write(&path, imported).unwrap();
        store.rollback(&backup, &report, &transaction).unwrap();
        assert_eq!(read_config(&path).unwrap(), original);
        fs::write(&path, "fov:0.9\nexternal:new\n").unwrap();
        assert!(store.rollback(&backup, &report, &transaction).is_err());
        assert_eq!(read_config(&path).unwrap(), b"fov:0.9\nexternal:new\n");
    }

    #[test]
    fn persisted_partial_transaction_recovers_after_store_restart() {
        let fixture = RecoveryFixture::new();
        let lock = fixture.store.lock().unwrap();
        let manifest = fixture.persist_interrupted_import();
        assert_eq!(read_config(&fixture.paths[0]).unwrap(), fixture.imported[0]);
        assert_eq!(read_config(&fixture.paths[1]).unwrap(), fixture.original[1]);
        drop(lock);
        let restarted = BackupStore::new(fixture.store.root.clone());
        let _lock = restarted.lock().unwrap();
        assert_eq!(
            restarted.pending_backup_id().unwrap(),
            Some(manifest.id.clone())
        );
        assert_eq!(restarted.list().unwrap()[0].status, "recovery-required");
        match restarted.recover() {
            Ok(recovered) => {
                assert!(recovered);
                assert_eq!(read_config(&fixture.paths[0]).unwrap(), fixture.original[0]);
                assert_eq!(read_config(&fixture.paths[1]).unwrap(), fixture.original[1]);
                assert!(!restarted.root.join("transaction.json").exists());
                assert_eq!(restarted.pending_backup_id().unwrap(), None);
                assert!(!restarted.recover().unwrap());
                assert_eq!(restarted.list().unwrap()[0].status, "ready");
            }
            Err(message) => {
                assert!(
                    message.contains("MinecraftとLunar Clientを終了"),
                    "{message}"
                );
                assert_eq!(read_config(&fixture.paths[0]).unwrap(), fixture.imported[0]);
                assert_eq!(read_config(&fixture.paths[1]).unwrap(), fixture.original[1]);
                assert!(restarted.root.join("transaction.json").exists());
            }
        }
    }

    #[test]
    fn persisted_recovery_preserves_external_edits_and_keeps_journal() {
        let fixture = RecoveryFixture::new();
        let lock = fixture.store.lock().unwrap();
        let manifest = fixture.persist_interrupted_import();
        let external = b"guiScale:4\nunknownVersionKey:external-change\n";
        fs::write(&fixture.paths[1], external).unwrap();
        drop(lock);
        let restarted = BackupStore::new(fixture.store.root.clone());
        let _lock = restarted.lock().unwrap();
        let message = restarted.recover().unwrap_err();
        assert_eq!(read_config(&fixture.paths[1]).unwrap(), external);
        if message.contains("MinecraftとLunar Clientを終了") {
            assert_eq!(read_config(&fixture.paths[0]).unwrap(), fixture.imported[0]);
        } else {
            assert!(message.contains("中断後に設定が変更"), "{message}");
            assert_eq!(read_config(&fixture.paths[0]).unwrap(), fixture.original[0]);
        }
        assert!(restarted.root.join("transaction.json").exists());
        assert_eq!(
            restarted.pending_backup_id().unwrap(),
            Some(manifest.id.clone())
        );
        assert_eq!(restarted.list().unwrap()[0].status, "recovery-required");
        assert!(restarted.delete(&manifest.id).is_err());
        assert_eq!(restarted.load(&manifest.id).unwrap().id, manifest.id);
    }

    #[test]
    fn persisted_journal_cannot_escape_the_backup_or_scanned_paths() {
        let fixture = RecoveryFixture::new();
        let _lock = fixture.store.lock().unwrap();
        let manifest = fixture.persist_interrupted_import();
        let journal = fixture.store.root.join("transaction.json");
        let mut transaction: Transaction =
            serde_json::from_slice(&read_config(&journal).unwrap()).unwrap();
        let outside = fixture.store.root.parent().unwrap().join("outside.txt");
        let protected = b"unrelated local file";
        fs::write(&outside, protected).unwrap();
        transaction.backup_id = "../../outside".into();
        atomic_write(&journal, &serde_json::to_vec(&transaction).unwrap()).unwrap();
        let message = fixture.store.recover().unwrap_err();
        assert!(message.contains("バックアップIDが無効"), "{message}");
        assert_eq!(fs::read(&outside).unwrap(), protected);
        assert_eq!(read_config(&fixture.paths[0]).unwrap(), fixture.imported[0]);
        transaction.backup_id = manifest.id;
        transaction.files[0].original_path = outside.to_string_lossy().into();
        atomic_write(&journal, &serde_json::to_vec(&transaction).unwrap()).unwrap();
        let message = fixture.store.recover().unwrap_err();
        assert!(
            message.contains("復旧先を確認") || message.contains("MinecraftとLunar Clientを終了"),
            "{message}"
        );
        assert_eq!(fs::read(&outside).unwrap(), protected);
        assert_eq!(read_config(&fixture.paths[0]).unwrap(), fixture.imported[0]);
        assert_eq!(read_config(&fixture.paths[1]).unwrap(), fixture.original[1]);
        assert!(journal.exists());
    }
}
