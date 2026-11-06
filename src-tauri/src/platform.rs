//! Small macOS helpers: notifications, sounds, locale, accent colour, Finder, file icons.

use objc2::AllocAnyThread;
use objc2::rc::autoreleasepool;
use objc2_app_kit::{
    NSBitmapImageFileType, NSBitmapImageRep, NSColor, NSColorSpace, NSImage, NSSound, NSWorkspace,
};
use objc2_foundation::{
    NSArray, NSBundle, NSDictionary, NSLocale, NSPoint, NSRect, NSSize, NSString, NSURL,
};
use std::path::{Path, PathBuf};
use tauri::AppHandle;
use tauri_plugin_notification::NotificationExt;

pub fn notify(app: &AppHandle, title: &str, body: &str) {
    if let Err(e) = app.notification().builder().title(title).body(body).show() {
        log::warn!("notification failed: {e}");
    }
}

pub fn play_copy_sound() {
    autoreleasepool(|_| {
        if let Some(s) = NSSound::soundNamed(&NSString::from_str("Pop")) {
            s.play();
        }
    });
}

/// "it" or "en", from the user's preferred macOS languages.
pub fn system_language() -> &'static str {
    autoreleasepool(|_| {
        let langs = NSLocale::preferredLanguages();
        for l in langs.iter() {
            let l = l.to_string();
            if l.starts_with("it") {
                return "it";
            }
            if l.starts_with("en") {
                return "en";
            }
        }
        "en"
    })
}

/// The user's macOS accent colour as `#RRGGBB`.
pub fn system_accent() -> Option<String> {
    autoreleasepool(|_| {
        let c =
            NSColor::controlAccentColor().colorUsingColorSpace(&NSColorSpace::sRGBColorSpace())?;
        let to = |v: f64| (v.clamp(0.0, 1.0) * 255.0).round() as u8;
        Some(format!(
            "#{:02X}{:02X}{:02X}",
            to(c.redComponent()),
            to(c.greenComponent()),
            to(c.blueComponent())
        ))
    })
}

pub fn reveal_in_finder(paths: &[String]) {
    autoreleasepool(|_| {
        let urls: Vec<_> = paths
            .iter()
            .map(|p| NSURL::fileURLWithPath(&NSString::from_str(p)))
            .collect();
        let arr = NSArray::from_retained_slice(&urls);
        NSWorkspace::sharedWorkspace().activateFileViewerSelectingURLs(&arr);
    });
}

pub fn nsimage_png(img: &NSImage, points: f64) -> Option<Vec<u8>> {
    unsafe {
        let mut rect = NSRect::new(NSPoint::new(0.0, 0.0), NSSize::new(points, points));
        let cg = img.CGImageForProposedRect_context_hints(&mut rect, None, None)?;
        let rep = NSBitmapImageRep::initWithCGImage(NSBitmapImageRep::alloc(), &cg);
        let data = rep
            .representationUsingType_properties(NSBitmapImageFileType::PNG, &NSDictionary::new())?;
        Some(data.to_vec())
    }
}

/// Finder icon for a file, cached on disk by extension (or full path for bundles/folders).
pub fn file_icon(path: &str, cache_dir: &Path) -> Option<PathBuf> {
    let p = Path::new(path);
    let key = if p.is_dir() {
        // Apps and folders have individual icons.
        let mut h: u64 = 1469598103934665603;
        for b in path.bytes() {
            h = (h ^ b as u64).wrapping_mul(1099511628211);
        }
        format!("dir-{h:x}")
    } else {
        format!(
            "ext-{}",
            p.extension()
                .and_then(|e| e.to_str())
                .unwrap_or("none")
                .to_lowercase()
        )
    };
    let out = cache_dir.join(format!("{key}.png"));
    if out.exists() {
        return Some(out);
    }
    let png = autoreleasepool(|_| {
        let img = NSWorkspace::sharedWorkspace().iconForFile(&NSString::from_str(path));
        nsimage_png(&img, 64.0)
    })?;
    std::fs::write(&out, png).ok()?;
    Some(out)
}

/// (bundle id, display name) of an `.app` bundle on disk.
pub fn bundle_info(path: &str) -> Option<(String, Option<String>)> {
    autoreleasepool(|_| {
        let b = NSBundle::bundleWithPath(&NSString::from_str(path))?;
        let id = b.bundleIdentifier()?.to_string();
        let name = Path::new(path)
            .file_stem()
            .map(|s| s.to_string_lossy().into_owned());
        Some((id, name))
    })
}
