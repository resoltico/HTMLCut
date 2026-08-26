use std::cmp::Ordering;
use std::collections::{BTreeMap, BTreeSet};

use scraper::{ElementRef, Html, Node, Selector};

use crate::contracts::{
    ContentCandidateInspection, DocumentInspection, HeadingInspection, LinkInspection,
    WhitespaceMode,
};
use crate::document::{
    build_node_path, document_base_href, element_has_utility_chrome_ancestor,
    element_looks_like_utility_chrome, extract_document_title, extract_heading_text, first_body,
    heading_level, href_is_meaningful_destination, render_html_as_text, resolve_url, select_first,
    serialize_children, structural_signal_tokens, summarize_counts, token_match_count,
};

const POSITIVE_CONTENT_TOKENS: [&str; 11] = [
    "article", "body", "content", "entry", "guide", "help", "main", "page", "post", "primary",
    "story",
];
const NEGATIVE_CONTENT_TOKENS: [&str; 17] = [
    "ad",
    "banner",
    "breadcrumb",
    "comment",
    "footer",
    "header",
    "language",
    "menu",
    "nav",
    "newsletter",
    "promo",
    "related",
    "share",
    "sidebar",
    "social",
    "toc",
    "toolbar",
];
const GENERIC_SELECTOR_CLASSES: [&str; 11] = [
    "article",
    "body",
    "container",
    "content",
    "inner",
    "layout",
    "main",
    "module",
    "outer",
    "page",
    "wrapper",
];
const LOW_SIGNAL_LINK_PATH_TOKENS: [&str; 15] = [
    "article-share",
    "article-tags",
    "breadcrumb",
    "comment",
    "comments",
    "footer",
    "menu",
    "nav",
    "newsletter",
    "promo",
    "related",
    "report",
    "share",
    "social",
    "toolbar",
];
const LOW_SIGNAL_LINK_HREF_FRAGMENTS: [&str; 11] = [
    "/fair-use/",
    "/policy",
    "/privacy",
    "/report/",
    "/rss",
    "/subscribe",
    "/tags/",
    "/terms",
    "privacy-policy",
    "terms-of-use",
    "terms-and-conditions",
];
const LOW_SIGNAL_LINK_TEXT_PHRASES: [&str; 8] = [
    "add as a preferred source",
    "follow us",
    "how it works",
    "preferred source",
    "privacy policy",
    "report a problem",
    "terms of use",
    "terms apply",
];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum CandidatePreference {
    Extraction,
    Reading,
}

#[derive(Clone)]
struct RankedContentCandidate {
    score: i32,
    inspection: ContentCandidateInspection,
    paragraph_count: usize,
    primary_heading_level: Option<u8>,
    primary_heading_count: usize,
    primary_heading_depth: Option<usize>,
    utility_descendant_count: usize,
}

struct ContentCandidateScoreInputs<'a> {
    tag_name: &'a str,
    has_main_role: bool,
    has_article_body_itemprop: bool,
    text_char_count: usize,
    heading_count: usize,
    link_count: usize,
    paragraph_count: usize,
    positive_signal_count: usize,
    negative_signal_count: usize,
    primary_heading_level: Option<u8>,
    primary_heading_count: usize,
    primary_heading_depth: Option<usize>,
    utility_descendant_count: usize,
    uses_exact_path_selector: bool,
}

#[derive(Clone, Copy)]
struct HeadingShellCandidate<'a> {
    text_char_count: usize,
    heading_count: usize,
    link_count: usize,
    selector: &'a str,
}

