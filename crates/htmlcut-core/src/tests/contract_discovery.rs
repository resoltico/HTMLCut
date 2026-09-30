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

fn reseal(token: &mut serde_json::Value) {
    let position = (token["position"].as_u64().unwrap() as u32).to_be_bytes();
    token["seal"] = serde_json::json!(crate::identity::framed(
        "htmlcut.discovery-token/1",
        &[
            token["prepared"].as_str().unwrap().as_bytes(),
            token["options"].as_str().unwrap().as_bytes(),
            &position,
            token["role"].as_str().unwrap().as_bytes()
        ]
    ));
}

#[test]
fn t25_cursor_handles_reject_malformed_tampered_role_and_out_of_range_evidence() {
    let doc = prepared("<p>x</p>");
    let page = doc.inspect(1, None).unwrap();
    for value in ["x".to_string(), "{}".into(), "x".repeat(1025)] {
        assert!(doc.inspect(1, Some(&value)).is_err());
    }
    let cursor = page.next_cursor.unwrap();
    let mut token: serde_json::Value = serde_json::from_str(&cursor).unwrap();
    token["position"] = serde_json::json!(100);
    let altered = serde_json::to_string(&token).unwrap();
    assert!(doc.inspect(1, Some(&altered)).is_err());
    reseal(&mut token);
    let out_of_range = serde_json::to_string(&token).unwrap();
    assert!(doc.inspect(1, Some(&out_of_range)).is_err());
    assert!(doc.propose(&cursor, 1).is_err());
    assert!(doc.propose(&page.elements[0].handle, 0).is_err());
    assert!(doc.propose(&page.elements[0].handle, 101).is_err());
    let mut handle: serde_json::Value = serde_json::from_str(&page.elements[0].handle).unwrap();
    handle["position"] = serde_json::json!(100);
    reseal(&mut handle);
    assert!(
        doc.propose(&serde_json::to_string(&handle).unwrap(), 1)
            .is_err()
    );
    let changed = PreparedDocument::new(
        SourceSnapshot::new(
            "<p>x</p>",
            SnapshotMetadata {
                base_url: Some("https://example.test/".into()),
            },
        )
        .unwrap(),
        Default::default(),
    )
    .unwrap();
    assert!(changed.inspect(1, Some(&cursor)).is_err());
    let mut fields: serde_json::Value = serde_json::from_str(&cursor).unwrap();
    fields["extra"] = serde_json::json!(true);
    assert!(
        doc.inspect(1, Some(&serde_json::to_string(&fields).unwrap()))
            .is_err()
    );
    let empty = doc
        .inspect(
            1,
            Some(
                &crate::canonical_json(&{
                    token["position"] = serde_json::json!(4);
                    reseal(&mut token);
                    token
                })
                .unwrap(),
            ),
        )
        .unwrap();
    assert!(empty.elements.is_empty());
    assert!(empty.next_cursor.is_none());
}

#[test]
fn t23_t24_descriptors_are_bounded_and_mark_attribute_or_value_truncation() {
    let html = format!(
        "<p a='{}' b='2' c='3' d='4' e='5' f='6' g='7' h='8' i='9'>{}</p>",
        "é".repeat(65),
        "x".repeat(65)
    );
    let doc = prepared(&html);
    let page = doc.inspect(20, None).unwrap();
    let p = page.elements.iter().find(|e| e.tag == "p").unwrap();
    assert_eq!(p.attributes.len(), 8);
    assert!(!p.attributes_complete);
    assert!(!p.attributes[0].complete);
    assert_eq!(p.attributes[0].value.chars().count(), 64);
    assert!(!p.complete);
    assert_eq!(p.preview.len(), 64);
    let long = format!("<{}>x</{}>", "a".repeat(129), "a".repeat(129));
    assert_eq!(
        prepared(&long).inspect(20, None).unwrap_err().code,
        ErrorCode::ResourceLimit
    );
    let long = format!("<p {}='x'>x</p>", "a".repeat(129));
    assert_eq!(
        prepared(&long).inspect(20, None).unwrap_err().code,
        ErrorCode::ResourceLimit
    );
    let plan = CompiledPlan::compile(&ExtractionPlan::css("p").unwrap()).unwrap();
    for maximum in [0, 4097] {
        assert!(doc.preview(&plan, maximum).is_err());
    }
    let many = prepared(&"<p></p>".repeat(21));
    let mut all = ExtractionPlan::css("p").unwrap();
    all.selection = Selection::All { min: 1, max: None };
    let preview = many
        .preview(&CompiledPlan::compile(&all).unwrap(), 128)
        .unwrap();
    assert_eq!(preview.values.len(), 20);
    assert!(!preview.complete);
}

#[test]
fn t23_proposals_bound_ancestry_and_escape_names_without_using_comment_siblings() {
    let document = prepared("<p>A</p><!--between--><p>B</p>");
    let page = document.inspect(20, None).unwrap();
    let element = page.elements.iter().find(|e| e.preview == "B").unwrap();
    let proposal = document.propose(&element.handle, 20).unwrap();
    assert!(proposal.selector.ends_with("p:nth-child(2)"));
    assert_eq!(
        document
            .execute(
                &CompiledPlan::compile(&ExtractionPlan::css(proposal.selector).unwrap()).unwrap()
            )
            .unwrap()
            .values,
        ["B"]
    );
    let long = prepared(&format!("<{}>x</{}>", "a".repeat(129), "a".repeat(129)));
    let mut token: serde_json::Value =
        serde_json::from_str(&long.inspect(1, None).unwrap().elements[0].handle).unwrap();
    token["position"] = serde_json::json!(3);
    reseal(&mut token);
    assert_eq!(
        long.propose(&crate::canonical_json(&token).unwrap(), 1)
            .unwrap_err()
            .code,
        ErrorCode::ResourceLimit
    );
    let tag = "a".repeat(100);
    let html = format!(
        "{}<p>tail</p>{}",
        format!("<{tag}>").repeat(80),
        format!("</{tag}>").repeat(80)
    );
    let deep = prepared(&html);
    let page = deep.inspect(100, None).unwrap();
    let tail = page.elements.iter().find(|e| e.tag == "p").unwrap();
    assert_eq!(
        deep.propose(&tail.handle, 100).unwrap_err().code,
        ErrorCode::ResourceLimit
    );
}
