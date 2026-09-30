use crate::model::ApplicationIcons;
use base64::{engine::general_purpose::STANDARD, Engine};
use std::collections::BTreeMap;
use std::fs::{self, File};
use std::io::{Cursor, Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};
use std::time::SystemTime;

const MAX_ICON_BYTES: usize = 1024 * 1024;
const MAX_CONTAINER_BYTES: u64 = 8 * 1024 * 1024;
const MAX_EXECUTABLE_BYTES: u64 = 256 * 1024 * 1024;
const MAX_PIXEL_BYTES: usize = 8 * 1024 * 1024;
const MAX_DIMENSION: u32 = 1024;

struct ValidIcon {
    png: Vec<u8>,
    width: u32,
    height: u32,
}

struct CachedIcon {
    modified: Option<SystemTime>,
    size: u64,
    data_uri: Option<String>,
}

pub fn detect_application_icons(platform: &str, home: &Path) -> Option<ApplicationIcons> {
    let (minecraft, lunar) = candidate_paths(platform, home);
    let icons = ApplicationIcons {
        minecraft: first_icon(&minecraft),
        lunar: first_icon(&lunar),
    };
    if icons.minecraft.is_some() || icons.lunar.is_some() {
        Some(icons)
    } else {
        None
    }
}

fn candidate_paths(platform: &str, home: &Path) -> (Vec<PathBuf>, Vec<PathBuf>) {
    if platform == "macOS" {
        let roots = [PathBuf::from("/Applications"), home.join("Applications")];
        let minecraft = roots
            .iter()
            .flat_map(|root| {
                ["Minecraft.app", "Minecraft Launcher.app"]
                    .into_iter()
                    .flat_map(move |name| mac_bundle_candidates(&root.join(name)))
            })
            .collect();
        let lunar = roots
            .iter()
            .flat_map(|root| mac_bundle_candidates(&root.join("Lunar Client.app")))
            .collect();
        return (minecraft, lunar);
    }
    if platform == "Windows" {
        let local = std::env::var_os("LOCALAPPDATA")
            .map(PathBuf::from)
            .unwrap_or_else(|| home.join("AppData/Local"));
        let program_roots: Vec<PathBuf> = ["ProgramFiles", "ProgramFiles(x86)"]
            .into_iter()
            .filter_map(std::env::var_os)
            .map(PathBuf::from)
            .collect();
        let mut minecraft = windows_launcher_candidates(
            &local.join("Programs/Minecraft Launcher"),
            "MinecraftLauncher.exe",
        );
        let mut lunar =
            windows_launcher_candidates(&local.join("Programs/lunarclient"), "Lunar Client.exe");
        lunar.extend(windows_launcher_candidates(
            &local.join("Programs/Lunar Client"),
            "Lunar Client.exe",
        ));
        for root in &program_roots {
            minecraft.extend(windows_launcher_candidates(
                &root.join("Minecraft Launcher"),
                "MinecraftLauncher.exe",
            ));
            lunar.extend(windows_launcher_candidates(
                &root.join("Lunar Client"),
                "Lunar Client.exe",
            ));
            minecraft.extend(store_package_candidates(&root.join("WindowsApps")));
        }
        if let Some(drive) = std::env::var_os("SystemDrive")
            .and_then(|drive| drive.into_string().ok())
            .filter(|drive| {
                drive.len() == 2
                    && drive.as_bytes()[0].is_ascii_alphabetic()
                    && drive.as_bytes()[1] == b':'
            })
        {
            let content =
                PathBuf::from(format!("{drive}\\")).join("XboxGames/Minecraft Launcher/Content");
            minecraft.extend(store_asset_candidates(&content));
            minecraft.push(content.join("Minecraft.exe"));
        }
        return (minecraft, lunar);
    }
    let mut minecraft = Vec::new();
    let mut lunar = Vec::new();
    for root in [
        PathBuf::from("/usr/share/icons"),
        home.join(".local/share/icons"),
    ] {
        for size in ["128x128", "256x256", "512x512"] {
            minecraft.push(
                root.join("hicolor")
                    .join(size)
                    .join("apps/minecraft-launcher.png"),
            );
            lunar.push(root.join("hicolor").join(size).join("apps/lunarclient.png"));
        }
    }
    (minecraft, lunar)
}

