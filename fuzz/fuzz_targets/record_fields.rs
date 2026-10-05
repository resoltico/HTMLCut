// SPDX-License-Identifier: MPL-2.0
#![cfg_attr(not(test), cfg_attr(feature = "fuzzing", no_main))]
#[cfg(all(feature = "fuzzing", not(test)))]
use libfuzzer_sys::fuzz_target;
#[cfg(all(feature = "fuzzing", not(test)))]
fuzz_target!(|data: &[u8]| {
    use htmlcut_core::*;
    let alphabet: Vec<char> = "ABC<&>é✓東京".chars().collect();
    let rows = data[..data.len().min(512)]
        .chunks(64)
        .map(|chunk| {
            chunk
                .iter()
                .map(|b| alphabet[*b as usize % alphabet.len()])
                .collect::<String>()
        })
        .collect::<Vec<_>>();
    let mut source = String::new();
    let mut expected = Vec::new();
    for (index, text) in rows.iter().enumerate() {
        let escaped = text
            .replace('&', "&amp;")
            .replace('<', "&lt;")
            .replace('>', "&gt;");
        source.push_str(&format!(
            "<article id='row{index}'></article><!-- gap --><aside><p>{escaped}</p></aside>"
        ));
        expected.push(serde_json::json!({"id":format!("row{index}"),"text":text,"absent":null}));
    }
    let plan = serde_json::json!({"version":6,"select":"article","match":"all","min":0,"fields":{"id":{"select":":scope","read":"attr:id"},"text":{"select":"p","read":"literal"},"absent":{"select":".absent","match":"optional","read":"literal"}},"following_siblings":1});
    let plan = ExtractionPlan::from_json(&serde_json::to_vec(&plan).unwrap()).unwrap();
    let compiled = CompiledPlan::compile(&plan).unwrap();
    let document = PreparedDocument::new(
        SourceSnapshot::new(source, Default::default()).unwrap(),
        Default::default(),
    )
    .unwrap();
    let result = document.execute(&compiled).unwrap();
    assert_eq!(
        serde_json::to_value(result.data()).unwrap(),
        serde_json::json!(expected)
    );
    let second = document.execute(&compiled).unwrap();
    assert_eq!(result.payload(), second.payload());
    assert_eq!(result.receipt().unwrap(), second.receipt().unwrap());
});
#[cfg(any(test, not(feature = "fuzzing")))]
fn main() {}
