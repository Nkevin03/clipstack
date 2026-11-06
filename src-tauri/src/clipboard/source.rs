//! Source application detection (`NSWorkspace.frontmostApplication`) and icon colour.

use objc2::AllocAnyThread;
use objc2::rc::autoreleasepool;
use objc2_app_kit::{NSBitmapImageFileType, NSBitmapImageRep, NSWorkspace};
use objc2_foundation::{NSDictionary, NSPoint, NSRect, NSSize};
use std::path::{Path, PathBuf};

pub struct Frontmost {
    pub bundle_id: String,
    pub name: String,
    pub pid: i32,
}

pub fn frontmost() -> Option<Frontmost> {
    autoreleasepool(|_| {
        let ws = NSWorkspace::sharedWorkspace();
        let app = ws.frontmostApplication()?;
        let bundle_id = app.bundleIdentifier()?.to_string();
        let name = app
            .localizedName()
            .map(|n| n.to_string())
            .unwrap_or_else(|| bundle_id.clone());
        Some(Frontmost {
            bundle_id,
            name,
            pid: app.processIdentifier(),
        })
    })
}

/// Render the icon of the running app with `pid` as a 64pt PNG into `dir/<bundle_id>.png`.
pub fn save_app_icon(pid: i32, bundle_id: &str, dir: &Path) -> Option<PathBuf> {
    let png = autoreleasepool(|_| unsafe {
        let app =
            objc2_app_kit::NSRunningApplication::runningApplicationWithProcessIdentifier(pid)?;
        let icon = app.icon()?;
        let mut rect = NSRect::new(NSPoint::new(0.0, 0.0), NSSize::new(64.0, 64.0));
        let cg = icon.CGImageForProposedRect_context_hints(&mut rect, None, None)?;
        let rep = NSBitmapImageRep::initWithCGImage(NSBitmapImageRep::alloc(), &cg);
        let props = NSDictionary::new();
        let data = rep.representationUsingType_properties(NSBitmapImageFileType::PNG, &props)?;
        Some(data.to_vec())
    })?;
    let safe: String = bundle_id
        .chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '.' || c == '-' {
                c
            } else {
                '_'
            }
        })
        .collect();
    let path = dir.join(format!("{safe}.png"));
    std::fs::write(&path, &png).ok()?;
    Some(path)
}

/// Dominant, reasonably saturated colour of an image, as `#RRGGBB`.
/// Transparent and near-grey pixels are ignored unless the image has nothing else.
pub fn dominant_color(png: &[u8]) -> Option<String> {
    let img = image::load_from_memory(png)
        .ok()?
        .thumbnail(32, 32)
        .to_rgba8();
    let mut buckets: std::collections::HashMap<(u8, u8, u8), (u32, u64, u64, u64)> =
        Default::default();
    let mut grey: (u32, u64, u64, u64) = (0, 0, 0, 0);
    for p in img.pixels() {
        let [r, g, b, a] = p.0;
        if a < 128 {
            continue;
        }
        let max = r.max(g).max(b) as i32;
        let min = r.min(g).min(b) as i32;
        let sat = if max == 0 { 0 } else { (max - min) * 100 / max };
        if sat < 25 || max < 40 || (min > 225) {
            grey = (
                grey.0 + 1,
                grey.1 + r as u64,
                grey.2 + g as u64,
                grey.3 + b as u64,
            );
            continue;
        }
        let key = (r >> 5, g >> 5, b >> 5);
        let e = buckets.entry(key).or_default();
        *e = (e.0 + 1, e.1 + r as u64, e.2 + g as u64, e.3 + b as u64);
    }
    let best = buckets.values().max_by_key(|v| v.0).copied();
    let (n, r, g, b) = match best {
        Some(v) if v.0 >= 4 => v,
        _ if grey.0 > 0 => grey,
        _ => return None,
    };
    let n = n as u64;
    Some(format!("#{:02X}{:02X}{:02X}", r / n, g / n, b / n))
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::{ImageBuffer, Rgba};

    fn png_of(f: impl Fn(u32, u32) -> [u8; 4]) -> Vec<u8> {
        let img = ImageBuffer::from_fn(64, 64, |x, y| Rgba(f(x, y)));
        let mut out = std::io::Cursor::new(Vec::new());
        img.write_to(&mut out, image::ImageFormat::Png).unwrap();
        out.into_inner()
    }

    #[test]
    fn picks_saturated_colour_over_white() {
        let png = png_of(|x, _| {
            if x < 20 {
                [255, 255, 255, 255]
            } else {
                [220, 30, 30, 255]
            }
        });
        let c = dominant_color(&png).unwrap();
        assert!(
            c.starts_with("#D") || c.starts_with("#C") || c.starts_with("#E"),
            "{c}"
        );
    }

    #[test]
    fn grey_fallback_and_transparent() {
        assert!(dominant_color(&png_of(|_, _| [128, 128, 128, 255])).is_some());
        assert!(dominant_color(&png_of(|_, _| [0, 0, 0, 0])).is_none());
    }
}