fn mac_bundle_candidates(bundle: &Path) -> Vec<PathBuf> {
    let resources = bundle.join("Contents/Resources");
    [
        "favicon.icns",
        "icon.icns",
        "AppIcon.icns",
        "minecraft.icns",
        "icon.png",
        "favicon.png",
        "minecraft.png",
        "lunarclient.png",
    ]
    .into_iter()
    .map(|name| resources.join(name))
    .collect()
}

fn windows_launcher_candidates(directory: &Path, executable: &str) -> Vec<PathBuf> {
    [
        "icon.png",
        "icon.ico",
        "resources/icon.png",
        "resources/app/icon.png",
        "resources/app.asar.unpacked/icon.png",
        executable,
    ]
    .into_iter()
    .map(|name| directory.join(name))
    .collect()
}

fn store_package_candidates(directory: &Path) -> Vec<PathBuf> {
    let Ok(entries) = fs::read_dir(directory) else {
        return Vec::new();
    };
    let mut packages = Vec::new();
    for entry in entries.flatten().take(512) {
        let name = entry.file_name();
        let Some(name) = name.to_str() else { continue };
        if name.starts_with("Microsoft.4297127D64EC6_")
            && name.ends_with("_8wekyb3d8bbwe")
            && name.len() <= 160
            && entry
                .file_type()
                .is_ok_and(|kind| kind.is_dir() && !kind.is_symlink())
        {
            packages.push(entry.path());
        }
    }
    packages.sort();
    packages.reverse();
    packages
        .into_iter()
        .take(4)
        .flat_map(|package| {
            let mut candidates = store_asset_candidates(&package);
            candidates.push(package.join("Minecraft.exe"));
            candidates
        })
        .collect()
}

fn store_asset_candidates(package: &Path) -> Vec<PathBuf> {
    let assets = package.join("Assets");
    let mut candidates: Vec<PathBuf> = [
        "Square150x150Logo.png",
        "Square44x44Logo.png",
        "Logo.png",
        "AppIcon.png",
        "MinecraftLauncher.png",
    ]
    .into_iter()
    .map(|name| assets.join(name))
    .collect();
    if let Ok(entries) = fs::read_dir(&assets) {
        for entry in entries.flatten().take(128) {
            let name = entry.file_name();
            let Some(name) = name.to_str() else { continue };
            if name.len() <= 128
                && name.ends_with(".png")
                && [
                    "Square44x44Logo.targetsize-",
                    "Square150x150Logo.scale-",
                    "Square44x44Logo.scale-",
                ]
                .iter()
                .any(|prefix| name.starts_with(prefix))
                && name
                    .bytes()
                    .all(|byte| byte.is_ascii_alphanumeric() || b"-_.".contains(&byte))
            {
                candidates.insert(0, entry.path());
            }
        }
    }
    candidates
}

fn first_icon(candidates: &[PathBuf]) -> Option<String> {
    static CACHE: OnceLock<Mutex<BTreeMap<PathBuf, CachedIcon>>> = OnceLock::new();
    let cache = CACHE.get_or_init(|| Mutex::new(BTreeMap::new()));
    for path in candidates {
        let Ok(metadata) = fs::symlink_metadata(path) else {
            continue;
        };
        if !metadata.is_file() || metadata.file_type().is_symlink() {
            continue;
        }
        let modified = metadata.modified().ok();
        if let Ok(cache) = cache.lock() {
            if let Some(cached) = cache
                .get(path)
                .filter(|cached| cached.modified == modified && cached.size == metadata.len())
            {
                if cached.data_uri.is_some() {
                    return cached.data_uri.clone();
                }
                continue;
            }
        }
        let data_uri = read_icon(path)
            .map(|icon| format!("data:image/png;base64,{}", STANDARD.encode(icon.png)));
        if let Ok(mut cache) = cache.lock() {
            if cache.len() >= 128 {
                cache.clear();
            }
            cache.insert(
                path.clone(),
                CachedIcon {
                    modified,
                    size: metadata.len(),
                    data_uri: data_uri.clone(),
                },
            );
        }
        if data_uri.is_some() {
            return data_uri;
        }
    }
    None
}

