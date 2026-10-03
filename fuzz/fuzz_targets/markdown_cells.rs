#![cfg_attr(not(test), cfg_attr(feature = "fuzzing", no_main))]
#[cfg(all(feature = "fuzzing", not(test)))]
use libfuzzer_sys::fuzz_target;
#[cfg(all(feature = "fuzzing", not(test)))]
fn decode_row_cells(input: &str) -> Vec<String> {
    use pulldown_cmark::{Event, Parser, Tag, TagEnd};
    let mut depth = 0;
    let mut cells = Vec::new();
    let mut current = None::<String>;
    for event in Parser::new(input) {
        match event {
            Event::Start(Tag::List(_)) => depth += 1,
            Event::End(TagEnd::List(_)) => depth -= 1,
            Event::Start(Tag::Item) if depth == 2 => current = Some(String::new()),
            Event::End(TagEnd::Item) if depth == 2 => cells.push(current.take().unwrap()),
            Event::Text(text) if current.is_some() => current.as_mut().unwrap().push_str(&text),
            Event::End(TagEnd::CodeBlock) => {
                current.as_mut().unwrap().pop();
            }
            _ => (),
        }
    }
    cells
}

#[cfg(all(feature = "fuzzing", not(test)))]
fuzz_target!(|data: &[u8]| {
    use htmlcut_core::{
        CompiledPlan, ExtractionPlan, PreparationLimits, PreparedDocument, Projection,
        SnapshotMetadata, SourceSnapshot, ValueProjection,
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
        plan.projection = Projection::Value(ValueProjection::Markdown {});
        let result = document
            .execute(&CompiledPlan::compile(&plan).unwrap())
            .unwrap();
        assert_eq!(result.data.as_values().unwrap().len(), 1);
        let expected_first = if pre {
            first.to_owned()
        } else {
            first.split_ascii_whitespace().collect::<Vec<_>>().join(" ")
        };
        let expected_second = second
            .split_ascii_whitespace()
            .collect::<Vec<_>>()
            .join(" ");
        assert_eq!(
            decode_row_cells(&result.data.as_values().unwrap()[0]),
            [expected_first, expected_second]
        );
    }
});
#[cfg(any(test, not(feature = "fuzzing")))]
fn main() {}
