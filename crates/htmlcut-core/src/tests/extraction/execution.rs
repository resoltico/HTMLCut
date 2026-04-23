use super::*;

#[test]
fn extraction_runs_cover_selector_and_slice_candidate_selection_branches() {
    let mut selector_no_match_request = selector_request("<article>Hello</article>");
    selector_no_match_request.extraction = ExtractionSpec::selector(selector_query("aside"));
    let selector_no_match = extract(&selector_no_match_request, &RuntimeOptions::default());
    assert!(!selector_no_match.ok);
    assert_eq!(selector_no_match.diagnostics[0].code, "NO_MATCH");

    let selector_multiple = extract(
        &selector_request("<article>One</article><article>Two</article>"),
        &RuntimeOptions::default(),
    );
    assert!(selector_multiple.ok);
    assert!(
        selector_multiple
            .diagnostics
            .iter()
            .any(|item| item.code == "MULTIPLE_MATCHES")
    );

    let slice_no_match = extract(
        &slice_request("<div>Hello</div>", "<section>", "</section>"),
        &RuntimeOptions::default(),
    );
    assert!(!slice_no_match.ok);
    assert_eq!(slice_no_match.diagnostics[0].code, "NO_MATCH");

    let slice_multiple = extract(
        &slice_request(
            "<article>One</article><article>Two</article>",
            "<article>",
            "</article>",
        ),
        &RuntimeOptions::default(),
    );
    assert!(slice_multiple.ok);
    assert!(
        slice_multiple
            .diagnostics
            .iter()
            .any(|item| item.code == "MULTIPLE_MATCHES")
    );
}