fn open_icon_file(path: &Path, limit: u64) -> Option<File> {
    let metadata = fs::symlink_metadata(path).ok()?;
    if !metadata.is_file() || metadata.file_type().is_symlink() || metadata.len() > limit {
        return None;
    }
    let canonical = dunce::canonicalize(path).ok()?;
    if canonical != path {
        return None;
    }
    for ancestor in path.ancestors() {
        if fs::symlink_metadata(ancestor)
            .ok()?
            .file_type()
            .is_symlink()
        {
            return None;
        }
    }
    File::open(path).ok()
}

fn read_icon(path: &Path) -> Option<ValidIcon> {
    let extension = path.extension()?.to_str()?.to_ascii_lowercase();
    if extension == "exe" {
        let mut file = open_icon_file(path, MAX_EXECUTABLE_BYTES)?;
        return pe_icon(&mut file);
    }
    let mut file = open_icon_file(path, MAX_CONTAINER_BYTES)?;
    let mut bytes = Vec::new();
    file.by_ref()
        .take(MAX_CONTAINER_BYTES + 1)
        .read_to_end(&mut bytes)
        .ok()?;
    if bytes.len() as u64 > MAX_CONTAINER_BYTES {
        return None;
    }
    match extension.as_str() {
        "png" => validate_png(&bytes),
        "icns" => icns_icon(&bytes),
        "ico" => ico_icon(&bytes),
        _ => None,
    }
}

fn validate_png(bytes: &[u8]) -> Option<ValidIcon> {
    if bytes.len() > MAX_ICON_BYTES || !bytes.starts_with(b"\x89PNG\r\n\x1a\n") {
        return None;
    }
    let mut decoder = png::Decoder::new_with_limits(
        Cursor::new(bytes),
        png::Limits {
            bytes: MAX_PIXEL_BYTES,
        },
    );
    decoder.set_transformations(png::Transformations::EXPAND | png::Transformations::STRIP_16);
    let mut reader = decoder.read_info().ok()?;
    let info = reader.info();
    if info.width < 8
        || info.height < 8
        || info.width > MAX_DIMENSION
        || info.height > MAX_DIMENSION
        || info.animation_control.is_some()
    {
        return None;
    }
    let output_size = reader.output_buffer_size()?;
    if output_size > MAX_PIXEL_BYTES {
        return None;
    }
    let mut pixels = vec![0; output_size];
    let output = reader.next_frame(&mut pixels).ok()?;
    reader.finish().ok()?;
    let mut encoded = Vec::new();
    {
        let mut encoder = png::Encoder::new(&mut encoded, output.width, output.height);
        encoder.set_color(output.color_type);
        encoder.set_depth(output.bit_depth);
        encoder
            .write_header()
            .ok()?
            .write_image_data(&pixels[..output.buffer_size()])
            .ok()?;
    }
    if encoded.len() > MAX_ICON_BYTES {
        return None;
    }
    Some(ValidIcon {
        png: encoded,
        width: output.width,
        height: output.height,
    })
}

fn better_icon(best: Option<ValidIcon>, incoming: ValidIcon) -> Option<ValidIcon> {
    let rank = |icon: &ValidIcon| icon.width.abs_diff(256) + icon.height.abs_diff(256);
    if best
        .as_ref()
        .is_none_or(|best| rank(&incoming) < rank(best))
    {
        Some(incoming)
    } else {
        best
    }
}

fn icns_icon(bytes: &[u8]) -> Option<ValidIcon> {
    if bytes.len() < 8 || &bytes[..4] != b"icns" || read_be_u32(bytes, 4)? as usize != bytes.len() {
        return None;
    }
    let mut offset = 8;
    let mut best = None;
    while offset < bytes.len() {
        let length = read_be_u32(bytes, offset + 4)? as usize;
        if length < 8 {
            return None;
        }
        let end = offset.checked_add(length)?;
        let chunk = bytes.get(offset + 8..end)?;
        if let Some(icon) = validate_png(chunk) {
            best = better_icon(best, icon);
        }
        offset = end;
    }
    best
}

