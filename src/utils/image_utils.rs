use crate::capture_providers::shared::PixelFormat;

#[derive(Debug, thiserror::Error)]
pub enum ImageError {
    #[error("Pixel format not supported")]
    PixelFormatNotSupported,
}

pub fn ensure_image_rgba(
    bytes: &mut [u8],
    image_format: &mut PixelFormat,
) -> Result<(), ImageError> {
    match image_format {
        PixelFormat::RGBA8 => (),
        PixelFormat::BGRA8 => bgra_to_rgba(bytes),
        _ => return Err(ImageError::PixelFormatNotSupported),
    };
    *image_format = PixelFormat::RGBA8;
    Ok(())
}

pub fn bgra_to_rgba(bytes: &mut [u8]) {
    for pixel in bytes.chunks_exact_mut(4) {
        pixel.swap(0, 2); // swap B and R
    }
}
