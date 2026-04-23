use super::*;

#[test]
fn extraction_specs_cover_optional_selector_and_slice_views() {
    assert!(SelectionSpec::single().index().is_none());
    assert!(SelectionSpec::First.index().is_none());
    assert!(SelectionSpec::All.index().is_none());
    assert_eq!(
        SelectionSpec::nth(NonZeroUsize::new(2).expect("index")).index(),
        Some(NonZeroUsize::new(2).expect("index"))
    );

    let selector = ExtractionSpec::selector(selector_query("article"));
    assert_eq!(
        selector
            .selector_query()
            .expect("selector query should exist")
            .as_ref(),
        "article"
    );
    assert!(selector.slice_spec().is_none());

    let slice = ExtractionSpec::slice(slice_spec("<article>", "</article>"));
    assert!(slice.selector_query().is_none());
    assert!(slice.slice_spec().is_some());

    assert!(ValueSpec::Text.attribute_name().is_none());
    assert!(ValueSpec::InnerHtml.attribute_name().is_none());
    assert!(ValueSpec::OuterHtml.attribute_name().is_none());
    assert!(ValueSpec::Structured.attribute_name().is_none());
}