fn ico_icon(bytes: &[u8]) -> Option<ValidIcon> {
    if bytes.get(..4)? != [0, 0, 1, 0] {
        return None;
    }
    let count = read_le_u16(bytes, 4)? as usize;
    if count == 0 || count > 64 || 6 + count * 16 > bytes.len() {
        return None;
    }
    let mut best = None;
    for index in 0..count {
        let entry = 6 + index * 16;
        let length = read_le_u32(bytes, entry + 8)? as usize;
        let offset = read_le_u32(bytes, entry + 12)? as usize;
        if length > MAX_ICON_BYTES {
            continue;
        }
        let data = bytes.get(offset..offset.checked_add(length)?)?;
        if let Some(icon) = validate_png(data) {
            best = better_icon(best, icon);
        }
    }
    best
}

struct PeSection {
    address: u32,
    raw_offset: u32,
    raw_size: u32,
}

fn pe_icon(file: &mut File) -> Option<ValidIcon> {
    let size = file.metadata().ok()?.len();
    let dos = read_at(file, 0, 64, size)?;
    if &dos[..2] != b"MZ" {
        return None;
    }
    let pe_offset = read_le_u32(&dos, 60)? as u64;
    if pe_offset > 1024 * 1024 {
        return None;
    }
    let header = read_at(file, pe_offset, 24, size)?;
    if &header[..4] != b"PE\0\0" {
        return None;
    }
    let section_count = read_le_u16(&header, 6)? as usize;
    let optional_length = read_le_u16(&header, 20)? as usize;
    if section_count == 0 || section_count > 96 || optional_length > 4096 {
        return None;
    }
    let optional = read_at(file, pe_offset + 24, optional_length, size)?;
    let directories = match read_le_u16(&optional, 0)? {
        0x10b => 96,
        0x20b => 112,
        _ => return None,
    };
    let resource_address = read_le_u32(&optional, directories + 16)?;
    let resource_size = read_le_u32(&optional, directories + 20)? as usize;
    if !(16..=4 * 1024 * 1024).contains(&resource_size) {
        return None;
    }
    let table = read_at(
        file,
        pe_offset + 24 + optional_length as u64,
        section_count * 40,
        size,
    )?;
    let mut sections = Vec::new();
    for index in 0..section_count {
        let offset = index * 40;
        sections.push(PeSection {
            address: read_le_u32(&table, offset + 12)?,
            raw_size: read_le_u32(&table, offset + 16)?,
            raw_offset: read_le_u32(&table, offset + 20)?,
        });
    }
    let resource_offset = rva_offset(&sections, resource_address, resource_size)?;
    let resources = read_at(file, resource_offset, resource_size, size)?;
    let mut payloads = Vec::new();
    for (name, target) in resource_entries(&resources, 0)? {
        if name == 3 && target & 0x80000000 != 0 {
            collect_icon_resources(&resources, (target & 0x7fffffff) as usize, 0, &mut payloads)?;
        }
    }
    let mut best = None;
    for (address, length) in payloads {
        let length = length as usize;
        if length > MAX_ICON_BYTES {
            continue;
        }
        let offset = rva_offset(&sections, address, length)?;
        let png = read_at(file, offset, length, size)?;
        if let Some(icon) = validate_png(&png) {
            best = better_icon(best, icon);
        }
    }
    best
}

fn resource_entries(bytes: &[u8], offset: usize) -> Option<Vec<(u32, u32)>> {
    let named = read_le_u16(bytes, offset.checked_add(12)?)? as usize;
    let numbered = read_le_u16(bytes, offset.checked_add(14)?)? as usize;
    let count = named.checked_add(numbered)?;
    if count > 256 {
        return None;
    }
    let mut entries = Vec::new();
    for index in 0..count {
        let entry = offset.checked_add(16 + index * 8)?;
        entries.push((read_le_u32(bytes, entry)?, read_le_u32(bytes, entry + 4)?));
    }
    Some(entries)
}

fn collect_icon_resources(
    bytes: &[u8],
    offset: usize,
    depth: usize,
    payloads: &mut Vec<(u32, u32)>,
) -> Option<()> {
    if depth > 3 || payloads.len() >= 64 {
        return None;
    }
    for (_, target) in resource_entries(bytes, offset)? {
        if target & 0x80000000 != 0 {
            collect_icon_resources(bytes, (target & 0x7fffffff) as usize, depth + 1, payloads)?;
        } else {
            let entry = target as usize;
            payloads.push((read_le_u32(bytes, entry)?, read_le_u32(bytes, entry + 4)?));
            if payloads.len() > 64 {
                return None;
            }
        }
    }
    Some(())
}

