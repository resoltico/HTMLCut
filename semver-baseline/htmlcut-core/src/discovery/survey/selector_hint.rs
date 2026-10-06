// SPDX-License-Identifier: MPL-2.0
//! CSS hints are emitted only after matching exactly the surveyed original nodes.

use scraper::{ElementRef, Html};
use selectors::work_budget::SelectorWorkBudget;

use super::{Candidate, class_signature};
use crate::ExtractionError;

pub(super) fn verified(
    document: &Html,
    group: &Candidate<'_>,
    members: &[ElementRef<'_>],
    budget: &SelectorWorkBudget,
) -> Result<Option<String>, ExtractionError> {
    let mut candidates = Vec::new();
    let tag = &group.key.tag;
    // Survey admission already bounds nonempty parsed HTML tags and class tokens;
    // the tokenizer replaces NUL before they become Candidate values.
    let escaped_tag = escaped_identifier(tag);
    let item = if let Some(classes) = &group.key.classes {
        let classes = classes
            .iter()
            .map(|class| escaped_identifier(class))
            .collect::<Vec<_>>();
        let specific = format!("{escaped_tag}.{}", classes.join("."));
        candidates.push(specific.clone());
        specific
    } else {
        escaped_tag.clone()
    };
    let mut id_anchor = None;
    let mut class_anchor = None;
    for (depth, ancestor) in std::iter::once(group.parent)
        .chain(group.parent.ancestors().filter_map(ElementRef::wrap))
        .take(8)
        .enumerate()
    {
        crate::execution::charge(budget, 1)?;
        let combinator = if depth == 0 { " > " } else { " " };
        if id_anchor.is_none()
            && let Some(id) = ancestor.attr("id")
            && let Some(id) = css_escape(id)
        {
            id_anchor = Some(format!("#{id}{combinator}{item}"));
        }
        if class_anchor.is_none()
            && let Some(tag) = css_escape(ancestor.value().name())
            && let Some(classes) = class_signature(ancestor, budget)?
            && let Some(class) = classes.iter().find_map(|class| css_escape(class))
        {
            class_anchor = Some(format!("{}.{}{combinator}{item}", tag, class));
        }
        if id_anchor.is_some() && class_anchor.is_some() {
            break;
        }
    }
    candidates.extend(id_anchor);
    candidates.extend(class_anchor);
    candidates.push(item);
    for css in candidates {
        let selector = crate::compilation::compile_selector(&css)?;
        let matches = crate::execution::matches(
            document,
            None,
            &selector,
            crate::ExecutionLimits::default().max_candidates,
            budget,
        )?;
        if matches.len() == members.len() {
            crate::execution::charge(budget, matches.len())?;
            if same_nodes(&matches, members) {
                return Ok(Some(css));
            }
        }
    }
    Ok(None)
}

fn same_nodes(found: &[ElementRef<'_>], members: &[ElementRef<'_>]) -> bool {
    found
        .iter()
        .zip(members)
        .all(|(found, member)| found.id() == member.id())
}

fn css_escape(value: &str) -> Option<String> {
    // CSS replaces NUL, so it cannot yield an exact hint for a NUL identifier.
    if value.is_empty() || value.len() > 128 || value.contains('\0') {
        return None;
    }
    Some(escaped_identifier(value))
}

fn escaped_identifier(value: &str) -> String {
    let mut escaped = String::new();
    cssparser::serialize_identifier(value, &mut escaped).expect("String formatting cannot fail");
    escaped
}

#[cfg(test)]
mod tests {
    use scraper::Html;
    use selectors::work_budget::SelectorWorkBudget;

    use super::{css_escape, same_nodes, verified};
    use crate::discovery::survey::{Candidate, GroupKey};

    #[test]
    fn selector_tokens_are_conservative_and_exact_identity_rejects_equal_counts() {
        for valid in ["li", "_item", "post-1"] {
            assert_eq!(css_escape(valid).as_deref(), Some(valid));
        }
        let oversized = "x".repeat(129);
        for invalid in ["", "\0", &oversized] {
            assert_eq!(css_escape(invalid), None);
        }
        let document = Html::parse_document("<p id=a>A</p><p id=b>B</p>");
        let nodes = document
            .tree
            .nodes()
            .filter_map(scraper::ElementRef::wrap)
            .filter(|element| element.value().name() == "p")
            .collect::<Vec<_>>();
        assert!(same_nodes(&nodes, &nodes));
        assert!(!same_nodes(&nodes, &[nodes[1], nodes[0]]));
    }

    #[test]
    fn a_same_count_wrong_order_never_produces_a_hint() {
        let document = Html::parse_document("<div><p>A</p><p>B</p></div>");
        let elements = document
            .tree
            .nodes()
            .filter_map(scraper::ElementRef::wrap)
            .collect::<Vec<_>>();
        let parent = elements
            .iter()
            .find(|element| element.value().name() == "div")
            .unwrap();
        let members = elements
            .iter()
            .copied()
            .filter(|element| element.value().name() == "p")
            .collect::<Vec<_>>();
        let group = Candidate {
            parent: *parent,
            key: GroupKey {
                tag: "p".into(),
                classes: None,
            },
            count: 2,
            first_order: 0,
        };
        assert_eq!(
            verified(
                &document,
                &group,
                &[members[1], members[0]],
                &SelectorWorkBudget::new(10_000)
            )
            .unwrap(),
            None
        );
    }
}
