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
    if !css_ident(tag) {
        return Ok(None);
    }
    let item = match &group.key.classes {
        Some(classes) if classes.iter().all(|class| css_ident(class)) => {
            let specific = format!("{tag}.{}", classes.join("."));
            candidates.push(specific.clone());
            specific
        }
        _ => tag.clone(),
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
            && css_ident(id)
        {
            id_anchor = Some(format!("#{id}{combinator}{item}"));
        }
        if class_anchor.is_none()
            && css_ident(ancestor.value().name())
            && let Some(classes) = class_signature(ancestor, budget)?
            && let Some(class) = classes.iter().find(|class| css_ident(class))
        {
            class_anchor = Some(format!(
                "{}.{}{combinator}{item}",
                ancestor.value().name(),
                class
            ));
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

fn css_ident(value: &str) -> bool {
    let mut bytes = value.bytes();
    bytes
        .next()
        .is_some_and(|first| first.is_ascii_alphabetic() || first == b'_')
        && bytes.all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-'))
}

#[cfg(test)]
mod tests {
    use scraper::Html;
    use selectors::work_budget::SelectorWorkBudget;

    use super::{css_ident, same_nodes, verified};
    use crate::discovery::outline::{Candidate, GroupKey};

    #[test]
    fn selector_tokens_are_conservative_and_exact_identity_rejects_equal_counts() {
        for valid in ["li", "_item", "post-1"] {
            assert!(css_ident(valid));
        }
        for invalid in ["", "1item", "has:colon", "nonasciié"] {
            assert!(!css_ident(invalid));
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
