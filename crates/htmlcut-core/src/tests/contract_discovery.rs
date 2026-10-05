// SPDX-License-Identifier: MPL-2.0
use super::*;

#[test]
fn identifier_inspection_keeps_the_count_exact_and_separates_table_cells() {
    let document = prepared(
        "<main id='content' class='main docs main' data-secret='token'><table><tr><td>1,429,404,000</td><td>17.3%</td></tr></table></main><main id='other'>Other</main>",
    );
    let result = document.inspect_identifiers("main", 1).unwrap();
    assert_eq!(result.count, 2);
    assert!(!result.samples_complete);
    let sample = &result.samples[0];
    assert_eq!(sample.tag, "main");
    assert_eq!(sample.id.as_deref(), Some("content"));
    assert_eq!(sample.classes, ["docs", "main"]);
    assert!(sample.identifiers_complete && sample.text_complete);
    assert_eq!(sample.text, "1,429,404,000 17.3%");
    assert!(!crate::canonical_json(&result).unwrap().contains("token"));
    let plain = document.inspect("main", 1).unwrap();
    assert_eq!(plain.samples[0].text, "1,429,404,00017.3%");
}

#[test]
fn identifier_samples_omit_oversized_tokens_and_label_incompleteness() {
    let classes = (0..9).map(|i| format!("c{i}")).collect::<Vec<_>>();
    let source = format!(
        "<main id='{}' class='{} {}'>a<svg><text>b</text></svg>c</main>",
        "x".repeat(129),
        classes.join(" "),
        "long".repeat(17)
    );
    let sample = prepared(&source)
        .inspect_identifiers("main", 1)
        .unwrap()
        .samples
        .remove(0);
    assert_eq!(sample.id, None);
    assert_eq!(sample.classes, classes[..8]);
    assert!(!sample.identifiers_complete);
    assert_eq!(sample.text, "abc");
    assert!(sample.text_complete);
}

#[test]
fn identifier_preview_marks_a_structural_boundary_that_exceeds_its_limit() {
    for (count, complete) in [(158, true), (159, false)] {
        let source = format!(
            "<table><tr><td>{}</td><td>x</td></tr></table>",
            "a".repeat(count)
        );
        let sample = prepared(&source)
            .inspect_identifiers("table", 1)
            .unwrap()
            .samples
            .remove(0);
        assert_eq!(
            sample.text,
            if complete {
                format!("{} x", "a".repeat(count))
            } else {
                "a".repeat(count)
            }
        );
        assert_eq!(sample.text_complete, complete);
    }
}

#[test]
fn identifier_inspection_labels_empty_results_and_rejects_invalid_requests_before_parsing() {
    let document = prepared("<p>a\u{a0}b</p>");
    assert_eq!(
        document.inspect_identifiers("p", 1).unwrap().samples[0].text,
        "a b"
    );
    let missing = document.inspect_identifiers("aside", 1).unwrap();
    assert_eq!(missing.count, 0);
    assert!(missing.samples_complete && missing.samples.is_empty());

    let lazy = prepared("<p>x</p>");
    assert_eq!(
        lazy.inspect_identifiers("[", 1).unwrap_err().code,
        ErrorCode::InvalidSelector
    );
    assert_eq!(
        lazy.inspect_identifiers("p", 0).unwrap_err().code,
        ErrorCode::InvalidOptions
    );
    assert_eq!(lazy.parse_count(), 0);
    let tag = "x".repeat(129);
    assert_eq!(
        prepared(&format!("<{tag}></{tag}>"))
            .inspect_identifiers(&tag, 1)
            .unwrap_err()
            .code,
        ErrorCode::ResourceLimit
    );
}

#[test]
fn identifier_inspection_refuses_an_oversized_encoded_answer() {
    let tag = "x".repeat(128);
    let classes = (0..8)
        .map(|i| format!("c{i}{}", "x".repeat(62)))
        .collect::<Vec<_>>()
        .join(" ");
    let element = format!(
        "<{tag} id='{}' class='{classes}'>{}</{tag}>",
        "i".repeat(128),
        "\u{1}".repeat(160)
    );
    let document = prepared(&element.repeat(10));
    assert_eq!(
        document.inspect_identifiers(&tag, 10).unwrap_err().code,
        ErrorCode::ResourceLimit
    );
}

#[test]
fn selector_count_is_complete_while_samples_are_explicitly_abbreviated() {
    let document = prepared("<p>A</p><p>B</p><p>C</p><p>D</p>");
    let result = document.inspect("p", 3).unwrap();
    assert_eq!(result.count, 4);
    assert_eq!(result.samples.len(), 3);
    assert!(!result.samples_complete);
    assert_eq!(
        result
            .samples
            .iter()
            .map(|s| s.text.as_str())
            .collect::<Vec<_>>(),
        ["A", "B", "C"]
    );
    let complete = document.inspect("p", 4).unwrap();
    assert!(complete.samples_complete);
    let missing = document.inspect("aside", 3).unwrap();
    assert_eq!(missing.count, 0);
    assert!(missing.samples_complete);
}