pub(crate) fn build_document_inspection(
    document: &Html,
    effective_base_url: Option<&str>,
    sample_limit: usize,
) -> DocumentInspection {
    let root_tag = select_first(document, "html")
        .map(|html| html.value().name().to_owned())
        .unwrap_or_else(|| "html".to_owned());
    let body_text_char_count = text::normalized_body_text_char_count(document);
    let mut tag_counts = BTreeMap::<String, usize>::new();
    let mut class_counts = BTreeMap::<String, usize>::new();
    let mut link_count = 0usize;
    let mut image_count = 0usize;
    let mut form_count = 0usize;
    let mut table_count = 0usize;
    let mut script_count = 0usize;
    let mut style_count = 0usize;
    let mut element_count = 0usize;

    for node_ref in document.tree.nodes() {
        let Some(element) = ElementRef::wrap(node_ref) else {
            continue;
        };

        let tag_name = element.value().name().to_owned();
        *tag_counts.entry(tag_name.clone()).or_insert(0) += 1;
        element_count += 1;

        match tag_name.as_str() {
            "a" => link_count += 1,
            "img" => image_count += 1,
            "form" => form_count += 1,
            "table" => table_count += 1,
            "script" => script_count += 1,
            "style" => style_count += 1,
            _ => {}
        }

        if let Some(classes) = element.value().attr("class") {
            for class_name in classes.split_whitespace() {
                *class_counts.entry(class_name.to_owned()).or_insert(0) += 1;
            }
        }
    }

    let mut extraction_candidates = candidates::build::build_ranked_content_candidates_for(
        document,
        sample_limit,
        CandidatePreference::Extraction,
    );
    let reading_candidates = candidates::build::build_ranked_content_candidates_for(
        document,
        sample_limit,
        CandidatePreference::Reading,
    );
    if extraction_candidates.is_empty() {
        extraction_candidates = reading_candidates.clone();
    } else {
        candidates::promotion::promote_precise_reading_descendant_candidate(
            &mut extraction_candidates,
            &reading_candidates,
        );
        candidates::promotion::promote_title_bearing_reading_ancestor_candidate(
            &mut extraction_candidates,
            &reading_candidates,
        );
        candidates::promotion::promote_cleaner_reading_descendant_candidate(
            &mut extraction_candidates,
            &reading_candidates,
        );
    }
    let content_candidate_paths = reading_candidates
        .iter()
        .map(|candidate| candidate.inspection.path.clone())
        .collect::<Vec<_>>();
    let mut headings =
        samples::build_heading_samples(document, sample_limit, &content_candidate_paths);
    candidates::scoring::prepend_document_title_heading_if_missing(
        document,
        sample_limit,
        &mut headings,
    );
    let links = samples::build_link_samples(
        document,
        effective_base_url,
        sample_limit,
        &content_candidate_paths,
    );

    DocumentInspection {
        title: extract_document_title(document),
        root_tag,
        element_count,
        text_char_count: body_text_char_count,
        link_count,
        image_count,
        form_count,
        table_count,
        script_count,
        style_count,
        document_base_href: document_base_href(document),
        top_tags: summarize_counts(tag_counts, sample_limit),
        top_classes: summarize_counts(class_counts, sample_limit),
        extraction_candidates: extraction_candidates
            .into_iter()
            .map(|candidate| candidate.inspection)
            .collect(),
        reading_candidates: reading_candidates
            .into_iter()
            .map(|candidate| candidate.inspection)
            .collect(),
        headings,
        links,
    }
}

#[cfg(test)]
fn build_content_candidates(
    document: &Html,
    sample_limit: usize,
) -> Vec<ContentCandidateInspection> {
    candidates::build::build_ranked_content_candidates_for(
        document,
        sample_limit,
        CandidatePreference::Reading,
    )
    .into_iter()
    .map(|candidate| candidate.inspection)
    .collect()
}

#[cfg(test)]
pub(crate) fn opaque_div_has_long_form_shape_for_tests(
    text_char_count: usize,
    prose_paragraph_count: usize,
    meaningful_h1_count: usize,
) -> bool {
    candidates::build::opaque_div_has_long_form_shape_for_tests(
        text_char_count,
        prose_paragraph_count,
        meaningful_h1_count,
    )
}

/// Test-only inputs for the content-candidate ranking policy.
#[cfg(test)]
#[derive(Clone, Debug)]
pub(crate) struct ContentCandidateTestInput {
    pub(crate) tag_name: &'static str,
    pub(crate) selector: &'static str,
    pub(crate) path: &'static str,
    pub(crate) text_char_count: usize,
    pub(crate) heading_count: usize,
    pub(crate) link_count: usize,
    pub(crate) paragraph_count: usize,
    pub(crate) primary_heading_level: Option<u8>,
    pub(crate) primary_heading_count: usize,
    pub(crate) primary_heading_depth: Option<usize>,
    pub(crate) utility_descendant_count: usize,
    pub(crate) has_main_role: bool,
    pub(crate) has_article_body_itemprop: bool,
    pub(crate) positive_signal_count: usize,
    pub(crate) negative_signal_count: usize,
    pub(crate) uses_exact_path_selector: bool,
}

#[cfg(test)]
impl Default for ContentCandidateTestInput {
    fn default() -> Self {
        Self {
            tag_name: "div",
            selector: "#candidate",
            path: "html > body > div#candidate",
            text_char_count: 8_000,
            heading_count: 0,
            link_count: 0,
            paragraph_count: 0,
            primary_heading_level: None,
            primary_heading_count: 0,
            primary_heading_depth: None,
            utility_descendant_count: 0,
            has_main_role: false,
            has_article_body_itemprop: false,
            positive_signal_count: 0,
            negative_signal_count: 0,
            uses_exact_path_selector: false,
        }
    }
}

/// Test-only preference selector for ranking-policy scenarios.
#[cfg(test)]
#[derive(Clone, Copy, Debug)]
pub(crate) enum ContentCandidateTestPreference {
    Extraction,
    Reading,
}

