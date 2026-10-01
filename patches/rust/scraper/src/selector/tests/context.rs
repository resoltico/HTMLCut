//! Scoped identity restoration at the generic matcher context boundary.
use super::Simple;
use crate::{ElementRef, Html, Selector};
use selectors::matching::{
    MatchingContext, MatchingForInvalidation, MatchingMode, NeedsSelectorFlags, QuirksMode,
    SelectorCaches,
};

#[test]
fn nested_host_context_restores_the_previous_identity() {
    let document = Html::parse_document("<main><aside></aside></main>");
    let root = document
        .select(&Selector::parse("main").unwrap())
        .next()
        .unwrap();
    let child = document
        .select(&Selector::parse("aside").unwrap())
        .next()
        .unwrap();
    let mut caches = SelectorCaches::default();
    let mut context = MatchingContext::<Simple>::new(
        MatchingMode::Normal,
        None,
        &mut caches,
        QuirksMode::NoQuirks,
        NeedsSelectorFlags::No,
        MatchingForInvalidation::No,
    );
    assert_eq!(context.shadow_host(), None);
    context.with_shadow_host(Some(root), |outer| {
        assert_eq!(outer.shadow_host(), Some(selectors::Element::opaque(&root)));
        outer.with_shadow_host(Some(child), |inner| {
            assert_eq!(
                inner.shadow_host(),
                Some(selectors::Element::opaque(&child))
            )
        });
        assert_eq!(outer.shadow_host(), Some(selectors::Element::opaque(&root)));
        outer.with_shadow_host(None::<ElementRef>, |inner| {
            assert_eq!(inner.shadow_host(), None)
        });
        assert_eq!(outer.shadow_host(), Some(selectors::Element::opaque(&root)));
    });
    assert_eq!(context.shadow_host(), None);
}
