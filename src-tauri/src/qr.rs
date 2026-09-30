use base64::{engine::general_purpose::STANDARD, Engine};
use image::{ImageFormat, ImageReader, Limits};
use std::{io::Cursor, path::Path};

pub fn load_image(path: &Path) -> Result<String, String> {
    let bytes = crate::backup::read_config(path)?;
    if bytes.len() > 8 * 1024 * 1024 {
        return Err("8 MB以下のQR画像を選択してください。".into());
    }
    image_data(&bytes)
}

pub fn save_image(path: &Path, data_url: &str) -> Result<(), String> {
    if !path
        .extension()
        .and_then(|s| s.to_str())
        .is_some_and(|s| s.eq_ignore_ascii_case("png"))
    {
        return Err("PNG画像として保存してください。".into());
    }
    let encoded = data_url
        .strip_prefix("data:image/png;base64,")
        .filter(|s| s.len() <= 8 * 1024 * 1024)
        .ok_or_else(|| "QR画像を保存できませんでした。".to_string())?;
    let bytes = STANDARD
        .decode(encoded)
        .map_err(|_| "QR画像を保存できませんでした。".to_string())?;
    image_data(&bytes)?;
    crate::backup::atomic_write(path, &bytes)
}

fn image_data(bytes: &[u8]) -> Result<String, String> {
    let mut reader = ImageReader::new(Cursor::new(bytes))
        .with_guessed_format()
        .map_err(|_| {
            "画像を読み込めませんでした。PNG、JPEG、WebP画像を選択してください。".to_string()
        })?;
    if !matches!(
        reader.format(),
        Some(ImageFormat::Png | ImageFormat::Jpeg | ImageFormat::WebP)
    ) {
        return Err("PNG、JPEG、WebP画像を選択してください。".into());
    }
    let mut limits = Limits::default();
    limits.max_image_width = Some(4096);
    limits.max_image_height = Some(4096);
    limits.max_alloc = Some(64 * 1024 * 1024);
    reader.limits(limits);
    let image = reader.decode().map_err(|_| {
        "画像を読み込めませんでした。4096ピクセル以下の画像を選択してください。".to_string()
    })?;
    let mut output = Cursor::new(Vec::new());
    image
        .write_to(&mut output, ImageFormat::Png)
        .map_err(|_| "画像を読み込めませんでした。".to_string())?;
    if output.get_ref().len() > 16 * 1024 * 1024 {
        return Err("QRコード部分を切り取った画像を選択してください。".into());
    }
    Ok(format!(
        "data:image/png;base64,{}",
        STANDARD.encode(output.into_inner())
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn qr_image_is_normalized_and_non_images_are_rejected() {
        let image = image::RgbImage::from_pixel(8, 8, image::Rgb([255, 255, 255]));
        let mut input = Cursor::new(Vec::new());
        image.write_to(&mut input, ImageFormat::Png).unwrap();
        let uri = image_data(input.get_ref()).unwrap();
        assert!(uri.starts_with("data:image/png;base64,"));
        let decoded = STANDARD.decode(uri.split_once(',').unwrap().1).unwrap();
        assert_eq!(image::load_from_memory(&decoded).unwrap().width(), 8);
        assert!(image_data(b"not an image").is_err());
        assert!(image_data(&input.get_ref()[..20]).is_err());
    }

    #[test]
    fn images_over_the_dimension_limit_are_rejected() {
        let image = image::RgbImage::new(4097, 1);
        let mut input = Cursor::new(Vec::new());
        image.write_to(&mut input, ImageFormat::Png).unwrap();
        assert!(image_data(input.get_ref()).is_err());
    }

    #[test]
    fn saved_qr_images_can_be_read_again() {
        let directory = tempfile::tempdir().unwrap();
        let directory_path = dunce::canonicalize(directory.path()).unwrap();
        let path = directory_path.join("share.png");
        let image = image::RgbImage::from_pixel(8, 8, image::Rgb([255, 255, 255]));
        let mut input = Cursor::new(Vec::new());
        image.write_to(&mut input, ImageFormat::Png).unwrap();
        let uri = image_data(input.get_ref()).unwrap();
        save_image(&path, &uri).unwrap();
        assert_eq!(load_image(&path).unwrap(), uri);
        assert!(save_image(&path, "data:image/png;base64,invalid").is_err());
        assert!(save_image(&directory_path.join("share.exe"), &uri).is_err());
        assert_eq!(load_image(&path).unwrap(), uri);
    }
}
