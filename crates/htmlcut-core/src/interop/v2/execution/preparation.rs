//! Single-parse construction of opaque prepared documents.

use ego_tree::NodeId;
use scraper::Node;

#[cfg(test)]
use std::cell::Cell;

use crate::document::{parse_document_node, resolve_document_base_url};
use crate::interop::v2::stable_json::digest_stable_json;

use super::super::{
    HtmlInput, PreparationError, PreparationErrorCode, PreparationLimits, PreparedDocument,
};

#[cfg(test)]
thread_local! {
    static PREPARATION_PARSE_COUNT: Cell<usize> = const { Cell::new(0) };
}

#[cfg(test)]
pub(crate) fn reset_preparation_parse_count_for_tests() {
    PREPARATION_PARSE_COUNT.with(|count| count.set(0));
}

#[cfg(test)]
pub(crate) fn preparation_parse_count_for_tests() -> usize {
    PREPARATION_PARSE_COUNT.with(Cell::get)
}

/// Prepares one exact HTML input for repeated plan execution.
pub fn prepare_document(
    input: HtmlInput,
    limits: PreparationLimits,
) -> Result<PreparedDocument, Box<PreparationError>> {
    let input_digest_sha256 = preparation_digest(&input, &limits);
    if u64::try_from(input.html.len()).expect("usize must fit u64 on supported HTMLCut targets")
        > u64::from(limits.max_input_bytes.get())
    {
        return Err(Box::new(PreparationError::new(
            PreparationErrorCode::InputTooLarge,
            input_digest_sha256,
            "HTML input exceeds the configured preparation byte limit.",
        )));
    }
    if limits.validate().is_err() {
        return Err(Box::new(PreparationError::new(
            PreparationErrorCode::InternalInvariantViolation,
            input_digest_sha256,
            "HTML preparation limits are outside the supported range.",
        )));
    }
    #[cfg(test)]
    PREPARATION_PARSE_COUNT.with(|count| count.set(count.get().saturating_add(1)));
    let document = parse_document_node(&input.html);
    let complexity = document_complexity(&document);
    if complexity.element_count > u64::from(limits.max_elements.get()) {
        return Err(Box::new(PreparationError::new(
            PreparationErrorCode::DocumentTooComplex,
            input_digest_sha256,
            "Parsed HTML exceeds the configured element limit.",
        )));
    }
    if complexity.max_depth > u64::from(limits.max_dom_depth.get()) {
        return Err(Box::new(PreparationError::new(
            PreparationErrorCode::DocumentTooDeep,
            input_digest_sha256,
            "Parsed HTML exceeds the configured DOM depth limit.",
        )));
    }
    let effective_base_url = resolve_document_base_url(
        &document,
        input
            .input_base_url
            .as_ref()
            .map(ToString::to_string)
            .as_deref(),
    );
    Ok(PreparedDocument::from_prepared_parts(
        input,
        input_digest_sha256,
        document,
        complexity.element_ids,
        effective_base_url,
    ))
}

struct DocumentComplexity {
    element_count: u64,
    max_depth: u64,
    element_ids: Vec<NodeId>,
}

fn document_complexity(document: &scraper::Html) -> DocumentComplexity {
    let mut stack = vec![(document.tree.root(), 0_u64)];
    let mut element_count = 0_u64;
    let mut max_depth = 0_u64;
    let mut element_ids = Vec::new();
    while let Some((node, depth)) = stack.pop() {
        if matches!(node.value(), Node::Element(_)) {
            element_count = element_count.saturating_add(1);
            max_depth = max_depth.max(depth);
            element_ids.push(node.id());
        }
        let children = node.children().collect::<Vec<_>>();
        for child in children.into_iter().rev() {
            stack.push((child, depth.saturating_add(1)));
        }
    }
    DocumentComplexity {
        element_count,
        max_depth,
        element_ids,
    }
}

