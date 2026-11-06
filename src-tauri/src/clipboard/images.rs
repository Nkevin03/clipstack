//! Image persistence: original bytes + a ≤400 px PNG thumbnail, both on disk.

use super::pasteboard::ImageFormat;
use std::path::Path;

pub struct SavedImage {
    pub path: String,
    pub thumb: Option<String>,
    pub width: i64,
    pub height: i64,
}

pub const THUMB_MAX: u32 = 400;

/// Save `bytes` as `<images>/<hash>.<ext>` and a thumbnail `<thumbs>/<hash>.png`.
/// Returns `None` if the data cannot be decoded as an image.
pub fn save(
    bytes: &[u8],
    fmt: ImageFormat,
    hash: &str,
    images: &Path,
    thumbs: &Path,
) -> Option<SavedImage> {
    let img = image::load_from_memory(bytes).ok()?;
    let (w, h) = (img.width(), img.height());
    let path = images.join(format!("{hash}.{}", fmt.ext()));
    if !path.exists() {
        std::fs::write(&path, bytes).ok()?;
    }
    let thumb_path = thumbs.join(format!("{hash}.png"));
    let thumb = if w <= THUMB_MAX && h <= THUMB_MAX && fmt == ImageFormat::Png {
        None // the original is already small enough to display
    } else if thumb_path.exists()
        || img
            .thumbnail(THUMB_MAX, THUMB_MAX)
            .save(&thumb_path)
            .is_ok()
    {
        Some(thumb_path.to_string_lossy().into_owned())
    } else {
        None
    };
    Some(SavedImage {
        path: path.to_string_lossy().into_owned(),
        thumb,
        width: w as i64,
        height: h as i64,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn saves_original_and_thumbnail() {
        let dir = tempfile::tempdir().unwrap();
        let thumbs = tempfile::tempdir().unwrap();
        let img = image::RgbaImage::from_pixel(1200, 600, image::Rgba([10, 200, 30, 255]));
        let mut buf = std::io::Cursor::new(Vec::new());
        img.write_to(&mut buf, image::ImageFormat::Png).unwrap();
        let s = save(
            buf.get_ref(),
            ImageFormat::Png,
            "abc",
            dir.path(),
            thumbs.path(),
        )
        .unwrap();
        assert_eq!((s.width, s.height), (1200, 600));
        let t = image::open(s.thumb.unwrap()).unwrap();
        assert_eq!((t.width(), t.height()), (400, 200));
        assert!(
            save(
                b"not an image",
                ImageFormat::Png,
                "x",
                dir.path(),
                dir.path()
            )
            .is_none()
        );
    }
}
