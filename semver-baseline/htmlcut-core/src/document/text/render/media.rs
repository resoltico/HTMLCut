use scraper::ElementRef;

use super::super::signals::{structural_signal_tokens, token_match_count};

pub(super) fn image_has_caption_context(element: &ElementRef<'_>) -> bool {
    let mut ancestor = element.parent();
    let mut depth = 0usize;

    while let Some(current) = ancestor {
        let Some(ancestor_element) = ElementRef::wrap(current) else {
            ancestor = current.parent();
            depth += 1;
            continue;
        };

        if matches!(ancestor_element.value().name(), "figure" | "figcaption") {
            return true;
        }

        for descendant in ancestor_element.descendants().filter_map(ElementRef::wrap) {
            if descendant.id() == element.id() {
                continue;
            }
            if descendant.value().name() == "figcaption" {
                return true;
            }
            let tokens = structural_signal_tokens(&descendant);
            if token_match_count(&tokens, &["caption"]) > 0 {
                return true;
            }
        }

        depth += 1;
        if depth >= 3 {
            break;
        }
        ancestor = current.parent();
    }

    false
}
