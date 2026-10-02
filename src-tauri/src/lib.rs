pub mod backup;
pub mod codec;
pub mod display;
mod game_processes;
pub mod hud;
mod icons;
pub mod importer;
pub mod lunar;
pub mod minecraft;
pub mod model;
mod qr;
pub mod safety;
pub mod scanner;
mod updates;

use backup::{BackupManifest, BackupStore};
use codec::{EncodingResult, ShareEnvelope, ShareMetadata};
use importer::{HudLayoutContext, HudLayoutMode, HudLayoutRequest, ImportPreview, ImportTarget};
use model::{ScanReport, ScanRequest, Setting};
use serde::Serialize;
use std::path::PathBuf;
use tauri::{Manager, State};

fn hud_window(size: display::PixelSize, scale_factor: f64) -> Option<hud::HudWindow> {
    if !scale_factor.is_finite() || !(0.5..=8.0).contains(&scale_factor) {
        return None;
    }
    Some(hud::HudWindow {
        physical_width: size.width,
        physical_height: size.height,
        logical_height: if cfg!(target_os = "macos") {
            (f64::from(size.height) / scale_factor).round() as u32
        } else {
            size.height
        },
        macos: cfg!(target_os = "macos"),
    })
}

fn hud_layout_context(
    window: &tauri::WebviewWindow,
    layout: &HudLayoutRequest,
) -> HudLayoutContext {
    let resolved = match layout.mode {
        HudLayoutMode::Preserve => return HudLayoutContext::default(),
        HudLayoutMode::Auto => display::measure_maximized(window)
            .ok()
            .map(|display| (display.client_physical, display.scale_factor)),
        HudLayoutMode::Manual => layout.window_size.zip(window.scale_factor().ok()),
    };
    match resolved {
        Some((size, scale)) => HudLayoutContext {
            window_size: Some(size),
            window: hud_window(size, scale),
        },
        None => HudLayoutContext::default(),
    }
}

#[derive(Clone)]
struct AppState {
    store: BackupStore,
}

fn diagnostic(store: &BackupStore, event: &str, successful: bool) {
    use std::io::Write;
    let path = store
        .root
        .parent()
        .unwrap_or(&store.root)
        .join("diagnostics.log");
    let large = std::fs::metadata(&path)
        .map(|m| m.len() > 262144)
        .unwrap_or(false);
    if let Ok(mut file) = std::fs::OpenOptions::new()
        .create(true)
        .append(!large)
        .write(true)
        .truncate(large)
        .open(path)
    {
        let _ = writeln!(
            file,
            "{} {} {}",
            chrono::Utc::now().to_rfc3339(),
            event,
            if successful { "ok" } else { "failed" }
        );
    }
}

async fn blocking<T: Send + 'static>(
    operation: impl FnOnce() -> Result<T, String> + Send + 'static,
) -> Result<T, String> {
    tauri::async_runtime::spawn_blocking(operation)
        .await
        .map_err(|_| "処理を完了できませんでした。再試行してください。".to_string())?
}

#[tauri::command]
async fn scan_configuration(
    request: ScanRequest,
    state: State<'_, AppState>,
) -> Result<ScanReport, String> {
    let store = state.store.clone();
    blocking(move || {
        let _lock = store.lock()?;
        let recovered = store.recover()?;
        let mut report = scanner::scan(request)?;
        if recovered {
            report
                .warnings
                .push("中断した処理をバックアップから自動復旧しました。".into());
        }
        diagnostic(&store, "scan", true);
        Ok(report)
    })
    .await
}