fn rva_offset(sections: &[PeSection], address: u32, length: usize) -> Option<u64> {
    for section in sections {
        let Some(offset) = address.checked_sub(section.address) else {
            continue;
        };
        if (offset as u64).checked_add(length as u64)? <= section.raw_size as u64 {
            return (section.raw_offset as u64).checked_add(offset as u64);
        }
    }
    None
}

fn read_at(file: &mut File, offset: u64, length: usize, total: u64) -> Option<Vec<u8>> {
    if length > 4 * 1024 * 1024 || offset.checked_add(length as u64)? > total {
        return None;
    }
    file.seek(SeekFrom::Start(offset)).ok()?;
    let mut bytes = vec![0; length];
    file.read_exact(&mut bytes).ok()?;
    Some(bytes)
}

fn read_le_u16(bytes: &[u8], offset: usize) -> Option<u16> {
    Some(u16::from_le_bytes(
        bytes.get(offset..offset.checked_add(2)?)?.try_into().ok()?,
    ))
}

fn read_le_u32(bytes: &[u8], offset: usize) -> Option<u32> {
    Some(u32::from_le_bytes(
        bytes.get(offset..offset.checked_add(4)?)?.try_into().ok()?,
    ))
}

fn read_be_u32(bytes: &[u8], offset: usize) -> Option<u32> {
    Some(u32::from_be_bytes(
        bytes.get(offset..offset.checked_add(4)?)?.try_into().ok()?,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn png_image(size: u32) -> Vec<u8> {
        let mut bytes = Vec::new();
        {
            let mut encoder = png::Encoder::new(&mut bytes, size, size);
            encoder.set_color(png::ColorType::Rgba);
            encoder.set_depth(png::BitDepth::Eight);
            encoder
                .write_header()
                .unwrap()
                .write_image_data(&vec![120; (size * size * 4) as usize])
                .unwrap();
        }
        bytes
    }

    #[test]
    fn valid_png_is_decoded_and_corrupt_or_oversized_images_are_rejected() {
        let image = png_image(32);
        let icon = validate_png(&image).unwrap();
        assert_eq!((icon.width, icon.height), (32, 32));
        assert!(validate_png(b"not an image").is_none());
        let mut damaged = image;
        damaged[29] ^= 1;
        assert!(validate_png(&damaged).is_none());
        assert!(validate_png(&png_image(2048)).is_none());
    }

    #[test]
    fn embedded_icns_and_ico_pngs_are_extracted_without_other_payloads() {
        let image = png_image(32);
        let mut icns = b"icns".to_vec();
        icns.extend_from_slice(&((16 + image.len()) as u32).to_be_bytes());
        icns.extend_from_slice(b"icp5");
        icns.extend_from_slice(&((8 + image.len()) as u32).to_be_bytes());
        icns.extend_from_slice(&image);
        assert!(icns_icon(&icns).is_some());
        let mut ico = vec![0, 0, 1, 0, 1, 0, 32, 32, 0, 0, 1, 0, 32, 0];
        ico.extend_from_slice(&(image.len() as u32).to_le_bytes());
        ico.extend_from_slice(&22u32.to_le_bytes());
        ico.extend_from_slice(&image);
        assert!(ico_icon(&ico).is_some());
        icns[7] ^= 1;
        assert!(icns_icon(&icns).is_none());
        assert!(ico_icon(&[0, 0, 1, 0, 255, 255]).is_none());
    }

    #[test]
    fn windows_pe_icons_use_only_the_bounded_rt_icon_resource() {
        let directory = tempfile::tempdir().unwrap();
        let root = dunce::canonicalize(directory.path()).unwrap();
        let image = png_image(32);
        let mut resources = vec![0; 128 + image.len()];
        resources[14..16].copy_from_slice(&1u16.to_le_bytes());
        resources[16..20].copy_from_slice(&3u32.to_le_bytes());
        resources[20..24].copy_from_slice(&0x80000020u32.to_le_bytes());
        resources[46..48].copy_from_slice(&1u16.to_le_bytes());
        resources[48..52].copy_from_slice(&1u32.to_le_bytes());
        resources[52..56].copy_from_slice(&0x80000040u32.to_le_bytes());
        resources[78..80].copy_from_slice(&1u16.to_le_bytes());
        resources[80..84].copy_from_slice(&1033u32.to_le_bytes());
        resources[84..88].copy_from_slice(&96u32.to_le_bytes());
        resources[96..100].copy_from_slice(&0x3080u32.to_le_bytes());
        resources[100..104].copy_from_slice(&(image.len() as u32).to_le_bytes());
        resources[128..].copy_from_slice(&image);
        for machine in [0x8664u16, 0xaa64u16] {
            let mut executable = vec![0; 0x400 + resources.len()];
            executable[..2].copy_from_slice(b"MZ");
            executable[60..64].copy_from_slice(&0x80u32.to_le_bytes());
            executable[0x80..0x84].copy_from_slice(b"PE\0\0");
            executable[0x84..0x86].copy_from_slice(&machine.to_le_bytes());
            executable[0x86..0x88].copy_from_slice(&2u16.to_le_bytes());
            executable[0x94..0x96].copy_from_slice(&240u16.to_le_bytes());
            let optional = 0x98;
            executable[optional..optional + 2].copy_from_slice(&0x20bu16.to_le_bytes());
            executable[optional + 128..optional + 132].copy_from_slice(&0x3000u32.to_le_bytes());
            executable[optional + 132..optional + 136]
                .copy_from_slice(&(resources.len() as u32).to_le_bytes());
            let table = optional + 240;
            executable[table + 12..table + 16].copy_from_slice(&0x1000u32.to_le_bytes());
            executable[table + 16..table + 20].copy_from_slice(&128u32.to_le_bytes());
            executable[table + 20..table + 24].copy_from_slice(&0x200u32.to_le_bytes());
            executable[table + 52..table + 56].copy_from_slice(&0x3000u32.to_le_bytes());
            executable[table + 56..table + 60]
                .copy_from_slice(&(resources.len() as u32).to_le_bytes());
            executable[table + 60..table + 64].copy_from_slice(&0x400u32.to_le_bytes());
            executable[0x400..].copy_from_slice(&resources);
            let path = root.join(format!("launcher-{machine}.exe"));
            fs::write(&path, &executable).unwrap();
            assert!(read_icon(&path).is_some());
            executable[0x400 + 52..0x400 + 56].copy_from_slice(&0x80000020u32.to_le_bytes());
            fs::write(&path, &executable).unwrap();
            assert!(read_icon(&path).is_none());
        }
    }

    #[test]
    fn missing_corrupt_and_linked_icons_leave_the_fallback_available() {
        let directory = tempfile::tempdir().unwrap();
        let root = dunce::canonicalize(directory.path()).unwrap();
        assert!(first_icon(&[root.join("missing.png")]).is_none());
        fs::write(root.join("broken.png"), b"invalid").unwrap();
        assert!(first_icon(&[root.join("broken.png")]).is_none());
        fs::write(root.join("icon.png"), png_image(32)).unwrap();
        assert!(first_icon(&[root.join("icon.png")])
            .unwrap()
            .starts_with("data:image/png;base64,"));
        #[cfg(unix)]
        {
            std::os::unix::fs::symlink(root.join("icon.png"), root.join("linked.png")).unwrap();
            assert!(first_icon(&[root.join("linked.png")]).is_none());
        }
    }

    #[test]
    fn app_candidates_exclude_game_configuration_and_account_folders() {
        let (minecraft, lunar) = candidate_paths("macOS", Path::new("/Users/Example"));
        assert!(minecraft
            .iter()
            .any(|path| path.ends_with("Minecraft.app/Contents/Resources/favicon.icns")));
        assert!(lunar
            .iter()
            .any(|path| path.ends_with("Lunar Client.app/Contents/Resources/icon.icns")));
        assert!(minecraft
            .iter()
            .chain(&lunar)
            .all(|path| !path.to_string_lossy().contains(".minecraft")
                && !path.to_string_lossy().contains(".lunarclient")));
    }
}
