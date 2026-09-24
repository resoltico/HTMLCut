use htmlcut_core::{
    InspectionOptions, MaxBytes, RuntimeOptions, SourceRequest, inspect_source,
    interop::v2::{HtmlInput, PreparationLimits, prepare_document},
};

pub fn drive(data: &[u8]) {
    let html = String::from_utf8_lossy(data);
    let source = SourceRequest::memory("fuzz", html.as_ref());
    let runtime = RuntimeOptions {
        max_bytes: MaxBytes::new(html.len().max(1)).expect("non-zero fuzz byte limit"),
        ..RuntimeOptions::default()
    };

    if let Ok(input) = HtmlInput::new("fuzz", html.as_ref()) {
        let _ = prepare_document(input, PreparationLimits::default());
    }
    let _ = inspect_source(&source, &runtime, &InspectionOptions::default());
}