#[cfg(test)]
fn content_candidate_score_inputs_for_tests(
    input: &ContentCandidateTestInput,
) -> ContentCandidateScoreInputs<'_> {
    ContentCandidateScoreInputs {
        tag_name: input.tag_name,
        has_main_role: input.has_main_role,
        has_article_body_itemprop: input.has_article_body_itemprop,
        text_char_count: input.text_char_count,
        heading_count: input.heading_count,
        link_count: input.link_count,
        paragraph_count: input.paragraph_count,
        positive_signal_count: input.positive_signal_count,
        negative_signal_count: input.negative_signal_count,
        primary_heading_level: input.primary_heading_level,
        primary_heading_count: input.primary_heading_count,
        primary_heading_depth: input.primary_heading_depth,
        utility_descendant_count: input.utility_descendant_count,
        uses_exact_path_selector: input.uses_exact_path_selector,
    }
}

/// Returns extraction and reading scores for a declared ranking-policy fixture.
#[cfg(test)]
pub(crate) fn content_candidate_scores_for_tests(input: &ContentCandidateTestInput) -> (i32, i32) {
    let inputs = content_candidate_score_inputs_for_tests(input);
    (
        candidates::scoring::content_candidate_score_for(&inputs, CandidatePreference::Extraction),
        candidates::scoring::content_candidate_score_for(&inputs, CandidatePreference::Reading),
    )
}

/// Returns the extraction-ranking score, including its bounded reading-score contribution.
#[cfg(test)]
pub(crate) fn extraction_candidate_score_for_tests(input: &ContentCandidateTestInput) -> i32 {
    let inputs = content_candidate_score_inputs_for_tests(input);
    candidates::build::ranked_content_candidate_score_for(&inputs, CandidatePreference::Extraction)
}

/// Returns whether extraction should prefer a similarly complete, utility-light descendant.
#[cfg(test)]
pub(crate) fn extraction_prefers_utility_light_descendant_for_tests(
    outer: &ContentCandidateTestInput,
    inner: &ContentCandidateTestInput,
) -> bool {
    candidates::build::extraction_prefers_utility_light_descendant(
        CandidatePreference::Extraction,
        outer.text_char_count,
        outer.heading_count,
        outer.link_count,
        outer.paragraph_count,
        outer.utility_descendant_count,
        inner.text_char_count,
        inner.heading_count,
        inner.link_count,
        inner.paragraph_count,
        inner.utility_descendant_count,
    )
}

/// Returns whether extraction should prefer a descendant with substantially less heading and link chrome.
#[cfg(test)]
pub(crate) fn extraction_prefers_heading_and_link_light_descendant_for_tests(
    outer: &ContentCandidateTestInput,
    inner: &ContentCandidateTestInput,
) -> bool {
    candidates::build::extraction_prefers_heading_and_link_light_descendant(
        CandidatePreference::Extraction,
        outer.text_char_count,
        outer.heading_count,
        outer.link_count,
        inner.text_char_count,
        inner.heading_count,
        inner.link_count,
    )
}

/// Returns whether extraction should prefer a near-complete descendant with fewer links.
#[cfg(test)]
pub(crate) fn extraction_prefers_near_complete_link_light_descendant_for_tests(
    outer: &ContentCandidateTestInput,
    inner: &ContentCandidateTestInput,
) -> bool {
    candidates::build::extraction_prefers_near_complete_link_light_descendant(
        CandidatePreference::Extraction,
        outer.text_char_count,
        outer.heading_count,
        outer.link_count,
        inner.text_char_count,
        inner.heading_count,
        inner.link_count,
    )
}

/// Returns whether a near-complete descendant removes a very large link burden.
#[cfg(test)]
pub(crate) fn prefers_heavy_link_descendant_for_tests(
    outer: &ContentCandidateTestInput,
    inner: &ContentCandidateTestInput,
) -> bool {
    candidates::build::prefers_heavy_link_descendant(
        outer.text_char_count,
        outer.link_count,
        inner.text_char_count,
        inner.link_count,
    )
}

/// Returns whether extraction should prefer a stable, link-light near-complete descendant.
#[cfg(test)]
pub(crate) fn extraction_prefers_stable_link_light_descendant_for_tests(
    outer: &ContentCandidateTestInput,
    inner: &ContentCandidateTestInput,
    drops_outer_title_signal: bool,
) -> bool {
    candidates::build::extraction_prefers_stable_link_light_descendant(
        CandidatePreference::Extraction,
        drops_outer_title_signal,
        outer.text_char_count,
        outer.heading_count,
        outer.link_count,
        outer.selector,
        inner.text_char_count,
        inner.heading_count,
        inner.link_count,
        inner.selector,
    )
}

