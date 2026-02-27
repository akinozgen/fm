use base64::Engine;
use file_icon_provider::get_file_icon;
use image::codecs::png::PngEncoder;
use image::{ColorType, ImageEncoder};

pub fn get_file_icon_png_base64(path: &str, size: u16) -> Result<String, String> {
  let size = size.max(16).min(128);
  let icon = get_file_icon(path, size).map_err(|e| format!("icon error: {e:?}"))?;

  let mut png_bytes: Vec<u8> = Vec::new();
  let encoder = PngEncoder::new(&mut png_bytes);
  encoder
    .write_image(&icon.pixels, icon.width, icon.height, ColorType::Rgba8.into())
    .map_err(|e| format!("png error: {e:?}"))?;

  let b64 = base64::engine::general_purpose::STANDARD.encode(png_bytes);
  Ok(b64)
}