fn preparation_digest(input: &HtmlInput, limits: &PreparationLimits) -> String {
    digest_stable_json(&(input, limits))
        .expect("closed preparation input and limits must serialize to stable JSON")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::num::NonZeroU32;

    fn non_zero_limit(value: u64) -> NonZeroU32 {
        NonZeroU32::new(u32::try_from(value).expect("test limit must fit the public width"))
            .expect("prepared test documents have non-zero complexity")
    }

    fn html_at_document_depth(depth: u32) -> String {
        // `parse_document_node` supplies `html` and `body` at depths one and two. The generated
        // `div` chain therefore reaches the requested DOM depth without relying on recursive
        // test construction or traversal.
        let nesting = usize::try_from(
            depth
                .checked_sub(2)
                .expect("document depth must exceed body"),
        )
        .expect("published depth limit fits usize");
        format!(
            "{}leaf{}",
            "<div>".repeat(nesting),
            "</div>".repeat(nesting)
        )
    }

    #[test]
    fn preparation_accepts_input_at_the_exact_byte_limit() {
        let html = "x".repeat(64);
        let limits = PreparationLimits::new(
            non_zero_limit(html.len() as u64),
            NonZeroU32::new(32).expect("non-zero"),
            NonZeroU32::new(32).expect("non-zero"),
        )
        .expect("limits");

        prepare_document(HtmlInput::new("exact-input", html).expect("input"), limits)
            .expect("input at the configured byte limit");
    }

    #[test]
    fn preparation_accepts_document_at_the_exact_element_limit() {
        let html = "<main><article><p>One</p></article></main>";
        let parsed = parse_document_node(html);
        let complexity = document_complexity(&parsed);
        let limits = PreparationLimits::new(
            NonZeroU32::new(1_024).expect("non-zero"),
            non_zero_limit(complexity.element_count),
            non_zero_limit(complexity.max_depth),
        )
        .expect("limits");

        prepare_document(
            HtmlInput::new("exact-elements", html).expect("input"),
            limits,
        )
        .expect("document at the configured element limit");
    }

    #[test]
    fn preparation_accepts_document_at_the_exact_depth_limit() {
        let html = "<main><article><section><p>One</p></section></article></main>";
        let parsed = parse_document_node(html);
        let complexity = document_complexity(&parsed);
        let limits = PreparationLimits::new(
            NonZeroU32::new(1_024).expect("non-zero"),
            non_zero_limit(complexity.element_count),
            non_zero_limit(complexity.max_depth),
        )
        .expect("limits");

        prepare_document(HtmlInput::new("exact-depth", html).expect("input"), limits)
            .expect("document at the configured depth limit");
    }

    #[test]
    fn preparation_supports_the_default_and_hard_dom_depths_iteratively() {
        let default_depth = PreparationLimits::default().max_dom_depth.get();
        let hard_depth = 4_096;
        let hard_limits = PreparationLimits::new(
            NonZeroU32::new(50 * 1024 * 1024).expect("non-zero"),
            NonZeroU32::new(1_000_000).expect("non-zero"),
            NonZeroU32::new(hard_depth).expect("non-zero"),
        )
        .expect("published hard limits");

        for (depth, limits) in [
            (default_depth, PreparationLimits::default()),
            (hard_depth, hard_limits),
        ] {
            let html = html_at_document_depth(depth);
            let parsed = parse_document_node(&html);
            assert_eq!(
                document_complexity(&parsed).max_depth,
                u64::from(depth),
                "fixture must reach the published depth exactly"
            );
            prepare_document(
                HtmlInput::new("exact-published-depth", html).expect("input"),
                limits,
            )
            .expect("document at the published depth limit");
        }
    }

    #[test]
    fn preparation_refuses_one_level_beyond_the_hard_dom_depth_without_recursing() {
        let hard_depth = 4_096;
        let html = html_at_document_depth(hard_depth + 1);
        let parsed = parse_document_node(&html);
        assert_eq!(
            document_complexity(&parsed).max_depth,
            u64::from(hard_depth + 1)
        );
        let limits = PreparationLimits::new(
            NonZeroU32::new(50 * 1024 * 1024).expect("non-zero"),
            NonZeroU32::new(1_000_000).expect("non-zero"),
            NonZeroU32::new(hard_depth).expect("non-zero"),
        )
        .expect("published hard limits");

        let error = prepare_document(
            HtmlInput::new("beyond-hard-depth", html).expect("input"),
            limits,
        )
        .expect_err("document beyond the hard depth limit");
        assert_eq!(error.error_code, PreparationErrorCode::DocumentTooDeep);
    }
}
