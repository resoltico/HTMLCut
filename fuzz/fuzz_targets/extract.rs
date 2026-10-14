// SPDX-License-Identifier: MPL-2.0
#![no_main]
use htmlcut_core::{Contract, Document, Limits, OutlineOptions, Source};
use libfuzzer_sys::fuzz_target;
fuzz_target!(|bytes: &[u8]| {
    if bytes.len() < 2 {
        return;
    }
    let length = usize::from(u16::from_le_bytes([bytes[0], bytes[1]])).min(bytes.len() - 2);
    let (contract, document) = bytes[2..].split_at(length);
    let Ok(text) = std::str::from_utf8(contract) else {
        return;
    };
    let Ok(contract) = Contract::from_json(text) else {
        return;
    };
    let mut limits = Limits::default();
    limits.max_input_bytes = 65536;
    limits.max_nodes = 4096;
    limits.max_output_bytes = 65536;
    limits.time_limit = Some(std::time::Duration::from_millis(50));
    if let Ok(document) = Document::parse(Source::new(document), &limits) {
        let _ = document.extract(&contract);
        let _ = document.outline(&OutlineOptions::default());
    }
});
