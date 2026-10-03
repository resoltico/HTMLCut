use super::*;

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