/// Returns whether extraction should retain an outer candidate that is substantially larger.
#[cfg(test)]
pub(crate) fn extraction_preserves_large_outer_candidate_for_tests(
    outer: &ContentCandidateTestInput,
    inner: &ContentCandidateTestInput,
) -> bool {
    candidates::build::extraction_preserves_large_outer_candidate(
        CandidatePreference::Extraction,
        outer.text_char_count,
        outer.heading_count,
        outer.paragraph_count,
        inner.text_char_count,
        inner.heading_count,
        inner.paragraph_count,
    )
}

/// Returns whether a similarly complete descendant removes substantial utility chrome.
#[cfg(test)]
pub(crate) fn prefers_utility_light_descendant_for_tests(
    outer: &ContentCandidateTestInput,
    inner: &ContentCandidateTestInput,
) -> bool {
    candidates::build::prefers_utility_light_descendant(
        outer.text_char_count,
        outer.heading_count,
        outer.link_count,
        outer.paragraph_count,
        outer.utility_descendant_count,
        inner.text_char_count,
        inner.heading_count,
        inner.link_count,
        inner.paragraph_count,
        inner.utility_descendant_count,
    )
}

/// Returns whether a title-bearing outer candidate should be retained for extraction.
#[cfg(test)]
pub(crate) fn preserves_title_bearing_outer_candidate_for_tests(
    outer: &ContentCandidateTestInput,
    inner: &ContentCandidateTestInput,
    drops_outer_title_signal: bool,
) -> bool {
    candidates::build::preserves_title_bearing_outer_candidate(
        outer.paragraph_count,
        drops_outer_title_signal,
        outer.text_char_count,
        outer.link_count,
        inner.text_char_count,
        inner.link_count,
    )
}

/// Returns whether the `#scope` candidate is excluded as utility chrome.
#[cfg(test)]
pub(crate) fn content_candidate_is_excluded_for_utility_chrome_for_tests(markup: &str) -> bool {
    let document = Html::parse_document(markup);
    let scope = select_first(&document, "#scope").expect("#scope fixture element");
    candidates::build::should_skip_content_candidate(&scope)
}

/// Returns whether a declared candidate has enough readable content for recommendation.
#[cfg(test)]
pub(crate) fn content_candidate_has_readable_density_for_tests(
    tag_name: &str,
    text_char_count: usize,
    heading_count: usize,
    link_count: usize,
    body_block_count: usize,
    prose_paragraph_count: usize,
) -> bool {
    candidates::scoring::candidate_has_readable_density(
        tag_name,
        text_char_count,
        heading_count,
        link_count,
        body_block_count,
        prose_paragraph_count,
    )
}

/// Returns whether the `#scope` element has the structural shape of prose.
#[cfg(test)]
pub(crate) fn content_candidate_has_narrative_section_shape_for_tests(markup: &str) -> bool {
    let document = Html::parse_document(markup);
    let scope = select_first(&document, "#scope").expect("#scope fixture element");
    candidates::scoring::element_has_narrative_section_shape(&scope)
}

#[cfg(test)]
fn ranked_content_candidate_for_tests(input: &ContentCandidateTestInput) -> RankedContentCandidate {
    RankedContentCandidate {
        score: 0,
        inspection: ContentCandidateInspection {
            selector: input.selector.to_owned(),
            path: input.path.to_owned(),
            tag_name: input.tag_name.to_owned(),
            text_char_count: input.text_char_count,
            heading_count: input.heading_count,
            link_count: input.link_count,
        },
        paragraph_count: input.paragraph_count,
        primary_heading_level: input.primary_heading_level,
        primary_heading_count: input.primary_heading_count,
        primary_heading_depth: input.primary_heading_depth,
        utility_descendant_count: input.utility_descendant_count,
    }
}

/// Returns the outer and inner score deltas from one nested-candidate policy comparison.
#[cfg(test)]
pub(crate) fn nested_content_candidate_bias_deltas_for_tests(
    outer: &ContentCandidateTestInput,
    inner: &ContentCandidateTestInput,
    preference: ContentCandidateTestPreference,
) -> (i32, i32) {
    let preference = match preference {
        ContentCandidateTestPreference::Extraction => CandidatePreference::Extraction,
        ContentCandidateTestPreference::Reading => CandidatePreference::Reading,
    };
    let mut candidates = [
        ranked_content_candidate_for_tests(outer),
        ranked_content_candidate_for_tests(inner),
    ];
    candidates::build::apply_nested_content_candidate_bias_for(&mut candidates, preference);
    (candidates[0].score, candidates[1].score)
}

mod candidates;
mod samples;
mod selectors;
mod text;

#[cfg(test)]
mod tests;