#[test]
fn preview_limits_count_unicode_scalars_and_never_truncate_extraction() {
    for count in [0, 159, 160, 161] {
        let text = "é".repeat(count);
        let document = prepared(&format!("<p>{text}</p>"));
        let inspected = document.inspect("p", 3).unwrap();
        assert_eq!(inspected.samples[0].text.chars().count(), count.min(160));
        assert_eq!(inspected.samples[0].text_complete, count <= 160);
        let result = document
            .execute(&CompiledPlan::compile(&ExtractionPlan::css("p").unwrap()).unwrap())
            .unwrap();
        assert_eq!(result.data.as_values().unwrap(), [text]);
    }
}

#[test]
fn inspection_names_are_bounded_and_complete_flags_are_honest() {
    let attrs = (0..10)
        .map(|i| format!(" a{i}='value'"))
        .collect::<String>();
    let oversized = "x".repeat(257);
    let document = prepared(&format!("<p{attrs} {oversized}='secret'> X  Y </p>"));
    let sample = &document.inspect("p", 3).unwrap().samples[0];
    assert_eq!(sample.attributes.len(), 8);
    assert!(!sample.attributes_complete);
    assert_eq!(
        sample.attributes,
        ["a0", "a1", "a2", "a3", "a4", "a5", "a6", "a7"]
    );
    assert_eq!(sample.text, "X Y");
    assert!(sample.text_complete);
    assert!(!crate::canonical_json(sample).unwrap().contains("secret"));
}

#[test]
fn inspection_rejects_bad_grammar_and_limits_before_lazy_preparation() {
    let document = prepared("<p>x</p>");
    for samples in [0, 11, u32::MAX] {
        assert!(document.inspect("p", samples).is_err());
    }
    assert_eq!(
        document.inspect("[", 3).unwrap_err().code,
        ErrorCode::InvalidSelector
    );
    assert!(document.inspect("", 3).is_err());
    assert_eq!(document.parse_count(), 0);
    let document = prepared(&format!("<{}>X</{}>", "x".repeat(129), "x".repeat(129)));
    assert_eq!(
        document.inspect(&"x".repeat(129), 3).unwrap_err().code,
        ErrorCode::ResourceLimit
    );
}

#[test]
fn heavy_match_tail_exhaustion_is_not_a_successful_partial_count() {
    let source = format!("<main>{}</main>", "<p>x</p>".repeat(100_001));
    let document = prepared(&source);
    assert_eq!(
        document.inspect("p", 3).unwrap_err().code,
        ErrorCode::ResourceLimit
    );
}

#[test]
#[ignore = "large parser/work acceptance is executed once by the maintained full gate"]
fn million_element_selector_inspection_fails_closed() {
    let source = format!("<main>{}</main>", "<p>x</p>".repeat(1_000_000));
    let document = PreparedDocument::new(
        SourceSnapshot::new(source, Default::default()).unwrap(),
        PreparationLimits {
            max_elements: 1_000_000,
            max_nodes: 4_000_000,
            max_parse_work: 100_000_000,
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(
        document.inspect("p", 3).unwrap_err().code,
        ErrorCode::ResourceLimit
    );
}

#[test]
fn normalized_spaces_reserve_a_complete_scalar_pair_at_the_preview_boundary() {
    for (source, expected, complete) in [
        (
            format!("  {}   ✓  ", "a".repeat(158)),
            format!("{} ✓", "a".repeat(158)),
            true,
        ),
        (
            format!("  {}   ✓  ", "a".repeat(159)),
            "a".repeat(159),
            false,
        ),
        (
            format!("{}✓", "é ".repeat(80)),
            format!("{}é", "é ".repeat(79)),
            false,
        ),
    ] {
        let sample = prepared(&format!("<p>{source}</p>"))
            .inspect("p", 1)
            .unwrap()
            .samples
            .remove(0);
        assert_eq!(sample.text, expected);
        assert_eq!(sample.text_complete, complete);
        assert!(sample.text.chars().count() <= 160);
    }
}

#[test]
fn full_supported_names_are_retained_and_the_complete_answer_cap_is_not_truncation() {
    let names = (0..8)
        .map(|i| format!("x{i}{}", "a".repeat(254)))
        .collect::<Vec<_>>();
    let attributes = names
        .iter()
        .map(|name| format!(" {name}='value'"))
        .collect::<String>();
    let source = format!("<p{attributes}>{}</p>", "✓".repeat(160));
    let document = prepared(&source.repeat(10));
    let one = document.inspect("p", 1).unwrap();
    assert_eq!(one.count, 10);
    assert_eq!(one.samples[0].attributes, names);
    assert!(one.samples[0].attributes_complete);
    assert_eq!(one.samples[0].text, "✓".repeat(160));
    assert!(one.samples[0].text_complete);
    assert_eq!(
        document.inspect("p", 10).unwrap_err().code,
        ErrorCode::ResourceLimit
    );
    let tag = "x".repeat(128);
    assert_eq!(
        prepared(&format!("<{tag}></{tag}>"))
            .inspect(&tag, 1)
            .unwrap()
            .samples[0]
            .tag,
        tag
    );
}