#[tauri::command]
async fn encode_share(
    settings: Vec<Setting>,
    mut metadata: ShareMetadata,
    request: Option<ScanRequest>,
    window: tauri::WebviewWindow,
) -> Result<EncodingResult, String> {
    blocking(move || {
        metadata.hud_viewport = None;
        let profiles: std::collections::BTreeSet<_> = settings
            .iter()
            .filter(|setting| hud::is_adaptable_coordinate(setting))
            .map(|setting| setting.profile.as_str())
            .collect();
        if profiles.len() > 1 {
            return Err("Select one Lunar profile before sharing HUD positions.".into());
        }
        if let Some(setting) = settings
            .iter()
            .find(|setting| hud::is_adaptable_coordinate(setting))
        {
            let report = scanner::scan(request.unwrap_or_default())?;
            let context = hud_layout_context(&window, &HudLayoutRequest::default());
            metadata.hud_viewport = context
                .window
                .as_ref()
                .and_then(|window| hud::viewport(&report, &setting.profile, window, &[]).ok());
        }
        codec::encode(settings, metadata)
    })
    .await
}

#[tauri::command]
async fn decode_share(code: String) -> Result<ShareEnvelope, String> {
    blocking(move || codec::decode(&code)).await
}

#[tauri::command]
async fn preview_import(
    code: String,
    selected_ids: Vec<String>,
    target: ImportTarget,
    request: ScanRequest,
    hud_layout: Option<HudLayoutRequest>,
    window: tauri::WebviewWindow,
) -> Result<ImportPreview, String> {
    blocking(move || {
        let layout = hud_layout.unwrap_or_default();
        layout.validate()?;
        let context = hud_layout_context(&window, &layout);
        importer::preview_with_layout(&code, &selected_ids, &target, &request, &layout, &context)
    })
    .await
}

#[tauri::command]
#[allow(clippy::too_many_arguments)]
async fn apply_import(
    code: String,
    selected_ids: Vec<String>,
    target: ImportTarget,
    request: ScanRequest,
    allow_running: bool,
    preview_fingerprint: String,
    hud_layout: Option<HudLayoutRequest>,
    window: tauri::WebviewWindow,
    state: State<'_, AppState>,
) -> Result<BackupManifest, String> {
    let store = state.store.clone();
    blocking(move || {
        let _lock = store.lock()?;
        store.recover()?;
        let layout = hud_layout.unwrap_or_default();
        layout.validate()?;
        let context = hud_layout_context(&window, &layout);
        let result = importer::apply_with_layout(
            &store,
            &code,
            &selected_ids,
            &target,
            &request,
            allow_running,
            &preview_fingerprint,
            &layout,
            &context,
        );
        diagnostic(&store, "import", result.is_ok());
        result
    })
    .await
}

#[tauri::command]
async fn list_backups(state: State<'_, AppState>) -> Result<Vec<BackupManifest>, String> {
    let store = state.store.clone();
    blocking(move || {
        let _lock = store.lock()?;
        store.list()
    })
    .await
}

#[tauri::command]
async fn create_backup(
    request: ScanRequest,
    state: State<'_, AppState>,
) -> Result<BackupManifest, String> {
    let store = state.store.clone();
    blocking(move || {
        let _lock = store.lock()?;
        store.recover()?;
        let report = scanner::scan(request.clone())?;
        let paths = report
            .files
            .iter()
            .map(|f| PathBuf::from(&f.path))
            .collect::<Vec<_>>();
        let result = store.snapshot(&report, &request, "manual", &paths);
        diagnostic(&store, "backup", result.is_ok());
        result
    })
    .await
}

#[tauri::command]
async fn restore_backup(
    id: String,
    allow_running: bool,
    state: State<'_, AppState>,
) -> Result<BackupManifest, String> {
    let store = state.store.clone();
    blocking(move || {
        let _lock = store.lock()?;
        if let Err(error) = store.recover() {
            if store.pending_backup_id()? != Some(id.clone()) {
                return Err(error);
            }
        }
        let manifest = store.load(&id)?;
        let report = scanner::scan(manifest.request.clone())?;
        if !allow_running && !report.running_processes.is_empty() {
            return Err(
                "MinecraftまたはLunar Clientが起動中です。終了してから復元してください。".into(),
            );
        }
        let updates = store.verify(&manifest, &report)?;
        let result = store.apply(&report, &manifest.request, "restore", updates);
        diagnostic(&store, "restore", result.is_ok());
        result
    })
    .await
}

