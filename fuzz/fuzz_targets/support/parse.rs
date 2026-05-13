use htmlcut_core::{
    InspectionOptions, MaxBytes, RuntimeOptions, SourceRequest, inspect_source, parse_document,
};

pub fn drive(data: &[u8]) {
    let html = String::from_utf8_lossy(data);
    let source = SourceRequest::memory("fuzz", html.as_ref());
    let runtime = RuntimeOptions {
        max_bytes: MaxBytes::new(html.len().max(1)).expect("non-zero fuzz byte limit"),
        ..RuntimeOptions::default()
    };

    let _ = parse_document(&source, &runtime);
    let _ = inspect_source(&source, &runtime, &InspectionOptions::default());
}
