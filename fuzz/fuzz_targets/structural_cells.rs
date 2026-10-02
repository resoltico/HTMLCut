#![cfg_attr(not(test), cfg_attr(feature = "fuzzing", no_main))]
#[cfg(all(feature = "fuzzing", not(test)))]
use libfuzzer_sys::fuzz_target;
#[cfg(all(feature = "fuzzing", not(test)))]
fn decode_row_cells(mut input: &str) -> Vec<String> {
    let mut cells = Vec::new();
    while !input.is_empty() {
        input = input.strip_prefix("[cell]").expect("cell opening");
        let mut payload = String::new();
        if let Some(framed) = input.strip_prefix('\n')
            && let Some((fence, rest)) = framed.split_once('\n')
            && fence.len() >= 3
            && fence.bytes().all(|byte| byte == b'`')
        {
            let closing = format!("\n{fence}[/cell]");
            let (literal, remaining) = rest.split_once(&closing).expect("balanced pre/cell");
            payload.push_str(literal);
            input = remaining;
        } else {
            loop {
                if let Some(remaining) = input.strip_prefix("[/cell]") {
                    input = remaining;
                    break;
                }
                let character = input.chars().next().expect("balanced plain cell");
                input = &input[character.len_utf8()..];
                if character == '\\' {
                    let escaped = input.chars().next().expect("complete escape");
                    assert!("\\[]()|`".contains(escaped), "reserved escape only");
                    payload.push(escaped);
                    input = &input[escaped.len_utf8()..];
                } else {
                    assert_ne!(character, '[', "unescaped structural delimiter");
                    payload.push(character);
                }
            }
        }
        cells.push(payload);
        if !input.is_empty() {
            input = input.strip_prefix(" | ").expect("separator outside cells");
        }
    }
    cells
}

#[cfg(all(feature = "fuzzing", not(test)))]
fuzz_target!(|data: &[u8]| {
    use htmlcut_core::{
        CompiledPlan, ExtractionPlan, PreparationLimits, PreparedDocument, Projection,
        SnapshotMetadata, SourceSnapshot,
    };
    let alphabet: Vec<char> = "ABC []()\\|`\n\t✓€東京<&>".chars().collect();
    let payloads: Vec<String> = data[..data.len().min(512)]
        .chunks(256)
        .map(|part| {
            part.iter()
                .map(|byte| alphabet[usize::from(*byte) % alphabet.len()])
                .collect()
        })
        .collect();
    let first = payloads.first().map(String::as_str).unwrap_or("");
    let second = payloads.get(1).map(String::as_str).unwrap_or("");
    let escape_html = |value: &str| {
        value
            .replace('&', "&amp;")
            .replace('<', "&lt;")
            .replace('>', "&gt;")
    };
    for (pre, wrapper) in [(false, ""), (false, "svg"), (false, "math"), (true, "")] {
        let first_html = if pre {
            format!("<pre><code>{}</code></pre>", escape_html(first))
        } else if wrapper.is_empty() {
            escape_html(first)
        } else {
            format!(
                "<{wrapper}><caption>{}</caption></{wrapper}>",
                escape_html(first)
            )
        };
        let html = format!(
            "<table><tr><td>{first_html}</td><td>{}</td></tr></table>",
            escape_html(second)
        );
        let snapshot = SourceSnapshot::new(html, SnapshotMetadata::default()).unwrap();
        let document = PreparedDocument::new(snapshot, PreparationLimits::default()).unwrap();
        let mut plan = ExtractionPlan::css("tr").unwrap();
        plan.projection = Projection::DocumentText {};
        let result = document
            .execute(&CompiledPlan::compile(&plan).unwrap())
            .unwrap();
        assert_eq!(result.values.len(), 1);
        assert_eq!(decode_row_cells(&result.values[0]), [first, second]);
    }
});
#[cfg(any(test, not(feature = "fuzzing")))]
fn main() {}
