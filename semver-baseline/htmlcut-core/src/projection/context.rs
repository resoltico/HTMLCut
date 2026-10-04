// SPDX-License-Identifier: MPL-2.0
//! Bounded structural facts from the immutable original DOM, never outside payload.

use super::*;
use std::collections::HashMap;

pub(super) fn html(element: &scraper::node::Element) -> bool {
    let namespace: &str = element.name.ns.as_ref();
    namespace == "http://www.w3.org/1999/xhtml"
}

pub(super) fn inherited_pre(
    root: ElementRef<'_>,
    budget: &SelectorWorkBudget,
) -> Result<bool, ExtractionError> {
    for parent in root.ancestors() {
        crate::execution::charge(budget, 1)?;
        if parent
            .value()
            .as_element()
            .is_some_and(|e| e.name() == "pre" && html(e))
        {
            return Ok(true);
        }
    }
    Ok(false)
}

#[derive(Default)]
pub(super) struct ListContext {
    ordinals: HashMap<NodeId, HashMap<NodeId, i64>>,
}
impl ListContext {
    pub(super) fn ordinal(
        &mut self,
        item: ElementRef<'_>,
        budget: &SelectorWorkBudget,
    ) -> Result<Option<i64>, ExtractionError> {
        crate::execution::charge(budget, 1)?;
        let Some(parent) = item.parent().and_then(ElementRef::wrap) else {
            return Ok(None);
        };
        if parent.value().name() != "ol" {
            return Ok(None);
        }
        if let std::collections::hash_map::Entry::Vacant(entry) = self.ordinals.entry(parent.id()) {
            let mut children = Vec::new();
            for child in parent.children() {
                crate::execution::charge(budget, 1)?;
                if let Some(child) = ElementRef::wrap(child)
                    && child.value().name() == "li"
                {
                    children.push(child);
                }
            }
            let reversed = parent.attr("reversed").is_some();
            let start = integer(parent.attr("start"), budget)?.unwrap_or(if reversed {
                children.len() as i64
            } else {
                1
            });
            let mut next = Some(start);
            let mut ordinals = HashMap::new();
            for child in children {
                crate::execution::charge(budget, 1)?;
                let number = integer(child.attr("value"), budget)?
                    .or(next)
                    .ok_or_else(|| ExtractionError::limit("rendering"))?;
                ordinals.insert(child.id(), number);
                next = if reversed {
                    number.checked_sub(1)
                } else {
                    number.checked_add(1)
                };
            }
            entry.insert(ordinals);
        }
        Ok(self
            .ordinals
            .get(&parent.id())
            .expect("prepared original list")
            .get(&item.id())
            .copied())
    }
}

fn integer(
    value: Option<&str>,
    budget: &SelectorWorkBudget,
) -> Result<Option<i64>, ExtractionError> {
    if let Some(value) = value {
        crate::execution::charge(budget, value.len().div_ceil(64) + 1)?;
    }
    ordinal(value)
}

pub(super) fn ordinal(value: Option<&str>) -> Result<Option<i64>, ExtractionError> {
    let Some(value) = value else {
        return Ok(None);
    };
    let value = value.trim_start_matches(|c: char| c.is_ascii_whitespace());
    let sign = usize::from(value.starts_with('+') || value.starts_with('-'));
    let digits = value[sign..].bytes().take_while(u8::is_ascii_digit).count();
    if digits == 0 {
        return Ok(None);
    }
    value[..sign + digits]
        .parse()
        .map(Some)
        .map_err(|_| ExtractionError::limit("rendering"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_integer_attribute_still_consumes_one_metadata_unit() {
        let budget = SelectorWorkBudget::new(1);
        assert_eq!(integer(Some(""), &budget).unwrap(), None);
        assert_eq!(budget.remaining(), 0);
        assert!(!budget.exhausted());
        let error = integer(Some(""), &budget).unwrap_err();
        assert_eq!(error.code, ErrorCode::ResourceLimit);
        assert_eq!(budget.remaining(), 0);
    }

    #[test]
    fn document_root_and_non_list_members_have_no_ordered_ordinal() {
        let document = scraper::Html::parse_document("<ol><li>A</li><span>B</span></ol>");
        let budget = SelectorWorkBudget::new(1000);
        let mut context = ListContext::default();
        for css in ["html", "span"] {
            let element = document
                .select(&scraper::Selector::parse(css).unwrap())
                .next()
                .unwrap();
            assert_eq!(context.ordinal(element, &budget).unwrap(), None);
        }
    }
}
