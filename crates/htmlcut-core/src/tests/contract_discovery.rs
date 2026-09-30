use super::*;

#[test]
fn t23_t25_pages_advance_terminate_and_bind_snapshot_options() {
    let document = prepared("<p>A</p><p>B</p><p>C</p>");
    let mut cursor = None;
    let mut handles = std::collections::HashSet::new();
    let mut total = 0;
    for _ in 0..10 {
        let page = document.inspect(2, cursor.as_deref()).unwrap();
        for element in &page.elements {
            assert!(handles.insert(element.handle.clone()));
            total += 1;
        }
        if page.next_cursor.is_none() {
            break;
        }
        assert_ne!(cursor, page.next_cursor);
        cursor = page.next_cursor;
    }
    assert_eq!(total, 6); // implicit html/head/body plus three paragraphs
    let cursor = document.inspect(2, None).unwrap().next_cursor.unwrap();
    assert!(document.inspect(3, Some(&cursor)).is_err());
    assert!(
        prepared("<p>changed</p>")
            .inspect(2, Some(&cursor))
            .is_err()
    );
    assert!(document.inspect(0, None).is_err());
    assert!(document.inspect(101, None).is_err());
}

#[test]
fn t24_preview_is_separate_and_resource_failure_stays_an_error() {
    let document = prepared("<p>αβγδε</p>");
    let mut plan = ExtractionPlan::css("p").unwrap();
    let preview = document
        .preview(&CompiledPlan::compile(&plan).unwrap(), 3)
        .unwrap();
    assert_eq!(preview.schema, "htmlcut.preview");
    assert_eq!(preview.values, ["αβγ"]);
    assert!(!preview.complete);
    assert!(
        document
            .preview(&CompiledPlan::compile(&plan).unwrap(), 5)
            .unwrap()
            .complete
    );
    plan.limits.max_value_bytes = 1;
    assert_eq!(
        document
            .preview(&CompiledPlan::compile(&plan).unwrap(), 3)
            .unwrap_err()
            .code,
        ErrorCode::ResourceLimit
    );
}

#[test]
fn t23_proposals_are_suggestions_and_require_explicit_plan_adoption() {
    let document = prepared("<p>A</p><p id='amount'>180</p>");
    let page = document.inspect(20, None).unwrap();
    let element = page
        .elements
        .iter()
        .find(|element| element.tag == "p" && element.preview == "180")
        .unwrap();
    let proposal = document.propose(&element.handle, 20).unwrap();
    assert_eq!(proposal.kind, "suggestion");
    assert_eq!(
        proposal.selector,
        "html:nth-child(1) > body:nth-child(2) > p:nth-child(2)"
    );
    assert!(document.propose(&element.handle, 10).is_err());
    assert!(
        prepared("<p>changed</p>")
            .propose(&element.handle, 20)
            .is_err()
    );
    let chosen_plan = ExtractionPlan::css(proposal.selector).unwrap();
    assert_eq!(
        document
            .execute(&CompiledPlan::compile(&chosen_plan).unwrap())
            .unwrap()
            .values,
        ["180"]
    );
    assert_eq!(document.parse_count(), 1);
}

#[test]
#[ignore = "large resource acceptance runs once in the full maintainer gate"]
fn million_element_pagination_reaches_the_tail_and_terminates() {
    let html = format!("{}<p>TAIL</p>", "<i>x</i>".repeat(999_996));
    let limits = PreparationLimits {
        max_elements: 1_000_000,
        max_nodes: 4_000_000,
        max_parse_work: 100_000_000,
        ..Default::default()
    };
    let document = PreparedDocument::new(
        SourceSnapshot::new(html, SnapshotMetadata::default()).unwrap(),
        limits,
    )
    .unwrap();
    let mut cursor = None;
    let mut total = 0;
    let mut tail = false;
    loop {
        let page = document.inspect(100, cursor.as_deref()).unwrap();
        assert!(!page.elements.is_empty());
        for element in &page.elements {
            let token: serde_json::Value = serde_json::from_str(&element.handle).unwrap();
            assert_eq!(token["position"], total);
            total += 1;
            if element.tag == "p" {
                assert_eq!(element.preview, "TAIL");
                tail = true;
            }
        }
        if page.next_cursor.is_none() {
            break;
        }
        assert_ne!(cursor, page.next_cursor);
        cursor = page.next_cursor;
    }
    assert_eq!(total, 1_000_000);
    assert!(tail);
    assert_eq!(document.parse_count(), 1);
}