#[tauri::command]
async fn delete_backup(id: String, state: State<'_, AppState>) -> Result<(), String> {
    let store = state.store.clone();
    blocking(move || {
        let _lock = store.lock()?;
        let result = store.delete(&id);
        diagnostic(&store, "delete-backup", result.is_ok());
        result
    })
    .await
}

fn open_target(target: &std::ffi::OsStr) -> Result<(), String> {
    #[cfg(target_os = "macos")]
    let result = std::process::Command::new("open").arg(target).spawn();
    #[cfg(target_os = "windows")]
    let result = std::process::Command::new("explorer").arg(target).spawn();
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    let result = std::process::Command::new("xdg-open").arg(target).spawn();
    result
        .map(|_| ())
        .map_err(|_| "場所を開けませんでした。".into())
}

#[tauri::command]
fn open_backup_folder(state: State<'_, AppState>) -> Result<(), String> {
    let _lock = state.store.lock()?;
    open_target(state.store.root.as_os_str())
}

#[tauri::command]
fn open_project_page(page: String) -> Result<(), String> {
    let url = match page.as_str() {
        "repository" => "https://github.com/Shake1227/PrismRelay",
        "releases" => "https://github.com/Shake1227/PrismRelay/releases",
        "license" => "https://github.com/Shake1227/PrismRelay/blob/main/LICENSE",
        "x" => "https://x.com/shake_1227",
        _ => return Err("このリンクは開けません。".into()),
    };
    open_target(std::ffi::OsStr::new(url))
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct AppInfo {
    name: &'static str,
    version: &'static str,
    platform: &'static str,
    repository: &'static str,
    license: &'static str,
}

#[tauri::command]
fn get_app_info() -> AppInfo {
    AppInfo {
        name: "Prism Relay",
        version: env!("CARGO_PKG_VERSION"),
        platform: std::env::consts::OS,
        repository: "https://github.com/Shake1227/PrismRelay",
        license: "GPL-3.0-or-later",
    }
}

#[tauri::command]
async fn check_updates() -> Result<updates::UpdateInfo, String> {
    blocking(updates::check).await
}

#[tauri::command]
async fn load_qr_image(path: String) -> Result<String, String> {
    blocking(move || qr::load_image(&PathBuf::from(path))).await
}

#[tauri::command]
async fn save_qr_image(path: String, data_url: String) -> Result<(), String> {
    blocking(move || qr::save_image(&PathBuf::from(path), &data_url)).await
}

#[tauri::command]
fn save_share_file(path: String, code: String) -> Result<(), String> {
    codec::decode(&code)?;
    let path = PathBuf::from(path);
    if path.extension().and_then(|s| s.to_str()) != Some("prism") {
        return Err(".prismファイルとして保存してください。".into());
    }
    if path.exists()
        && std::fs::symlink_metadata(&path)
            .map_err(|e| e.to_string())?
            .file_type()
            .is_symlink()
    {
        return Err("リンク先には保存できません。".into());
    }
    backup::atomic_write(&path, code.as_bytes())
}

#[tauri::command]
fn load_share_file(path: String) -> Result<String, String> {
    let bytes = backup::read_config(&PathBuf::from(path))?;
    if bytes.len() > 2 * 1024 * 1024 {
        return Err("共有コードファイルが大きすぎます。".into());
    }
    let code =
        String::from_utf8(bytes).map_err(|_| "共有コードファイルを読み取れません。".to_string())?;
    codec::decode(&code)?;
    Ok(code)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .on_permission_request(|_, _| tauri::webview::PermissionResponse::Default)
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            let root = app.path().app_data_dir()?.join("backups");
            app.manage(AppState {
                store: BackupStore::new(root),
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            scan_configuration,
            encode_share,
            decode_share,
            preview_import,
            apply_import,
            list_backups,
            create_backup,
            restore_backup,
            delete_backup,
            open_backup_folder,
            open_project_page,
            get_app_info,
            check_updates,
            save_share_file,
            load_share_file,
            load_qr_image,
            save_qr_image
        ])
        .run(tauri::generate_context!())
        .expect("Prism Relay could not start");
}
