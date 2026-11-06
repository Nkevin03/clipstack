//! On-device OCR of copied images with Apple's Vision framework (no network).
//! The recognised text is stored in `items.ocr_text` and becomes searchable via FTS.

use crate::db::queries as q;
use crate::state::AppState;
use objc2::AllocAnyThread;
use objc2::rc::autoreleasepool;
use objc2::runtime::AnyObject;
use objc2_foundation::{NSArray, NSDictionary, NSString, NSURL};
use objc2_vision::{
    VNImageOption, VNImageRequestHandler, VNRecognizeTextRequest, VNRequest,
    VNRequestTextRecognitionLevel,
};
use std::sync::Arc;
use tauri::{AppHandle, Emitter, Manager};

pub fn recognize(path: &str) -> Option<String> {
    autoreleasepool(|_| unsafe {
        let url = NSURL::fileURLWithPath(&NSString::from_str(path));
        let opts: objc2::rc::Retained<NSDictionary<VNImageOption, AnyObject>> = NSDictionary::new();
        let handler =
            VNImageRequestHandler::initWithURL_options(VNImageRequestHandler::alloc(), &url, &opts);
        let request = VNRecognizeTextRequest::new();
        request.setRecognitionLevel(VNRequestTextRecognitionLevel::Accurate);
        request.setUsesLanguageCorrection(true);
        request.setAutomaticallyDetectsLanguage(true);
        let as_req: &VNRequest = &request;
        let arr = NSArray::from_slice(&[as_req]);
        handler.performRequests_error(&arr).ok()?;
        let results = request.results()?;
        let lines: Vec<String> = results
            .iter()
            .filter_map(|obs| {
                obs.topCandidates(1)
                    .firstObject()
                    .map(|t| t.string().to_string())
            })
            .collect();
        let text = lines.join("\n").trim().to_string();
        (!text.is_empty()).then_some(text)
    })
}

pub fn spawn(app: AppHandle, id: i64, path: Option<String>) {
    let Some(path) = path else { return };
    std::thread::Builder::new()
        .name("ocr".into())
        .spawn(move || {
            if let Some(text) = recognize(&path) {
                let state = app.state::<Arc<AppState>>();
                if q::set_ocr_text(&state.db(), id, &text).is_ok() {
                    log::info!("ocr #{id}: {} chars", text.len());
                    let _ = app.emit("items:changed", ());
                }
            }
        })
        .ok();
}
