// SPDX-License-Identifier: MPL-2.0
#![no_main]
use htmlcut_core::{Contract, Document, ErrorCode, ExportFormat, Limits, OutlineOptions, Source};
use libfuzzer_sys::fuzz_target;

fn admitted<T>(result: Result<T, htmlcut_core::Error>) -> Option<T> {
    match result {
        Ok(value) => Some(value),
        Err(error) => {
            assert_ne!(error.error, ErrorCode::Internal, "{error:?}");
            None
        }
    }
}

fuzz_target!(|bytes: &[u8]| {
    if bytes.len() < 3 {
        return;
    }
    let mode = bytes[0];
    let length = usize::from(u16::from_le_bytes([bytes[1], bytes[2]])).min(bytes.len() - 3);
    let (contract, document) = bytes[3..].split_at(length);
    let mut limits = Limits::default();
    limits.max_input_bytes = 65536;
    limits.max_output_bytes = 65536;
    limits.max_reading_bytes = if mode & 2 == 0 {
        65536
    } else {
        1 + bytes.len() % 256
    };
    // The process owner and libFuzzer bound wall time/RSS. A load-dependent
    // cooperative deadline would silently skip later operations in this input.
    let context = htmlcut_core::ExecutionContext::new(None).unwrap();
    if mode & 1 != 0 {
        // Refusal callbacks remain reachable without compiling arbitrary contract
        // bytes, while each iteration decodes/parses its document only once.
        limits.max_nodes = usize::from(mode >> 2) % 33;
        let _ = admitted(Document::parse(
            Source::new(document),
            &limits.preparation(),
            &context,
        ));
        return;
    }
    limits.max_nodes = 4096;
    let Ok(text) = std::str::from_utf8(contract) else {
        return;
    };
    let Some(contract) = admitted(Contract::from_json(text, &context)) else {
        return;
    };
    if let Some(document) = admitted(Document::parse(
        Source::new(document),
        &limits.preparation(),
        &context,
    )) {
        if let Some(value) = admitted(document.extract(&contract, &limits.execution(), &context)) {
            let json = value
                .export(ExportFormat::Json, limits.max_output_bytes, &context)
                .expect("an admitted extraction must export within its canonical allowance");
            assert!(json.len() <= limits.max_output_bytes);
            match value.export(ExportFormat::Raw, limits.max_output_bytes, &context) {
                Ok(raw) => assert!(raw.len() <= json.len()),
                Err(error) => assert_eq!(error.error, ErrorCode::UnrepresentableOutput),
            }
        }
        if let Some(report) =
            admitted(document.outline(&OutlineOptions::default(), &limits.execution(), &context))
        {
            let text = report
                .render(limits.max_output_bytes, &context)
                .expect("an admitted outline must render within its canonical allowance");
            assert!(text.len() <= limits.max_output_bytes);
        }
    }
});
