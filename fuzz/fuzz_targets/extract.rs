// SPDX-License-Identifier: MPL-2.0
#![no_main]
use htmlcut_core::{
    Contract, Document, ErrorCode, ExportFormat, OutlineOptions, ParseOptions, ResultLimits,
};
use libfuzzer_sys::fuzz_target;

fn admitted<T>(result: Result<T, htmlcut_core::Error>) -> Option<T> {
    match result {
        Ok(value) => Some(value),
        Err(error) => {
            assert_ne!(error.code(), ErrorCode::Internal, "{error:?}");
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
    let mut limits = ParseOptions::default();
    let mut results = ResultLimits::default();
    limits.max_input_bytes = 65536;
    results.max_output_bytes = 65536;
    results.max_reading_bytes = if mode & 2 == 0 {
        65536
    } else {
        1 + bytes.len() % 256
    };
    // The process owner and libFuzzer bound wall time/RSS. A load-dependent
    // cooperative deadline would silently skip later operations in this input.
    let context = htmlcut_core::Stop::none();
    if mode & 1 != 0 {
        // Refusal callbacks remain reachable without compiling arbitrary contract
        // bytes, while each iteration decodes/parses its document only once.
        limits.max_nodes = usize::from(mode >> 2) % 33;
        let _ = admitted(Document::parse(document, &limits, &context));
        return;
    }
    limits.max_nodes = 4096;
    let Ok(text) = std::str::from_utf8(contract) else {
        return;
    };
    let Some(contract) = admitted(Contract::from_json(text, &context)) else {
        return;
    };
    if let Some(document) = admitted(Document::parse(document, &limits, &context)) {
        for format in [ExportFormat::Json, ExportFormat::Raw] {
            if let Some(value) = admitted(document.extract(&contract, format, &results, &context)) {
                match value.export(format, results.max_output_bytes, &context) {
                    Ok(bytes) => assert!(bytes.len() <= results.max_output_bytes),
                    Err(error) if format == ExportFormat::Raw => {
                        assert_eq!(error.code(), ErrorCode::UnrepresentableOutput)
                    }
                    Err(error) => panic!("admitted JSON extraction failed export: {error}"),
                }
            }
        }
        if let Some(report) =
            admitted(document.outline(&OutlineOptions::default(), &results, &context))
        {
            let text = report
                .render(results.max_output_bytes, &context)
                .expect("an admitted outline must render within its output allowance");
            assert!(text.len() <= results.max_output_bytes);
        }
    }
});
