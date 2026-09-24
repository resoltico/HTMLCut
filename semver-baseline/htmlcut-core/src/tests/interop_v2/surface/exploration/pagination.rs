use super::*;

#[test]
fn exact_terminal_page_has_no_cursor_or_phantom_element_limit() {
    let document = prepare_document(
        HtmlInput::new("terminal", "<main>One</main>").expect("source"),
        PreparationLimits::default(),
    )
    .expect("prepared");
    let exact = ExplorationOptions {
        max_elements: NonZeroU32::new(4).expect("non-zero"),
        ..ExplorationOptions::default()
    };
    let result = explore(&document, &exact).expect("exact page");
    assert_eq!(result.elements.len(), 4);
    assert_eq!(result.next_cursor, None);
    assert_eq!(result.truncation_reason, None);

    let first_options = ExplorationOptions {
        max_elements: NonZeroU32::new(2).expect("non-zero"),
        ..ExplorationOptions::default()
    };
    let first = explore(&document, &first_options).expect("first page");
    let last = explore(
        &document,
        &ExplorationOptions {
            cursor: first.next_cursor,
            ..first_options
        },
    )
    .expect("last page");
    assert_eq!(last.elements.len(), 2);
    assert_eq!(last.next_cursor, None);
    assert_eq!(last.truncation_reason, None);
}

#[test]
fn cursor_rejects_bad_digest_encoding_and_out_of_range_ordinal() {
    let document = prepare_document(
        HtmlInput::new("cursor-shape", "<main>One</main>").expect("source"),
        PreparationLimits::default(),
    )
    .expect("prepared");
    let options = ExplorationOptions {
        max_elements: NonZeroU32::new(2).expect("non-zero"),
        ..ExplorationOptions::default()
    };
    let page = explore(&document, &options).expect("first page");
    let cursor = page.next_cursor.expect("continuation");
    for (field, value) in [
        ("discovery_identity_sha256", serde_json::json!("X")),
        ("options_digest_sha256", serde_json::json!("X")),
        ("next_document_ordinal", serde_json::json!(10)),
    ] {
        let mut encoded = serde_json::to_value(&cursor).expect("cursor JSON");
        encoded[field] = value;
        let invalid = serde_json::from_value(encoded).expect("shape remains deserializable");
        let error = explore(
            &document,
            &ExplorationOptions {
                cursor: Some(invalid),
                ..options.clone()
            },
        )
        .expect_err("invalid cursor");
        assert_eq!(error.error_code, ExplorationErrorCode::InvalidCursor);
    }
}

#[test]
fn large_static_document_reaches_body_under_default_budget_and_paginates_to_tail() {
    let html = format!("<main>{}</main>", "<div></div>".repeat(1_100));
    let document = prepare_document(
        HtmlInput::new("many-siblings", html).expect("source"),
        PreparationLimits::default(),
    )
    .expect("prepared");
    let default_page = explore(&document, &ExplorationOptions::default()).expect("default page");
    assert!(
        default_page
            .elements
            .iter()
            .any(|element| element.element_name.local_name == "body")
    );

    let mut options = ExplorationOptions {
        max_elements: NonZeroU32::new(1_000).expect("non-zero"),
        max_work_units: NonZeroU32::new(5_000_000).expect("non-zero"),
        ..ExplorationOptions::default()
    };
    let mut paths = std::collections::BTreeSet::new();
    loop {
        let page = explore(&document, &options).expect("bounded page");
        for element in &page.elements {
            assert!(paths.insert(element.path.clone()), "duplicate page element");
        }
        let Some(cursor) = page.next_cursor else {
            assert_eq!(page.truncation_reason, None);
            break;
        };
        options.cursor = Some(cursor);
    }
    assert_eq!(paths.len(), 1_104);
    assert!(paths.contains("html:html[1]/html:body[1]/html:main[1]/html:div[1100]"));
}

#[test]
fn tiny_work_budget_records_omissions_and_advances_to_a_terminal_page() {
    let document = prepare_document(
        HtmlInput::new("tiny-work", "<main>One</main>").expect("source"),
        PreparationLimits::default(),
    )
    .expect("prepared");
    let mut options = ExplorationOptions {
        max_work_units: NonZeroU32::new(1).expect("non-zero"),
        ..ExplorationOptions::default()
    };
    for expected_ordinal in 2..=4 {
        let page = explore(&document, &options).expect("bounded page");
        assert!(page.elements.is_empty());
        assert_eq!(
            page.truncation_reason,
            Some(ExplorationTruncationReason::WorkLimit)
        );
        let cursor = page.next_cursor.expect("advancing cursor");
        assert_eq!(cursor.next_document_ordinal().get(), expected_ordinal);
        options.cursor = Some(cursor);
    }
    let last = explore(&document, &options).expect("terminal page");
    assert!(last.elements.is_empty());
    assert_eq!(last.next_cursor, None);
    assert_eq!(
        last.truncation_reason,
        Some(ExplorationTruncationReason::WorkLimit)
    );
}
