use super::*;

#[test]
fn url_rewriters_preserve_css_and_srcset_token_boundaries() {
    assert_eq!(
        rewrite_css_urls_for_tests(".hero { background: url(asset.png) }", None),
        ".hero { background: url(asset.png) }"
    );
    assert_eq!(
        rewrite_css_urls_for_tests(
            "/* url(ignore.png) */ .hero { background: url( hero.png ); mask: url('mask.svg') } @import /* note */ \"theme.css\";",
            Some("https://example.test/assets/"),
        ),
        "/* url(ignore.png) */ .hero { background: url( https://example.test/assets/hero.png ); mask: url('https://example.test/assets/mask.svg') } @import /* note */ \"https://example.test/assets/theme.css\";"
    );
    assert_eq!(
        rewrite_srcset_for_tests(
            "  data:image/svg+xml,<svg></svg> 1x, hero.png 2x , icons/next.png 3x",
            Some("https://example.test/assets/"),
        ),
        "data:image/svg+xml,<svg></svg> 1x, https://example.test/assets/hero.png 2x, https://example.test/assets/icons/next.png 3x"
    );
}

#[test]
fn css_rewriter_rejects_non_advancing_internal_steps() {
    for fault in [
        CssProgressFault::Url,
        CssProgressFault::Whitespace,
        CssProgressFault::String,
        CssProgressFault::PlainString,
        CssProgressFault::EscapedString,
    ] {
        assert!(css_progress_rejection_for_tests(fault));
    }
}

#[test]
fn css_dispatcher_rejects_non_advancing_operation_results() {
    for fault in [
        CssDispatchFault::Comment,
        CssDispatchFault::Url,
        CssDispatchFault::Import,
        CssDispatchFault::Character,
    ] {
        assert!(css_dispatch_rejection_for_tests(fault));
    }
}

#[test]
fn css_progress_requires_an_in_bounds_character_boundary() {
    assert!(css_progress_is_valid_for_tests("a", 0, 1));
    assert!(!css_progress_is_valid_for_tests("a", 0, 0));
    assert!(!css_progress_is_valid_for_tests("a", 0, 2));
    assert!(!css_progress_is_valid_for_tests("é", 0, 1));
    assert!(css_ignorable_rejects_nonadvancing_comment_for_tests());
}

#[test]
fn srcset_rewriter_rejects_non_advancing_internal_steps() {
    for value in [
        " asset.png",
        "asset.png",
        "asset.png 2x",
        "asset.png, next.png",
    ] {
        assert!(srcset_rejects_non_advancing_progress_for_tests(value));
    }
    assert!(srcset_rejects_staged_non_advancing_progress_for_tests(
        false
    ));
    assert!(srcset_rejects_staged_non_advancing_progress_for_tests(true));
    assert!(srcset_progress_is_valid_for_tests(0, 1));
    assert!(!srcset_progress_is_valid_for_tests(1, 1));
    assert!(!srcset_progress_is_valid_for_tests(usize::MAX, usize::MAX));
}
