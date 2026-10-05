// SPDX-License-Identifier: MPL-2.0
use arbitrary::Arbitrary;
#[derive(Arbitrary, Debug)]
pub struct InspectionInput {
    html: String,
    samples: u8,
    selector: String,
}
pub fn drive(input: InspectionInput) {
    let Some(document) = crate::snapshot::document(&input.html) else {
        return;
    };
    let css = crate::snapshot::text(&input.selector, 1024);
    let first = document.inspect(css, u32::from(input.samples % 12));
    let second = document.inspect(css, u32::from(input.samples % 12));
    assert_eq!(first, second);
    if let Ok(result) = first {
        assert!(result.samples.len() <= 10);
        assert!(
            result
                .samples
                .iter()
                .all(|s| s.text.chars().count() <= 160 && s.attributes.len() <= 8)
        );
        assert_eq!(
            result.samples_complete,
            result.count as usize == result.samples.len()
        );
    }
    // Small independently constructed text checks cover the shared reading across
    // extraction, exact guards and previews, including inert and block boundaries.
    let literal = crate::snapshot::text(&input.html, 128);
    let parsed_literal = literal.replace('\0', "");
    let normalized = parsed_literal
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");
    let escaped = literal
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;");
    let controlled = format!("<main><p>{escaped}</p><script>omitted</script><p>END</p></main>");
    let expected = if normalized.is_empty() {
        "END".to_string()
    } else {
        format!("{normalized} END")
    };
    let controlled_document = crate::snapshot::document(&controlled).unwrap();
    let request = serde_json::json!({"version":6,"select":"main","expect":[{"select":"main","equals":expected}]});
    let plan =
        htmlcut_core::ExtractionPlan::from_json(&serde_json::to_vec(&request).unwrap()).unwrap();
    let result = controlled_document
        .execute(&htmlcut_core::CompiledPlan::compile(&plan).unwrap())
        .unwrap();
    assert_eq!(
        result.data().as_values().unwrap(),
        std::slice::from_ref(&expected)
    );
    let observed = controlled_document.inspect("main", 1).unwrap();
    assert_eq!(observed.samples[0].text, expected);
    assert!(observed.samples[0].text_complete);
    let limit = u32::from(input.samples % 18);
    let first = document.survey(None, limit);
    let second = document.survey(None, limit);
    assert_eq!(first, second);
    if let Ok(result) = first {
        assert!(result.groups.len() <= 16);
        assert_eq!(
            result.groups_complete,
            result.group_count as usize == result.groups.len()
        );
        assert!(result.groups.iter().all(|group| {
            group.samples.len() <= 2
                && group
                    .samples
                    .iter()
                    .all(|sample| sample.text.chars().count() <= 64)
        }));
    }
}
