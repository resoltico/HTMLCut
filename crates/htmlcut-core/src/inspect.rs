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
    let body_text_char_count = normalized_body_text_char_count(document);
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

    let mut extraction_candidates = build_ranked_content_candidates_for(
        document,
        sample_limit,
        CandidatePreference::Extraction,
    );
    let reading_candidates =
        build_ranked_content_candidates_for(document, sample_limit, CandidatePreference::Reading);
    if extraction_candidates.is_empty() {
        extraction_candidates = reading_candidates.clone();
    } else {
        promote_precise_reading_descendant_candidate(
            &mut extraction_candidates,
            &reading_candidates,
        );
        promote_title_bearing_reading_ancestor_candidate(
            &mut extraction_candidates,
            &reading_candidates,
        );
        promote_cleaner_reading_descendant_candidate(
            &mut extraction_candidates,
            &reading_candidates,
        );
    }
    let content_candidate_paths = reading_candidates
        .iter()
        .map(|candidate| candidate.inspection.path.clone())
        .collect::<Vec<_>>();
    let mut headings = build_heading_samples(document, sample_limit, &content_candidate_paths);
    prepend_document_title_heading_if_missing(document, sample_limit, &mut headings);
    let links = build_link_samples(
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
    build_ranked_content_candidates_for(document, sample_limit, CandidatePreference::Reading)
        .into_iter()
        .map(|candidate| candidate.inspection)
        .collect()
}

fn build_ranked_content_candidates_for(
    document: &Html,
    sample_limit: usize,
    preference: CandidatePreference,
) -> Vec<RankedContentCandidate> {
    if sample_limit == 0 {
        return Vec::new();
    }

    let heading_selector = Selector::parse("h1, h2, h3, h4, h5, h6").expect("heading selector");
    let primary_heading_selector = Selector::parse("h1, h2").expect("primary heading selector");
    let link_selector = Selector::parse("a").expect("link selector");
    let paragraph_selector = Selector::parse("p").expect("paragraph selector");
    let list_item_selector = Selector::parse("li").expect("list item selector");
    let mut candidates = Vec::<RankedContentCandidate>::new();

    for node_ref in document.tree.nodes() {
        let Some(element) = ElementRef::wrap(node_ref) else {
            continue;
        };
        if element_looks_like_utility_chrome(&element)
            || element_has_utility_chrome_ancestor(&element)
        {
            continue;
        }
        let signal_tokens = structural_signal_tokens(&element);
        let positive_signal_count = token_match_count(&signal_tokens, &POSITIVE_CONTENT_TOKENS);
        if !is_content_candidate_container(&element, positive_signal_count) {
            continue;
        }

        let negative_signal_count = token_match_count(&signal_tokens, &NEGATIVE_CONTENT_TOKENS);
        let text_char_count =
            render_html_as_text(&serialize_children(&element), WhitespaceMode::Normalize)
                .chars()
                .count();
        if text_char_count == 0 {
            continue;
        }

        let heading_count = count_meaningful_headings(&element, &heading_selector);
        let link_count = count_meaningful_links(&element, &link_selector);
        let prose_paragraph_count = element.select(&paragraph_selector).count();
        let paragraph_count = narrative_block_count(
            prose_paragraph_count,
            element.select(&list_item_selector).count(),
        );
        if !candidate_has_readable_density(
            element.value().name(),
            text_char_count,
            heading_count,
            link_count,
            paragraph_count,
            prose_paragraph_count,
        ) {
            continue;
        }
        let primary_heading = first_meaningful_heading(&element, &primary_heading_selector);
        let primary_heading_level =
            primary_heading.and_then(|heading| heading_level(heading.value().name()));
        let primary_heading_count = count_meaningful_headings(&element, &primary_heading_selector);
        let primary_heading_depth = primary_heading
            .as_ref()
            .and_then(|heading| descendant_element_depth(&element, heading));
        let path = build_node_path(&element);
        let selector = recommend_content_selector(document, &element, &path);
        let utility_descendant_count = count_utility_descendant_roots(&element);
        let score_inputs = ContentCandidateScoreInputs {
            tag_name: element.value().name(),
            has_main_role: element
                .value()
                .attr("role")
                .is_some_and(|role| role.eq_ignore_ascii_case("main")),
            has_article_body_itemprop: element
                .value()
                .attr("itemprop")
                .is_some_and(|value| value.eq_ignore_ascii_case("articleBody")),
            text_char_count,
            heading_count,
            link_count,
            paragraph_count,
            positive_signal_count,
            negative_signal_count,
            primary_heading_level,
            primary_heading_count,
            primary_heading_depth,
            utility_descendant_count,
            uses_exact_path_selector: selector == path,
        };
        let score = match preference {
            CandidatePreference::Extraction => {
                let extraction_score =
                    content_candidate_score_for(&score_inputs, CandidatePreference::Extraction);
                let reading_score =
                    content_candidate_score_for(&score_inputs, CandidatePreference::Reading);
                extraction_score + (reading_score.max(0) / 3)
            }
            CandidatePreference::Reading => {
                content_candidate_score_for(&score_inputs, CandidatePreference::Reading)
            }
        };
        if score <= 0 {
            continue;
        }

        candidates.push(RankedContentCandidate {
            score,
            inspection: ContentCandidateInspection {
                selector,
                path,
                tag_name: element.value().name().to_owned(),
                text_char_count,
                heading_count,
                link_count,
            },
            paragraph_count,
            primary_heading_level,
            primary_heading_count,
            primary_heading_depth,
            utility_descendant_count,
        });
    }

    apply_nested_content_candidate_bias_for(&mut candidates, preference);

    candidates.sort_by(|left, right| compare_content_candidates_for(left, right, preference));

    candidates.into_iter().take(sample_limit).collect()
}

#[cfg(test)]
fn apply_nested_content_candidate_bias(candidates: &mut [RankedContentCandidate]) {
    apply_nested_content_candidate_bias_for(candidates, CandidatePreference::Reading);
}

fn apply_nested_content_candidate_bias_for(
    candidates: &mut [RankedContentCandidate],
    preference: CandidatePreference,
) {
    for outer_index in 0..candidates.len() {
        for inner_index in 0..candidates.len() {
            if outer_index == inner_index {
                continue;
            }

            let outer_path = candidates[outer_index].inspection.path.clone();
            let outer_selector = candidates[outer_index].inspection.selector.clone();
            let outer_text_char_count = candidates[outer_index].inspection.text_char_count;
            let outer_heading_count = candidates[outer_index].inspection.heading_count;
            let outer_link_count = candidates[outer_index].inspection.link_count;
            let outer_primary_heading_level = candidates[outer_index].primary_heading_level;
            let outer_primary_heading_count = candidates[outer_index].primary_heading_count;
            let outer_primary_heading_depth = candidates[outer_index].primary_heading_depth;
            let outer_utility_descendant_count = candidates[outer_index].utility_descendant_count;
            let outer_paragraph_count = candidates[outer_index].paragraph_count;

            let inner_path = candidates[inner_index].inspection.path.clone();
            let inner_selector = candidates[inner_index].inspection.selector.clone();
            let inner_text_char_count = candidates[inner_index].inspection.text_char_count;
            let inner_heading_count = candidates[inner_index].inspection.heading_count;
            let inner_link_count = candidates[inner_index].inspection.link_count;
            let inner_primary_heading_level = candidates[inner_index].primary_heading_level;
            let inner_primary_heading_count = candidates[inner_index].primary_heading_count;
            let inner_primary_heading_depth = candidates[inner_index].primary_heading_depth;
            let inner_utility_descendant_count = candidates[inner_index].utility_descendant_count;
            let inner_paragraph_count = candidates[inner_index].paragraph_count;

            if !inner_path.starts_with(&(outer_path + " > ")) {
                continue;
            }

            let drops_outer_title_signal = drops_outer_title_signal(
                outer_primary_heading_level,
                outer_primary_heading_depth,
                inner_primary_heading_level,
                inner_primary_heading_depth,
            );

            if outer_wrapper_adds_heading_shell(
                HeadingShellCandidate {
                    text_char_count: outer_text_char_count,
                    heading_count: outer_heading_count,
                    link_count: outer_link_count,
                    selector: &outer_selector,
                },
                HeadingShellCandidate {
                    text_char_count: inner_text_char_count,
                    heading_count: inner_heading_count,
                    link_count: inner_link_count,
                    selector: &inner_selector,
                },
            ) {
                let (inner_boost, outer_penalty) = if preference == CandidatePreference::Reading {
                    (1_900, 1_650)
                } else {
                    (2_250, 1_950)
                };
                candidates[inner_index].score += inner_boost;
                candidates[outer_index].score -= outer_penalty;
                continue;
            }

            if preference == CandidatePreference::Extraction
                && drops_outer_title_signal
                && inner_text_char_count * 100 >= outer_text_char_count * 85
                && outer_paragraph_count > 0
            {
                candidates[outer_index].score += 245;
                candidates[inner_index].score -= 280;
                continue;
            }

            if preference == CandidatePreference::Extraction
                && inner_text_char_count * 100 >= outer_text_char_count * 92
                && inner_paragraph_count + 1 >= outer_paragraph_count
                && outer_heading_count <= inner_heading_count + 2
                && (outer_link_count >= inner_link_count + 8
                    || outer_utility_descendant_count >= inner_utility_descendant_count + 2)
            {
                candidates[inner_index].score += 210;
                candidates[outer_index].score -= 165;
            }

            if preference == CandidatePreference::Extraction
                && inner_text_char_count * 100 >= outer_text_char_count * 88
                && outer_heading_count >= inner_heading_count + 12
                && outer_link_count >= inner_link_count + 24
            {
                candidates[inner_index].score += 760;
                candidates[outer_index].score -= 620;
            }

            if preference == CandidatePreference::Extraction
                && inner_text_char_count * 100 >= outer_text_char_count * 98
                && outer_heading_count >= inner_heading_count
                && outer_link_count >= inner_link_count + 20
            {
                candidates[inner_index].score += 320;
                candidates[outer_index].score -= 260;
            }

            if preference == CandidatePreference::Extraction
                && inner_text_char_count * 100 >= outer_text_char_count * 95
                && !drops_outer_title_signal
                && outer_heading_count <= inner_heading_count + 4
                && outer_link_count >= inner_link_count + 20
                && selector_stability_rank(&inner_selector)
                    >= selector_stability_rank(&outer_selector)
            {
                candidates[inner_index].score += 2400;
                candidates[outer_index].score -= 2000;
            }

            if inner_text_char_count * 100 >= outer_text_char_count * 98
                && outer_link_count >= inner_link_count + 120
            {
                let (inner_boost, outer_penalty) = if preference == CandidatePreference::Reading {
                    (620, 520)
                } else {
                    (700, 580)
                };
                candidates[inner_index].score += inner_boost;
                candidates[outer_index].score -= outer_penalty;
            }

            if preference == CandidatePreference::Extraction
                && outer_text_char_count >= inner_text_char_count.saturating_mul(6)
                && outer_paragraph_count >= inner_paragraph_count + 4
                && outer_heading_count >= inner_heading_count + 4
            {
                candidates[outer_index].score += 170;
                candidates[inner_index].score -= 190;
            }

            if inner_text_char_count * 100 >= outer_text_char_count * 78
                && inner_paragraph_count + 1 >= outer_paragraph_count
                && (outer_utility_descendant_count >= inner_utility_descendant_count + 8
                    || (outer_utility_descendant_count > inner_utility_descendant_count
                        && outer_link_count > inner_link_count + 8))
                && outer_heading_count <= inner_heading_count + 2
            {
                let (inner_boost, outer_penalty) = if preference == CandidatePreference::Extraction
                {
                    (145, 110)
                } else {
                    (120, 95)
                };
                candidates[inner_index].score += inner_boost;
                candidates[outer_index].score -= outer_penalty;
            }

            if outer_paragraph_count > 0
                && drops_outer_title_signal
                && inner_text_char_count * 100 >= outer_text_char_count * 70
                && outer_link_count <= inner_link_count + 70
            {
                let (outer_boost, inner_penalty) = if preference == CandidatePreference::Reading {
                    (185, 220)
                } else {
                    (95, 120)
                };
                candidates[outer_index].score += outer_boost;
                candidates[inner_index].score -= inner_penalty;
            }
            if outer_paragraph_count > 0
                && outer_primary_heading_count > inner_primary_heading_count
                && inner_text_char_count * 100 >= outer_text_char_count * 80
                && outer_link_count <= inner_link_count + 20
                && outer_utility_descendant_count <= inner_utility_descendant_count + 6
            {
                let (outer_boost, inner_penalty) = if preference == CandidatePreference::Reading {
                    (85, 115)
                } else {
                    (35, 50)
                };
                candidates[outer_index].score += outer_boost;
                candidates[inner_index].score -= inner_penalty;
            }
            if outer_paragraph_count > 0
                && inner_text_char_count * 100 >= outer_text_char_count * 80
                && outer_heading_count >= inner_heading_count + 4
                && outer_link_count <= inner_link_count + 20
                && outer_utility_descendant_count <= inner_utility_descendant_count + 6
            {
                let (outer_boost, inner_penalty) = if preference == CandidatePreference::Reading {
                    (90, 110)
                } else {
                    (40, 55)
                };
                candidates[outer_index].score += outer_boost;
                candidates[inner_index].score -= inner_penalty;
            }
            if outer_paragraph_count > 0
                && inner_text_char_count * 100 >= outer_text_char_count * 90
                && outer_heading_count >= inner_heading_count + 2
                && outer_link_count <= inner_link_count + 12
                && outer_utility_descendant_count <= inner_utility_descendant_count + 4
            {
                let (outer_boost, inner_penalty) = if preference == CandidatePreference::Reading {
                    (55, 75)
                } else {
                    (20, 30)
                };
                candidates[outer_index].score += outer_boost;
                candidates[inner_index].score -= inner_penalty;
            }
            if inner_text_char_count * 100 < outer_text_char_count * 68 {
                continue;
            }
            if outer_link_count > 0 && inner_link_count * 100 > outer_link_count * 80 {
                continue;
            }
            if outer_utility_descendant_count > inner_utility_descendant_count + 12 {
                continue;
            }

            let (inner_boost, outer_penalty) = if preference == CandidatePreference::Extraction {
                (90, 60)
            } else {
                (60, 40)
            };
            candidates[inner_index].score += inner_boost;
            candidates[outer_index].score -= outer_penalty;
        }
    }
}

#[cfg(test)]
fn compare_content_candidates(
    left: &RankedContentCandidate,
    right: &RankedContentCandidate,
) -> Ordering {
    compare_content_candidates_for(left, right, CandidatePreference::Reading)
}

fn compare_content_candidates_for(
    left: &RankedContentCandidate,
    right: &RankedContentCandidate,
    preference: CandidatePreference,
) -> Ordering {
    let base = right.score.cmp(&left.score);
    if base != Ordering::Equal {
        return base;
    }

    if preference == CandidatePreference::Extraction {
        return left
            .inspection
            .link_count
            .cmp(&right.inspection.link_count)
            .then_with(|| {
                selector_stability_rank(&right.inspection.selector)
                    .cmp(&selector_stability_rank(&left.inspection.selector))
            })
            .then_with(|| {
                path_depth(&right.inspection.path).cmp(&path_depth(&left.inspection.path))
            })
            .then_with(|| {
                right
                    .inspection
                    .text_char_count
                    .cmp(&left.inspection.text_char_count)
            })
            .then_with(|| left.inspection.selector.cmp(&right.inspection.selector));
    }

    right
        .inspection
        .text_char_count
        .cmp(&left.inspection.text_char_count)
        .then_with(|| {
            right
                .inspection
                .heading_count
                .cmp(&left.inspection.heading_count)
        })
        .then_with(|| {
            selector_stability_rank(&right.inspection.selector)
                .cmp(&selector_stability_rank(&left.inspection.selector))
        })
        .then_with(|| left.inspection.selector.cmp(&right.inspection.selector))
}

fn build_heading_samples(
    document: &Html,
    sample_limit: usize,
    scope_paths: &[String],
) -> Vec<HeadingInspection> {
    if sample_limit == 0 {
        return Vec::new();
    }

    let selector = Selector::parse("h1, h2, h3, h4, h5, h6").expect("heading selector");
    let mut seen_paths = BTreeSet::new();
    for scope_path in scope_paths.iter().map(String::as_str) {
        let headings = sample_headings_from_scope(
            document,
            Some(scope_path),
            sample_limit,
            &selector,
            &mut seen_paths,
        );
        if !headings.is_empty() {
            return headings;
        }
    }

    sample_headings_from_scope(document, None, sample_limit, &selector, &mut seen_paths)
}

fn sample_headings_from_scope(
    document: &Html,
    scope_path: Option<&str>,
    limit: usize,
    selector: &Selector,
    seen_paths: &mut BTreeSet<String>,
) -> Vec<HeadingInspection> {
    select_elements_in_scope(document, scope_path, selector)
        .filter_map(|element| {
            if element_looks_like_utility_chrome(&element)
                || element_has_utility_chrome_ancestor(&element)
            {
                return None;
            }

            let level = heading_level(element.value().name())?;
            let path = build_node_path(&element);
            if !seen_paths.insert(path.clone()) {
                return None;
            }
            let text = extract_heading_text(&element)?;

            Some(HeadingInspection { level, text, path })
        })
        .take(limit)
        .collect()
}

fn count_meaningful_headings(element: &ElementRef<'_>, selector: &Selector) -> usize {
    element
        .select(selector)
        .filter(|heading| extract_heading_text(heading).is_some())
        .count()
}

fn first_meaningful_heading<'a>(
    element: &'a ElementRef<'a>,
    selector: &Selector,
) -> Option<ElementRef<'a>> {
    element
        .select(selector)
        .find(|heading| extract_heading_text(heading).is_some())
}

fn build_link_samples(
    document: &Html,
    effective_base_url: Option<&str>,
    sample_limit: usize,
    scope_paths: &[String],
) -> Vec<LinkInspection> {
    if sample_limit == 0 {
        return Vec::new();
    }

    let selector = Selector::parse("a").expect("link selector");
    let mut seen_paths = BTreeSet::new();
    let mut links = Vec::new();
    for scope_path in scope_paths.iter().map(String::as_str) {
        links.extend(sample_links_from_scope(
            document,
            effective_base_url,
            Some(scope_path),
            sample_limit.saturating_sub(links.len()),
            &selector,
            &mut seen_paths,
        ));
        if links.len() >= sample_limit {
            return links;
        }
    }

    links.extend(sample_links_from_scope(
        document,
        effective_base_url,
        None,
        sample_limit.saturating_sub(links.len()),
        &selector,
        &mut seen_paths,
    ));
    links
}

fn sample_links_from_scope(
    document: &Html,
    effective_base_url: Option<&str>,
    scope_path: Option<&str>,
    limit: usize,
    selector: &Selector,
    seen_paths: &mut BTreeSet<String>,
) -> Vec<LinkInspection> {
    select_elements_in_scope(document, scope_path, selector)
        .filter_map(|element| {
            if element_looks_like_utility_chrome(&element)
                || element_has_utility_chrome_ancestor(&element)
            {
                return None;
            }

            let href = element
                .value()
                .attr("href")
                .map(str::trim)
                .filter(|value| href_is_meaningful_destination(value))
                .map(str::to_owned)?;
            let text =
                render_html_as_text(&serialize_children(&element), WhitespaceMode::Normalize);
            if text.is_empty() {
                return None;
            }
            if link_preview_is_low_signal(&href, &text, &path_hint_for_link(&element)) {
                return None;
            }

            let path = build_node_path(&element);
            if !seen_paths.insert(path.clone()) {
                return None;
            }

            let resolved_href = resolve_url(&href, effective_base_url);
            if effective_base_url.is_some_and(|base| same_page_url(&resolved_href, base)) {
                return None;
            }

            Some(LinkInspection {
                text,
                resolved_href: Some(resolved_href),
                href: Some(href),
                path,
            })
        })
        .take(limit)
        .collect()
}

fn path_hint_for_link(element: &ElementRef<'_>) -> String {
    build_node_path(element)
}

fn link_preview_is_low_signal(href: &str, text: &str, path: &str) -> bool {
    if !href.starts_with('#') {
        let normalized_text = text.to_ascii_lowercase();
        let normalized_href = href.to_ascii_lowercase();
        let normalized_path = path.to_ascii_lowercase();
        if LOW_SIGNAL_LINK_TEXT_PHRASES
            .iter()
            .any(|phrase| normalized_text.contains(phrase))
        {
            return true;
        }
        if LOW_SIGNAL_LINK_HREF_FRAGMENTS
            .iter()
            .any(|fragment| normalized_href.contains(fragment))
        {
            return true;
        }
        if LOW_SIGNAL_LINK_PATH_TOKENS
            .iter()
            .any(|token| normalized_path.contains(token))
        {
            return true;
        }
        return false;
    }

    let trimmed = text.trim();
    let has_word = trimmed.chars().any(char::is_alphabetic);
    let only_marker_chars = !trimmed.is_empty()
        && trimmed.chars().all(|character| {
            character.is_ascii_digit() || matches!(character, '*' | '#' | '[' | ']')
        });

    (!has_word && only_marker_chars) || path.contains("> sup:nth-of-type(")
}

fn count_meaningful_links(element: &ElementRef<'_>, selector: &Selector) -> usize {
    element
        .select(selector)
        .filter(|candidate| {
            if element_looks_like_utility_chrome(candidate) {
                return false;
            }
            if element_has_utility_chrome_ancestor(candidate) {
                return false;
            }

            let Some(href) = candidate
                .value()
                .attr("href")
                .map(str::trim)
                .filter(|value| href_is_meaningful_destination(value))
            else {
                return false;
            };
            let text =
                render_html_as_text(&serialize_children(candidate), WhitespaceMode::Normalize);
            if text.is_empty() {
                return false;
            }
            if link_preview_is_low_signal(href, &text, &path_hint_for_link(candidate)) {
                return false;
            }
            true
        })
        .count()
}

fn narrative_block_count(prose_paragraph_count: usize, list_item_count: usize) -> usize {
    prose_paragraph_count + list_item_count.div_ceil(3).min(6)
}

fn candidate_has_readable_density(
    tag_name: &str,
    text_char_count: usize,
    heading_count: usize,
    link_count: usize,
    body_block_count: usize,
    prose_paragraph_count: usize,
) -> bool {
    if text_char_count < 20 {
        return false;
    }

    if !matches!(tag_name, "article" | "main") && body_block_count == 0 && text_char_count < 120 {
        return false;
    }

    let chars_per_heading = text_char_count
        .checked_div(heading_count)
        .unwrap_or(usize::MAX);
    let chars_per_link = text_char_count
        .checked_div(link_count)
        .unwrap_or(usize::MAX);

    if prose_paragraph_count == 0 && body_block_count <= 2 && text_char_count < 220 {
        return chars_per_heading >= 24 && chars_per_link >= 18;
    }

    if text_char_count < 4_000 && heading_count > body_block_count.saturating_mul(3).max(12) {
        return false;
    }

    if text_char_count < 4_000 && link_count > body_block_count.saturating_mul(5).max(18) {
        return false;
    }
    true
}

fn same_page_url(candidate: &str, current: &str) -> bool {
    let Ok(mut candidate_url) = url::Url::parse(candidate) else {
        return false;
    };
    let Ok(mut current_url) = url::Url::parse(current) else {
        return false;
    };

    candidate_url.set_fragment(None);
    current_url.set_fragment(None);
    candidate_url == current_url
}

fn prepend_document_title_heading_if_missing(
    document: &Html,
    sample_limit: usize,
    headings: &mut Vec<HeadingInspection>,
) {
    if sample_limit == 0 || headings.iter().any(|heading| heading.level == 1) {
        return;
    }

    let selector = Selector::parse("h1").expect("h1 selector");
    let mut seen_paths = headings
        .iter()
        .map(|heading| heading.path.clone())
        .collect::<BTreeSet<_>>();
    let Some(document_heading) =
        sample_headings_from_scope(document, None, 1, &selector, &mut seen_paths)
            .into_iter()
            .next()
    else {
        return;
    };

    headings.insert(0, document_heading);
    if headings.len() > sample_limit {
        headings.truncate(sample_limit);
    }
}

fn select_elements_in_scope<'a>(
    document: &'a Html,
    scope_path: Option<&str>,
    selector: &'a Selector,
) -> Box<dyn Iterator<Item = ElementRef<'a>> + 'a> {
    if let Some(scope) = scope_path.and_then(|path| select_first(document, path)) {
        return Box::new(scope.select(selector));
    }

    Box::new(document.select(selector))
}

fn element_attr_equals_ignore_ascii_case(
    element: &ElementRef<'_>,
    attribute_name: &str,
    expected_value: &str,
) -> bool {
    match element.value().attr(attribute_name) {
        Some(value) => value.eq_ignore_ascii_case(expected_value),
        None => false,
    }
}

fn is_content_candidate_container(element: &ElementRef<'_>, positive_signal_count: usize) -> bool {
    match element.value().name() {
        "main" | "article" => true,
        "section" => {
            if positive_signal_count > 0 {
                return true;
            }
            if element_attr_equals_ignore_ascii_case(element, "role", "main") {
                return true;
            }
            if element_attr_equals_ignore_ascii_case(element, "itemprop", "articleBody") {
                return true;
            }
            element_has_narrative_section_shape(element)
        }
        "div" => {
            if positive_signal_count > 0 {
                return true;
            }
            if element_attr_equals_ignore_ascii_case(element, "role", "main") {
                return true;
            }
            if element_attr_equals_ignore_ascii_case(element, "itemprop", "articleBody") {
                return true;
            }
            false
        }
        _ => false,
    }
}

fn element_has_narrative_section_shape(element: &ElementRef<'_>) -> bool {
    let mut paragraph_like = 0usize;
    let mut heading_like = 0usize;
    let mut list_like = 0usize;

    for descendant in element.descendants().filter_map(ElementRef::wrap) {
        if descendant.id() == element.id() {
            continue;
        }

        match descendant.value().name() {
            "p" => paragraph_like += 1,
            "h1" | "h2" | "h3" => heading_like += 1,
            "li" => list_like += 1,
            _ => {}
        }

        if paragraph_like >= 3 {
            return true;
        }
        if paragraph_like >= 2 {
            if heading_like >= 1 {
                return true;
            }
            if list_like >= 2 {
                return true;
            }
        }
    }

    false
}

#[cfg(test)]
fn content_candidate_score(inputs: &ContentCandidateScoreInputs<'_>) -> i32 {
    content_candidate_score_for(inputs, CandidatePreference::Reading)
}

fn content_candidate_score_for(
    inputs: &ContentCandidateScoreInputs<'_>,
    preference: CandidatePreference,
) -> i32 {
    let shallow_primary_heading =
        has_shallow_primary_heading(inputs.primary_heading_level, inputs.primary_heading_depth);
    let (
        tag_bonus,
        role_bonus,
        itemprop_bonus,
        text_divisor,
        heading_multiplier,
        paragraph_multiplier,
        positive_multiplier,
        negative_multiplier,
        utility_multiplier,
        exact_path_penalty,
        heading_absence_penalty,
        short_text_penalty,
        body_absence_penalty,
        title_fragment_penalty,
        link_density_penalty,
        primary_heading_bonus,
        primary_heading_count_bonus,
    ) = match preference {
        CandidatePreference::Extraction => {
            let tag_bonus = match inputs.tag_name {
                "article" => 120,
                "main" => 70,
                "section" => 28,
                "div" => 18,
                _ => 0,
            };
            let utility_multiplier =
                if matches!(inputs.tag_name, "article" | "main") && shallow_primary_heading {
                    18
                } else {
                    24
                };
            let body_absence_penalty =
                if inputs.paragraph_count == 0 && inputs.text_char_count < 500 {
                    200
                } else if inputs.paragraph_count <= 1 && inputs.text_char_count < 420 {
                    95
                } else {
                    0
                };
            let title_fragment_penalty = if !matches!(inputs.tag_name, "article" | "main")
                && shallow_primary_heading
                && inputs.paragraph_count == 0
                && inputs.text_char_count < 420
            {
                200
            } else {
                0
            };
            let link_density_penalty = if inputs.text_char_count < 240 && inputs.link_count > 8 {
                30
            } else if inputs.link_count > inputs.paragraph_count.saturating_mul(6)
                && inputs.text_char_count < 1_600
            {
                25
            } else if inputs.link_count > inputs.paragraph_count.saturating_mul(4)
                && inputs.text_char_count < 6_500
            {
                60
            } else if inputs.link_count > inputs.paragraph_count.saturating_mul(3)
                && inputs.text_char_count < 12_000
            {
                34
            } else {
                0
            };
            (
                tag_bonus,
                28,
                55,
                105,
                10,
                7,
                22,
                34,
                utility_multiplier,
                220,
                55,
                35,
                body_absence_penalty,
                title_fragment_penalty,
                link_density_penalty,
                0,
                0,
            )
        }
        CandidatePreference::Reading => {
            let tag_bonus = match inputs.tag_name {
                "main" => 100,
                "article" => 90,
                "section" => 30,
                "div" => 15,
                _ => 0,
            };
            let utility_multiplier =
                if matches!(inputs.tag_name, "article" | "main") && shallow_primary_heading {
                    12
                } else {
                    18
                };
            let primary_heading_bonus = if shallow_primary_heading {
                inputs
                    .primary_heading_level
                    .map(primary_heading_bonus)
                    .unwrap_or(0)
            } else {
                0
            };
            let primary_heading_count_bonus = if shallow_primary_heading {
                (inputs.primary_heading_count.min(2) as i32) * 38
            } else {
                0
            };
            let body_absence_penalty =
                if inputs.paragraph_count == 0 && inputs.text_char_count < 500 {
                    180
                } else if inputs.paragraph_count <= 1 && inputs.text_char_count < 320 {
                    75
                } else {
                    0
                };
            let title_fragment_penalty = if !matches!(inputs.tag_name, "article" | "main")
                && shallow_primary_heading
                && inputs.paragraph_count == 0
                && inputs.text_char_count < 300
            {
                170
            } else {
                0
            };
            let link_density_penalty = if inputs.text_char_count < 240 && inputs.link_count > 8 {
                25
            } else if inputs.link_count > inputs.paragraph_count.saturating_mul(6)
                && inputs.text_char_count < 1_200
            {
                15
            } else if inputs.link_count > inputs.paragraph_count.saturating_mul(4)
                && inputs.text_char_count < 4_000
            {
                40
            } else if inputs.link_count > inputs.paragraph_count.saturating_mul(3)
                && inputs.text_char_count < 6_000
            {
                22
            } else {
                0
            };
            (
                tag_bonus,
                45,
                35,
                90,
                12,
                7,
                20,
                28,
                utility_multiplier,
                220,
                45,
                30,
                body_absence_penalty,
                title_fragment_penalty,
                link_density_penalty,
                primary_heading_bonus,
                primary_heading_count_bonus,
            )
        }
    };

    tag_bonus
        + inputs.has_main_role as i32 * role_bonus
        + inputs.has_article_body_itemprop as i32 * itemprop_bonus
        + (inputs.text_char_count.min(8_000) / text_divisor) as i32
        + (inputs.heading_count.min(8) as i32 * heading_multiplier)
        + primary_heading_bonus
        + primary_heading_count_bonus
        + (inputs.paragraph_count.min(16) as i32 * paragraph_multiplier)
        + (inputs.positive_signal_count.min(4) as i32 * positive_multiplier)
        - (inputs.negative_signal_count.min(4) as i32 * negative_multiplier)
        - (inputs.utility_descendant_count.min(12) as i32 * utility_multiplier)
        - (inputs.uses_exact_path_selector as i32 * exact_path_penalty)
        - heading_absence_penalty
        - short_text_penalty
        - body_absence_penalty
        - title_fragment_penalty
        - link_density_penalty
}

fn path_depth(path: &str) -> usize {
    path.matches(" > ").count()
}

fn selector_stability_rank(selector: &str) -> u8 {
    if selector.contains(":nth-of-type(") {
        return 0;
    }

    if selector.starts_with('#') {
        return 5;
    }

    if selector.contains("[itemprop=") || selector.contains("[role=") || selector.starts_with('[') {
        return 4;
    }

    if selector.contains('.') && selector.chars().next().is_some_and(char::is_alphabetic) {
        return 3;
    }

    if selector.starts_with('.') {
        return 2;
    }

    1
}

fn promote_precise_reading_descendant_candidate(
    extraction_candidates: &mut Vec<RankedContentCandidate>,
    reading_candidates: &[RankedContentCandidate],
) {
    let Some(current_extraction) = extraction_candidates.first() else {
        return;
    };
    let Some(reading_top) = reading_candidates.first() else {
        return;
    };

    let promoted_candidate = if current_extraction.inspection.path == reading_top.inspection.path {
        let descendant_prefix = format!("{} > ", reading_top.inspection.path);
        reading_candidates
            .iter()
            .skip(1)
            .filter(|candidate| {
                candidate.inspection.path.starts_with(&descendant_prefix)
                    && candidate.inspection.text_char_count * 100
                        >= reading_top.inspection.text_char_count * 92
                    && ((candidate.inspection.heading_count + 2
                        >= reading_top.inspection.heading_count
                        && candidate.inspection.link_count <= reading_top.inspection.link_count)
                        || outer_wrapper_adds_heading_shell(
                            HeadingShellCandidate {
                                text_char_count: reading_top.inspection.text_char_count,
                                heading_count: reading_top.inspection.heading_count,
                                link_count: reading_top.inspection.link_count,
                                selector: &reading_top.inspection.selector,
                            },
                            HeadingShellCandidate {
                                text_char_count: candidate.inspection.text_char_count,
                                heading_count: candidate.inspection.heading_count,
                                link_count: candidate.inspection.link_count,
                                selector: &candidate.inspection.selector,
                            },
                        ))
            })
            .max_by(|left, right| {
                content_tag_rank(&left.inspection.tag_name)
                    .cmp(&content_tag_rank(&right.inspection.tag_name))
                    .then_with(|| {
                        selector_stability_rank(&left.inspection.selector)
                            .cmp(&selector_stability_rank(&right.inspection.selector))
                    })
                    .then_with(|| {
                        right
                            .inspection
                            .text_char_count
                            .cmp(&left.inspection.text_char_count)
                    })
                    .then_with(|| right.inspection.link_count.cmp(&left.inspection.link_count))
                    .then_with(|| {
                        path_depth(&right.inspection.path).cmp(&path_depth(&left.inspection.path))
                    })
                    .then_with(|| left.inspection.selector.cmp(&right.inspection.selector))
            })
            .cloned()
    } else {
        let descendant_prefix = format!("{} > ", current_extraction.inspection.path);
        reading_candidates
            .iter()
            .filter(|candidate| {
                candidate.inspection.path.starts_with(&descendant_prefix)
                    && candidate.inspection.text_char_count * 100
                        >= current_extraction.inspection.text_char_count * 90
                    && (current_extraction.inspection.link_count
                        >= candidate.inspection.link_count + 20
                        || outer_wrapper_adds_heading_shell(
                            HeadingShellCandidate {
                                text_char_count: current_extraction.inspection.text_char_count,
                                heading_count: current_extraction.inspection.heading_count,
                                link_count: current_extraction.inspection.link_count,
                                selector: &current_extraction.inspection.selector,
                            },
                            HeadingShellCandidate {
                                text_char_count: candidate.inspection.text_char_count,
                                heading_count: candidate.inspection.heading_count,
                                link_count: candidate.inspection.link_count,
                                selector: &candidate.inspection.selector,
                            },
                        ))
                    && selector_stability_rank(&candidate.inspection.selector) >= 2
            })
            .max_by(|left, right| {
                selector_stability_rank(&left.inspection.selector)
                    .cmp(&selector_stability_rank(&right.inspection.selector))
                    .then_with(|| {
                        content_tag_rank(&left.inspection.tag_name)
                            .cmp(&content_tag_rank(&right.inspection.tag_name))
                    })
                    .then_with(|| {
                        right
                            .inspection
                            .text_char_count
                            .cmp(&left.inspection.text_char_count)
                    })
                    .then_with(|| right.inspection.link_count.cmp(&left.inspection.link_count))
                    .then_with(|| {
                        path_depth(&right.inspection.path).cmp(&path_depth(&left.inspection.path))
                    })
                    .then_with(|| left.inspection.selector.cmp(&right.inspection.selector))
            })
            .cloned()
    };
    let Some(promoted_candidate) = promoted_candidate else {
        return;
    };

    extraction_candidates
        .retain(|candidate| candidate.inspection.path != promoted_candidate.inspection.path);
    extraction_candidates.insert(0, promoted_candidate);
}

fn promote_title_bearing_reading_ancestor_candidate(
    extraction_candidates: &mut Vec<RankedContentCandidate>,
    reading_candidates: &[RankedContentCandidate],
) {
    let Some(current_extraction) = extraction_candidates.first() else {
        return;
    };
    let Some(reading_top) = reading_candidates.first() else {
        return;
    };
    if reading_top.inspection.path == current_extraction.inspection.path {
        return;
    }
    if !current_extraction
        .inspection
        .path
        .starts_with(&(reading_top.inspection.path.clone() + " > "))
    {
        return;
    }
    if !drops_outer_title_signal(
        reading_top.primary_heading_level,
        reading_top.primary_heading_depth,
        current_extraction.primary_heading_level,
        current_extraction.primary_heading_depth,
    ) {
        return;
    }
    if current_extraction.inspection.text_char_count * 100
        < reading_top.inspection.text_char_count * 85
    {
        return;
    }
    if reading_top.inspection.heading_count + 2 < current_extraction.inspection.heading_count {
        return;
    }
    if reading_top.inspection.link_count > current_extraction.inspection.link_count + 60 {
        return;
    }

    let promoted_candidate = reading_top.clone();
    extraction_candidates
        .retain(|candidate| candidate.inspection.path != promoted_candidate.inspection.path);
    extraction_candidates.insert(0, promoted_candidate);
}

fn promote_cleaner_reading_descendant_candidate(
    extraction_candidates: &mut Vec<RankedContentCandidate>,
    reading_candidates: &[RankedContentCandidate],
) {
    let Some(current_extraction) = extraction_candidates.first() else {
        return;
    };

    let descendant_prefix = format!("{} > ", current_extraction.inspection.path);
    let promoted_candidate = reading_candidates
        .iter()
        .filter(|candidate| {
            candidate.inspection.path.starts_with(&descendant_prefix)
                && candidate.inspection.text_char_count * 100
                    >= current_extraction.inspection.text_char_count * 90
                && candidate.inspection.heading_count + 2
                    >= current_extraction.inspection.heading_count
                && current_extraction.inspection.link_count >= candidate.inspection.link_count + 20
                && !drops_outer_title_signal(
                    current_extraction.primary_heading_level,
                    current_extraction.primary_heading_depth,
                    candidate.primary_heading_level,
                    candidate.primary_heading_depth,
                )
        })
        .min_by(|left, right| {
            left.inspection
                .link_count
                .cmp(&right.inspection.link_count)
                .then_with(|| {
                    selector_stability_rank(&right.inspection.selector)
                        .cmp(&selector_stability_rank(&left.inspection.selector))
                })
                .then_with(|| {
                    right
                        .inspection
                        .text_char_count
                        .cmp(&left.inspection.text_char_count)
                })
                .then_with(|| {
                    path_depth(&left.inspection.path).cmp(&path_depth(&right.inspection.path))
                })
                .then_with(|| left.inspection.selector.cmp(&right.inspection.selector))
        })
        .cloned();

    let Some(promoted_candidate) = promoted_candidate else {
        return;
    };

    extraction_candidates
        .retain(|candidate| candidate.inspection.path != promoted_candidate.inspection.path);
    extraction_candidates.insert(0, promoted_candidate);
}

fn content_tag_rank(tag_name: &str) -> u8 {
    match tag_name {
        "article" => 4,
        "main" => 3,
        "section" => 2,
        "div" => 1,
        _ => 0,
    }
}

fn normalized_body_text_char_count(document: &Html) -> usize {
    let mut rendered = String::new();
    if let Some(body) = first_body(document) {
        collect_visible_text_for_count(body.children(), &mut rendered);
    } else if select_first(document, "html").is_some() {
        collect_visible_text_for_count(document.root_element().children(), &mut rendered);
    } else {
        collect_visible_text_for_count(document.tree.root().children(), &mut rendered);
    }

    rendered.chars().count()
}

fn collect_visible_text_for_count<'a>(
    nodes: impl Iterator<Item = ego_tree::NodeRef<'a, Node>>,
    output: &mut String,
) {
    for node in nodes {
        match node.value() {
            Node::Text(contents) => {
                let normalized = collapse_inline_whitespace_for_count(contents);
                if normalized.is_empty() {
                    continue;
                }
                push_count_text(output, &normalized);
            }
            Node::Element(data) => {
                if matches!(
                    data.name(),
                    "head" | "noscript" | "script" | "style" | "template"
                ) {
                    continue;
                }

                if data.name() == "img" {
                    let alt_text = data
                        .attr("alt")
                        .map(collapse_inline_whitespace_for_count)
                        .filter(|alt| !alt.is_empty());
                    if let Some(alt_text) = alt_text {
                        push_count_text(output, &alt_text);
                    }
                    continue;
                }

                collect_visible_text_for_count(node.children(), output);
            }
            _ => collect_visible_text_for_count(node.children(), output),
        }
    }
}

fn push_count_text(output: &mut String, text: &str) {
    if output
        .chars()
        .last()
        .is_some_and(|character| !character.is_whitespace())
    {
        output.push(' ');
    }
    output.push_str(text);
}

fn collapse_inline_whitespace_for_count(input: &str) -> String {
    input.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn recommend_content_selector(
    document: &Html,
    element: &ElementRef<'_>,
    exact_path: &str,
) -> String {
    let target_node_id = element.id();
    let candidates = selector_candidates_for_element(element, exact_path);

    for candidate in candidates {
        if selector_uniquely_matches(document, &candidate, target_node_id) {
            return candidate;
        }
    }

    exact_path.to_owned()
}

fn selector_candidates_for_element(element: &ElementRef<'_>, _exact_path: &str) -> Vec<String> {
    let tag_name = element.value().name();
    let mut candidates = Vec::new();

    if let Some(id) = element
        .value()
        .attr("id")
        .map(str::trim)
        .filter(|id| !id.is_empty())
    {
        candidates.push(id_selector(id));
    }

    if let Some(role) = element
        .value()
        .attr("role")
        .map(str::trim)
        .filter(|role| !role.is_empty())
    {
        let escaped = css_string_literal(role);
        candidates.push(format!("{tag_name}[role=\"{escaped}\"]"));
        candidates.push(format!("[role=\"{escaped}\"]"));
    }

    if let Some(itemprop) = element
        .value()
        .attr("itemprop")
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        let escaped = css_string_literal(itemprop);
        candidates.push(format!("{tag_name}[itemprop=\"{escaped}\"]"));
    }

    let selector_classes = selector_classes(element);
    if !selector_classes.is_empty() {
        let class_suffix = selector_classes
            .iter()
            .map(|class_name| format!(".{class_name}"))
            .collect::<String>();
        candidates.push(format!("{tag_name}{class_suffix}"));
        candidates.push(class_suffix);
    }

    let mut seen = BTreeSet::new();
    candidates.push(tag_name.to_owned());
    candidates.retain(|candidate| seen.insert(candidate.clone()));
    candidates
}

fn selector_classes(element: &ElementRef<'_>) -> Vec<String> {
    let valid_classes = element
        .value()
        .attr("class")
        .into_iter()
        .flat_map(|value| value.split_whitespace())
        .filter(|class_name| simple_css_identifier(class_name))
        .map(str::to_owned)
        .collect::<Vec<_>>();

    let mut specific_classes = valid_classes
        .iter()
        .filter(|class_name| !GENERIC_SELECTOR_CLASSES.contains(&class_name.as_str()))
        .cloned()
        .collect::<Vec<_>>();
    if specific_classes.is_empty() {
        specific_classes = valid_classes;
    }
    specific_classes.truncate(2);
    specific_classes
}

fn selector_uniquely_matches(
    document: &Html,
    selector: &str,
    target_node_id: ego_tree::NodeId,
) -> bool {
    let Ok(selector) = Selector::parse(selector) else {
        return false;
    };
    let mut matches = document.select(&selector);
    let Some(first_match) = matches.next() else {
        return false;
    };
    first_match.id() == target_node_id && matches.next().is_none()
}

fn id_selector(id: &str) -> String {
    if simple_css_identifier(id) {
        format!("#{id}")
    } else {
        format!("[id=\"{}\"]", css_string_literal(id))
    }
}

fn simple_css_identifier(value: &str) -> bool {
    let mut characters = value.chars();
    let Some(first) = characters.next() else {
        return false;
    };
    if !matches!(first, 'A'..='Z' | 'a'..='z' | '_' | '-') {
        return false;
    }

    characters.all(|character| matches!(character, 'A'..='Z' | 'a'..='z' | '0'..='9' | '_' | '-'))
}

fn css_string_literal(value: &str) -> String {
    value.replace('\\', "\\\\").replace('"', "\\\"")
}

fn primary_heading_bonus(level: u8) -> i32 {
    match level {
        1 => 130,
        2 => 78,
        3 => 20,
        _ => 0,
    }
}

fn has_shallow_primary_heading(level: Option<u8>, depth: Option<usize>) -> bool {
    match (level, depth) {
        (Some(1), Some(primary_heading_depth)) => primary_heading_depth <= 5,
        (Some(2), Some(primary_heading_depth)) => primary_heading_depth <= 2,
        _ => false,
    }
}

fn drops_outer_title_signal(
    outer_level: Option<u8>,
    outer_depth: Option<usize>,
    inner_level: Option<u8>,
    inner_depth: Option<usize>,
) -> bool {
    if !has_shallow_primary_heading(outer_level, outer_depth) {
        return false;
    }

    if outer_level == Some(1) {
        !matches!(
            (inner_level, inner_depth),
            (Some(1), Some(inner_depth))
                if has_shallow_primary_heading(Some(1), Some(inner_depth))
        )
    } else {
        !matches!(
            (inner_level, inner_depth),
            (Some(1 | 2), Some(inner_depth))
                if has_shallow_primary_heading(inner_level, Some(inner_depth))
        )
    }
}

fn outer_wrapper_adds_heading_shell(
    outer: HeadingShellCandidate<'_>,
    inner: HeadingShellCandidate<'_>,
) -> bool {
    inner.heading_count >= 2
        && inner.text_char_count * 100 >= outer.text_char_count * 94
        && outer.heading_count >= inner.heading_count.saturating_add(24)
        && outer.heading_count >= inner.heading_count.saturating_mul(2)
        && outer.link_count <= inner.link_count.saturating_add(12)
        && selector_stability_rank(inner.selector) >= selector_stability_rank(outer.selector)
}

fn descendant_element_depth(
    ancestor: &ElementRef<'_>,
    descendant: &ElementRef<'_>,
) -> Option<usize> {
    if ancestor.id() == descendant.id() {
        return Some(0);
    }

    let mut depth = 0usize;
    let mut parent = descendant.parent();
    while let Some(current) = parent {
        if let Some(parent_element) = ElementRef::wrap(current) {
            depth += 1;
            if parent_element.id() == ancestor.id() {
                return Some(depth);
            }
        }
        parent = current.parent();
    }

    None
}

fn count_utility_descendant_roots(element: &ElementRef<'_>) -> usize {
    element
        .descendants()
        .filter_map(ElementRef::wrap)
        .filter(|descendant| descendant.id() != element.id())
        .filter(|descendant| element_looks_like_utility_chrome(descendant))
        .filter(|descendant| !has_utility_chrome_ancestor_before(descendant, element.id()))
        .count()
}

fn has_utility_chrome_ancestor_before(
    element: &ElementRef<'_>,
    boundary: ego_tree::NodeId,
) -> bool {
    let mut parent = element.parent();
    while let Some(current) = parent {
        if current.id() == boundary {
            return false;
        }
        if let Some(parent_element) = ElementRef::wrap(current)
            && element_looks_like_utility_chrome(&parent_element)
        {
            return true;
        }
        parent = current.parent();
    }

    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::document::{parse_document_node, select_first};
    use std::collections::BTreeSet;

    struct CandidateFixture<'a> {
        path: &'a str,
        selector: &'a str,
        text_char_count: usize,
        heading_count: usize,
        link_count: usize,
        primary_heading_level: Option<u8>,
        primary_heading_count: usize,
        primary_heading_depth: Option<usize>,
        utility_descendant_count: usize,
        score: i32,
    }

    struct PromotionFixture<'a> {
        selector: &'a str,
        path: &'a str,
        tag_name: &'a str,
        text_char_count: usize,
        heading_count: usize,
        link_count: usize,
        primary_heading_level: Option<u8>,
        primary_heading_depth: Option<usize>,
    }

    struct BiasFixture<'a> {
        selector: &'a str,
        path: &'a str,
        tag_name: &'a str,
        text_char_count: usize,
        heading_count: usize,
        link_count: usize,
        paragraph_count: usize,
        primary_heading_level: Option<u8>,
        primary_heading_count: usize,
        primary_heading_depth: Option<usize>,
        utility_descendant_count: usize,
        score: i32,
    }

    fn ranked_candidate(fixture: CandidateFixture<'_>) -> RankedContentCandidate {
        RankedContentCandidate {
            score: fixture.score,
            inspection: ContentCandidateInspection {
                selector: fixture.selector.to_owned(),
                path: fixture.path.to_owned(),
                tag_name: "section".to_owned(),
                text_char_count: fixture.text_char_count,
                heading_count: fixture.heading_count,
                link_count: fixture.link_count,
            },
            paragraph_count: 2,
            primary_heading_level: fixture.primary_heading_level,
            primary_heading_count: fixture.primary_heading_count,
            primary_heading_depth: fixture.primary_heading_depth,
            utility_descendant_count: fixture.utility_descendant_count,
        }
    }

    fn content_candidate(
        selector: &str,
        path: &str,
        tag_name: &str,
        text_char_count: usize,
        heading_count: usize,
        link_count: usize,
    ) -> ContentCandidateInspection {
        ContentCandidateInspection {
            selector: selector.to_owned(),
            path: path.to_owned(),
            tag_name: tag_name.to_owned(),
            text_char_count,
            heading_count,
            link_count,
        }
    }

    fn ranked_content_candidate(fixture: PromotionFixture<'_>) -> RankedContentCandidate {
        RankedContentCandidate {
            score: 0,
            inspection: content_candidate(
                fixture.selector,
                fixture.path,
                fixture.tag_name,
                fixture.text_char_count,
                fixture.heading_count,
                fixture.link_count,
            ),
            paragraph_count: 2,
            primary_heading_level: fixture.primary_heading_level,
            primary_heading_count: usize::from(fixture.primary_heading_level.is_some()),
            primary_heading_depth: fixture.primary_heading_depth,
            utility_descendant_count: 0,
        }
    }

    fn ranked_bias_candidate(fixture: BiasFixture<'_>) -> RankedContentCandidate {
        RankedContentCandidate {
            score: fixture.score,
            inspection: content_candidate(
                fixture.selector,
                fixture.path,
                fixture.tag_name,
                fixture.text_char_count,
                fixture.heading_count,
                fixture.link_count,
            ),
            paragraph_count: fixture.paragraph_count,
            primary_heading_level: fixture.primary_heading_level,
            primary_heading_count: fixture.primary_heading_count,
            primary_heading_depth: fixture.primary_heading_depth,
            utility_descendant_count: fixture.utility_descendant_count,
        }
    }

    #[test]
    fn nested_candidate_bias_and_scoring_helpers_cover_remaining_paths() {
        let mut sorted_candidates = [
            ranked_candidate(CandidateFixture {
                path: "a",
                selector: "selector-b",
                text_char_count: 500,
                heading_count: 2,
                link_count: 0,
                primary_heading_level: None,
                primary_heading_count: 0,
                primary_heading_depth: None,
                utility_descendant_count: 0,
                score: 100,
            }),
            ranked_candidate(CandidateFixture {
                path: "b",
                selector: "selector-a",
                text_char_count: 700,
                heading_count: 1,
                link_count: 0,
                primary_heading_level: None,
                primary_heading_count: 0,
                primary_heading_depth: None,
                utility_descendant_count: 0,
                score: 100,
            }),
            ranked_candidate(CandidateFixture {
                path: "c",
                selector: "selector-c",
                text_char_count: 700,
                heading_count: 3,
                link_count: 0,
                primary_heading_level: None,
                primary_heading_count: 0,
                primary_heading_depth: None,
                utility_descendant_count: 0,
                score: 100,
            }),
        ];
        sorted_candidates.sort_by(compare_content_candidates);
        assert_eq!(sorted_candidates[0].inspection.selector, "selector-c");
        assert_eq!(sorted_candidates[1].inspection.selector, "selector-a");
        assert_eq!(sorted_candidates[2].inspection.selector, "selector-b");

        let mut shallow_heading_pair = vec![
            ranked_candidate(CandidateFixture {
                path: "article:nth-of-type(1)",
                selector: "article",
                text_char_count: 1_000,
                heading_count: 2,
                link_count: 60,
                primary_heading_level: Some(1),
                primary_heading_count: 1,
                primary_heading_depth: Some(1),
                utility_descendant_count: 20,
                score: 500,
            }),
            ranked_candidate(CandidateFixture {
                path: "article:nth-of-type(1) > div:nth-of-type(1)",
                selector: "article > div",
                text_char_count: 800,
                heading_count: 1,
                link_count: 5,
                primary_heading_level: None,
                primary_heading_count: 0,
                primary_heading_depth: None,
                utility_descendant_count: 0,
                score: 500,
            }),
        ];
        apply_nested_content_candidate_bias(&mut shallow_heading_pair);
        assert_eq!(shallow_heading_pair[0].score, 590);
        assert_eq!(shallow_heading_pair[1].score, 400);

        let mut heading_rich_outer = vec![
            ranked_candidate(CandidateFixture {
                path: "main:nth-of-type(1)",
                selector: "main",
                text_char_count: 1_000,
                heading_count: 6,
                link_count: 6,
                primary_heading_level: None,
                primary_heading_count: 0,
                primary_heading_depth: None,
                utility_descendant_count: 1,
                score: 300,
            }),
            ranked_candidate(CandidateFixture {
                path: "main:nth-of-type(1) > section:nth-of-type(1)",
                selector: "main > section",
                text_char_count: 850,
                heading_count: 1,
                link_count: 10,
                primary_heading_level: None,
                primary_heading_count: 0,
                primary_heading_depth: None,
                utility_descendant_count: 1,
                score: 300,
            }),
        ];
        apply_nested_content_candidate_bias(&mut heading_rich_outer);
        assert_eq!(heading_rich_outer[0].score, 390);
        assert_eq!(heading_rich_outer[1].score, 190);

        let mut modest_heading_outer = vec![
            ranked_candidate(CandidateFixture {
                path: "div:nth-of-type(1)",
                selector: "div",
                text_char_count: 1_000,
                heading_count: 3,
                link_count: 2,
                primary_heading_level: None,
                primary_heading_count: 0,
                primary_heading_depth: None,
                utility_descendant_count: 1,
                score: 200,
            }),
            ranked_candidate(CandidateFixture {
                path: "div:nth-of-type(1) > section:nth-of-type(1)",
                selector: "div > section",
                text_char_count: 950,
                heading_count: 1,
                link_count: 3,
                primary_heading_level: None,
                primary_heading_count: 0,
                primary_heading_depth: None,
                utility_descendant_count: 1,
                score: 200,
            }),
        ];
        apply_nested_content_candidate_bias(&mut modest_heading_outer);
        assert_eq!(modest_heading_outer[0].score, 255);
        assert_eq!(modest_heading_outer[1].score, 125);

        let mut utility_heavy_outer = vec![
            ranked_candidate(CandidateFixture {
                path: "main:nth-of-type(1)",
                selector: "main",
                text_char_count: 1_000,
                heading_count: 1,
                link_count: 2,
                primary_heading_level: None,
                primary_heading_count: 0,
                primary_heading_depth: None,
                utility_descendant_count: 20,
                score: 150,
            }),
            ranked_candidate(CandidateFixture {
                path: "main:nth-of-type(1) > article:nth-of-type(1)",
                selector: "main > article",
                text_char_count: 900,
                heading_count: 1,
                link_count: 0,
                primary_heading_level: None,
                primary_heading_count: 0,
                primary_heading_depth: None,
                utility_descendant_count: 0,
                score: 150,
            }),
        ];
        apply_nested_content_candidate_bias(&mut utility_heavy_outer);
        assert_eq!(utility_heavy_outer[0].score, 55);
        assert_eq!(utility_heavy_outer[1].score, 270);

        let mut primary_heading_count_outer = vec![
            ranked_candidate(CandidateFixture {
                path: "main:nth-of-type(1)",
                selector: "main",
                text_char_count: 1_000,
                heading_count: 2,
                link_count: 6,
                primary_heading_level: Some(1),
                primary_heading_count: 2,
                primary_heading_depth: Some(1),
                utility_descendant_count: 1,
                score: 400,
            }),
            ranked_candidate(CandidateFixture {
                path: "main:nth-of-type(1) > article:nth-of-type(1)",
                selector: "main > article",
                text_char_count: 900,
                heading_count: 2,
                link_count: 8,
                primary_heading_level: Some(1),
                primary_heading_count: 1,
                primary_heading_depth: Some(2),
                utility_descendant_count: 3,
                score: 400,
            }),
        ];
        apply_nested_content_candidate_bias(&mut primary_heading_count_outer);
        assert_eq!(primary_heading_count_outer[0].score, 485);
        assert_eq!(primary_heading_count_outer[1].score, 285);

        let mut too_short_inner = vec![
            ranked_candidate(CandidateFixture {
                path: "section:nth-of-type(1)",
                selector: "section",
                text_char_count: 1_000,
                heading_count: 1,
                link_count: 20,
                primary_heading_level: None,
                primary_heading_count: 0,
                primary_heading_depth: None,
                utility_descendant_count: 2,
                score: 175,
            }),
            ranked_candidate(CandidateFixture {
                path: "section:nth-of-type(1) > div:nth-of-type(1)",
                selector: "section > div",
                text_char_count: 500,
                heading_count: 1,
                link_count: 0,
                primary_heading_level: None,
                primary_heading_count: 0,
                primary_heading_depth: None,
                utility_descendant_count: 2,
                score: 175,
            }),
        ];
        apply_nested_content_candidate_bias(&mut too_short_inner);
        assert_eq!(too_short_inner[0].score, 175);
        assert_eq!(too_short_inner[1].score, 175);

        let mut link_heavy_inner = vec![
            ranked_candidate(CandidateFixture {
                path: "section:nth-of-type(1)",
                selector: "section",
                text_char_count: 1_000,
                heading_count: 1,
                link_count: 50,
                primary_heading_level: None,
                primary_heading_count: 0,
                primary_heading_depth: None,
                utility_descendant_count: 0,
                score: 220,
            }),
            ranked_candidate(CandidateFixture {
                path: "section:nth-of-type(1) > div:nth-of-type(1)",
                selector: "section > div",
                text_char_count: 900,
                heading_count: 1,
                link_count: 41,
                primary_heading_level: None,
                primary_heading_count: 0,
                primary_heading_depth: None,
                utility_descendant_count: 0,
                score: 220,
            }),
        ];
        apply_nested_content_candidate_bias(&mut link_heavy_inner);
        assert_eq!(link_heavy_inner[0].score, 220);
        assert_eq!(link_heavy_inner[1].score, 220);

        let mut shallow_heading_link_guard = vec![
            ranked_candidate(CandidateFixture {
                path: "article:nth-of-type(1)",
                selector: "article",
                text_char_count: 1_000,
                heading_count: 2,
                link_count: 90,
                primary_heading_level: Some(1),
                primary_heading_count: 1,
                primary_heading_depth: Some(1),
                utility_descendant_count: 20,
                score: 320,
            }),
            ranked_candidate(CandidateFixture {
                path: "article:nth-of-type(1) > div:nth-of-type(1)",
                selector: "article > div",
                text_char_count: 800,
                heading_count: 1,
                link_count: 5,
                primary_heading_level: None,
                primary_heading_count: 0,
                primary_heading_depth: None,
                utility_descendant_count: 0,
                score: 320,
            }),
        ];
        apply_nested_content_candidate_bias(&mut shallow_heading_link_guard);
        assert_eq!(shallow_heading_link_guard[0].score, 225);
        assert_eq!(shallow_heading_link_guard[1].score, 440);

        let mut extraction_primary_heading_guard = vec![
            ranked_candidate(CandidateFixture {
                path: "main:nth-of-type(1)",
                selector: "main",
                text_char_count: 1_000,
                heading_count: 3,
                link_count: 24,
                primary_heading_level: Some(1),
                primary_heading_count: 1,
                primary_heading_depth: Some(1),
                utility_descendant_count: 8,
                score: 300,
            }),
            ranked_candidate(CandidateFixture {
                path: "main:nth-of-type(1) > div:nth-of-type(1)",
                selector: "main > div",
                text_char_count: 950,
                heading_count: 2,
                link_count: 8,
                primary_heading_level: None,
                primary_heading_count: 0,
                primary_heading_depth: None,
                utility_descendant_count: 0,
                score: 300,
            }),
        ];
        apply_nested_content_candidate_bias_for(
            &mut extraction_primary_heading_guard,
            CandidatePreference::Extraction,
        );
        assert!(
            extraction_primary_heading_guard[0].score > extraction_primary_heading_guard[1].score
        );

        let mut primary_heading_utility_guard = vec![
            ranked_candidate(CandidateFixture {
                path: "main:nth-of-type(1)",
                selector: "main",
                text_char_count: 1_000,
                heading_count: 2,
                link_count: 6,
                primary_heading_level: Some(1),
                primary_heading_count: 2,
                primary_heading_depth: Some(1),
                utility_descendant_count: 12,
                score: 410,
            }),
            ranked_candidate(CandidateFixture {
                path: "main:nth-of-type(1) > article:nth-of-type(1)",
                selector: "main > article",
                text_char_count: 900,
                heading_count: 2,
                link_count: 8,
                primary_heading_level: Some(1),
                primary_heading_count: 1,
                primary_heading_depth: Some(2),
                utility_descendant_count: 1,
                score: 410,
            }),
        ];
        apply_nested_content_candidate_bias(&mut primary_heading_utility_guard);
        assert_eq!(primary_heading_utility_guard[0].score, 315);
        assert_eq!(primary_heading_utility_guard[1].score, 530);

        let mut heading_rich_link_guard = vec![
            ranked_candidate(CandidateFixture {
                path: "main:nth-of-type(1)",
                selector: "main",
                text_char_count: 1_000,
                heading_count: 6,
                link_count: 30,
                primary_heading_level: None,
                primary_heading_count: 0,
                primary_heading_depth: None,
                utility_descendant_count: 20,
                score: 250,
            }),
            ranked_candidate(CandidateFixture {
                path: "main:nth-of-type(1) > section:nth-of-type(1)",
                selector: "main > section",
                text_char_count: 850,
                heading_count: 1,
                link_count: 0,
                primary_heading_level: None,
                primary_heading_count: 0,
                primary_heading_depth: None,
                utility_descendant_count: 1,
                score: 250,
            }),
        ];
        apply_nested_content_candidate_bias(&mut heading_rich_link_guard);
        assert_eq!(heading_rich_link_guard[0].score, 250);
        assert_eq!(heading_rich_link_guard[1].score, 250);

        let mut heading_rich_utility_guard = vec![
            ranked_candidate(CandidateFixture {
                path: "main:nth-of-type(1)",
                selector: "main",
                text_char_count: 1_000,
                heading_count: 6,
                link_count: 10,
                primary_heading_level: None,
                primary_heading_count: 0,
                primary_heading_depth: None,
                utility_descendant_count: 20,
                score: 255,
            }),
            ranked_candidate(CandidateFixture {
                path: "main:nth-of-type(1) > section:nth-of-type(1)",
                selector: "main > section",
                text_char_count: 850,
                heading_count: 1,
                link_count: 0,
                primary_heading_level: None,
                primary_heading_count: 0,
                primary_heading_depth: None,
                utility_descendant_count: 1,
                score: 255,
            }),
        ];
        apply_nested_content_candidate_bias(&mut heading_rich_utility_guard);
        assert_eq!(heading_rich_utility_guard[0].score, 255);
        assert_eq!(heading_rich_utility_guard[1].score, 255);

        let mut modest_heading_utility_guard = vec![
            ranked_candidate(CandidateFixture {
                path: "div:nth-of-type(1)",
                selector: "div",
                text_char_count: 1_000,
                heading_count: 3,
                link_count: 2,
                primary_heading_level: None,
                primary_heading_count: 0,
                primary_heading_depth: None,
                utility_descendant_count: 8,
                score: 210,
            }),
            ranked_candidate(CandidateFixture {
                path: "div:nth-of-type(1) > section:nth-of-type(1)",
                selector: "div > section",
                text_char_count: 950,
                heading_count: 1,
                link_count: 3,
                primary_heading_level: None,
                primary_heading_count: 0,
                primary_heading_depth: None,
                utility_descendant_count: 1,
                score: 210,
            }),
        ];
        apply_nested_content_candidate_bias(&mut modest_heading_utility_guard);
        assert_eq!(modest_heading_utility_guard[0].score, 210);
        assert_eq!(modest_heading_utility_guard[1].score, 210);

        let mut modest_heading_link_guard = vec![
            ranked_candidate(CandidateFixture {
                path: "div:nth-of-type(1)",
                selector: "div",
                text_char_count: 1_000,
                heading_count: 3,
                link_count: 15,
                primary_heading_level: None,
                primary_heading_count: 0,
                primary_heading_depth: None,
                utility_descendant_count: 20,
                score: 215,
            }),
            ranked_candidate(CandidateFixture {
                path: "div:nth-of-type(1) > section:nth-of-type(1)",
                selector: "div > section",
                text_char_count: 950,
                heading_count: 1,
                link_count: 0,
                primary_heading_level: None,
                primary_heading_count: 0,
                primary_heading_depth: None,
                utility_descendant_count: 1,
                score: 215,
            }),
        ];
        apply_nested_content_candidate_bias(&mut modest_heading_link_guard);
        assert_eq!(modest_heading_link_guard[0].score, 120);
        assert_eq!(modest_heading_link_guard[1].score, 335);

        let dense_links_short = content_candidate_score(&ContentCandidateScoreInputs {
            tag_name: "aside",
            has_main_role: false,
            has_article_body_itemprop: false,
            text_char_count: 200,
            heading_count: 0,
            link_count: 9,
            paragraph_count: 1,
            positive_signal_count: 0,
            negative_signal_count: 0,
            primary_heading_level: None,
            primary_heading_count: 0,
            primary_heading_depth: None,
            utility_descendant_count: 0,
            uses_exact_path_selector: false,
        });
        let dense_links_medium = content_candidate_score(&ContentCandidateScoreInputs {
            tag_name: "section",
            has_main_role: false,
            has_article_body_itemprop: false,
            text_char_count: 1_000,
            heading_count: 1,
            link_count: 13,
            paragraph_count: 2,
            positive_signal_count: 0,
            negative_signal_count: 0,
            primary_heading_level: Some(3),
            primary_heading_count: 1,
            primary_heading_depth: Some(1),
            utility_descendant_count: 0,
            uses_exact_path_selector: false,
        });
        let dense_links_large = content_candidate_score(&ContentCandidateScoreInputs {
            tag_name: "div",
            has_main_role: false,
            has_article_body_itemprop: false,
            text_char_count: 3_000,
            heading_count: 1,
            link_count: 9,
            paragraph_count: 2,
            positive_signal_count: 0,
            negative_signal_count: 0,
            primary_heading_level: None,
            primary_heading_count: 0,
            primary_heading_depth: None,
            utility_descendant_count: 0,
            uses_exact_path_selector: false,
        });
        let dense_links_wide = content_candidate_score(&ContentCandidateScoreInputs {
            tag_name: "div",
            has_main_role: false,
            has_article_body_itemprop: false,
            text_char_count: 5_000,
            heading_count: 1,
            link_count: 7,
            paragraph_count: 2,
            positive_signal_count: 0,
            negative_signal_count: 0,
            primary_heading_level: None,
            primary_heading_count: 0,
            primary_heading_depth: None,
            utility_descendant_count: 0,
            uses_exact_path_selector: false,
        });
        let bodyless = content_candidate_score(&ContentCandidateScoreInputs {
            tag_name: "section",
            has_main_role: false,
            has_article_body_itemprop: false,
            text_char_count: 100,
            heading_count: 0,
            link_count: 0,
            paragraph_count: 0,
            positive_signal_count: 0,
            negative_signal_count: 0,
            primary_heading_level: None,
            primary_heading_count: 0,
            primary_heading_depth: None,
            utility_descendant_count: 0,
            uses_exact_path_selector: false,
        });
        let near_bodyless = content_candidate_score(&ContentCandidateScoreInputs {
            tag_name: "section",
            has_main_role: false,
            has_article_body_itemprop: false,
            text_char_count: 200,
            heading_count: 0,
            link_count: 0,
            paragraph_count: 1,
            positive_signal_count: 0,
            negative_signal_count: 0,
            primary_heading_level: None,
            primary_heading_count: 0,
            primary_heading_depth: None,
            utility_descendant_count: 0,
            uses_exact_path_selector: false,
        });
        let bodyless_but_long = content_candidate_score(&ContentCandidateScoreInputs {
            tag_name: "section",
            has_main_role: false,
            has_article_body_itemprop: false,
            text_char_count: 520,
            heading_count: 0,
            link_count: 0,
            paragraph_count: 0,
            positive_signal_count: 0,
            negative_signal_count: 0,
            primary_heading_level: None,
            primary_heading_count: 0,
            primary_heading_depth: None,
            utility_descendant_count: 0,
            uses_exact_path_selector: false,
        });
        let near_bodyless_but_long = content_candidate_score(&ContentCandidateScoreInputs {
            tag_name: "section",
            has_main_role: false,
            has_article_body_itemprop: false,
            text_char_count: 400,
            heading_count: 0,
            link_count: 0,
            paragraph_count: 1,
            positive_signal_count: 0,
            negative_signal_count: 0,
            primary_heading_level: None,
            primary_heading_count: 0,
            primary_heading_depth: None,
            utility_descendant_count: 0,
            uses_exact_path_selector: false,
        });
        let dense_links_medium_guard = content_candidate_score(&ContentCandidateScoreInputs {
            tag_name: "section",
            has_main_role: false,
            has_article_body_itemprop: false,
            text_char_count: 1_400,
            heading_count: 1,
            link_count: 13,
            paragraph_count: 2,
            positive_signal_count: 0,
            negative_signal_count: 0,
            primary_heading_level: None,
            primary_heading_count: 0,
            primary_heading_depth: None,
            utility_descendant_count: 0,
            uses_exact_path_selector: false,
        });
        let dense_links_wide_guard = content_candidate_score(&ContentCandidateScoreInputs {
            tag_name: "section",
            has_main_role: false,
            has_article_body_itemprop: false,
            text_char_count: 6_200,
            heading_count: 1,
            link_count: 7,
            paragraph_count: 2,
            positive_signal_count: 0,
            negative_signal_count: 0,
            primary_heading_level: None,
            primary_heading_count: 0,
            primary_heading_depth: None,
            utility_descendant_count: 0,
            uses_exact_path_selector: false,
        });
        let dense_links_medium_guard_baseline =
            content_candidate_score(&ContentCandidateScoreInputs {
                tag_name: "section",
                has_main_role: false,
                has_article_body_itemprop: false,
                text_char_count: 1_400,
                heading_count: 1,
                link_count: 0,
                paragraph_count: 2,
                positive_signal_count: 0,
                negative_signal_count: 0,
                primary_heading_level: None,
                primary_heading_count: 0,
                primary_heading_depth: None,
                utility_descendant_count: 0,
                uses_exact_path_selector: false,
            });
        assert!(dense_links_short < dense_links_medium);
        assert!(dense_links_large != dense_links_wide);
        assert!(bodyless < near_bodyless);
        assert!(bodyless < bodyless_but_long);
        assert!(near_bodyless < near_bodyless_but_long);
        assert!(dense_links_medium_guard < dense_links_medium_guard_baseline);
        assert!(dense_links_wide_guard > dense_links_wide);
        assert_eq!(primary_heading_bonus(1), 130);
        assert_eq!(primary_heading_bonus(2), 78);
        assert_eq!(primary_heading_bonus(3), 20);
        assert_eq!(primary_heading_bonus(9), 0);
        assert!(has_shallow_primary_heading(Some(1), Some(5)));
        assert!(!has_shallow_primary_heading(Some(1), Some(6)));
        assert!(has_shallow_primary_heading(Some(2), Some(2)));
        assert!(!has_shallow_primary_heading(Some(2), Some(3)));
        assert!(drops_outer_title_signal(Some(1), Some(4), Some(2), Some(1)));
        assert!(!drops_outer_title_signal(
            Some(1),
            Some(4),
            Some(1),
            Some(3)
        ));
        assert!(drops_outer_title_signal(Some(2), Some(2), None, None));
        assert!(!drops_outer_title_signal(
            Some(2),
            Some(2),
            Some(1),
            Some(2)
        ));
    }

    #[test]
    fn selector_and_sampling_helpers_cover_remaining_branches() {
        let empty_document = Html::new_document();
        assert_eq!(
            build_document_inspection(&empty_document, None, 1).root_tag,
            "html"
        );

        let fragment = Html::parse_fragment(
            "<section class=\"fragment-box\"><form></form><script></script><style></style><table></table><img src=\"hero.png\"><a href=\"/guide\">Guide</a></section>",
        );
        let fragment_inspection = build_document_inspection(&fragment, None, 1);
        assert_eq!(fragment_inspection.root_tag, "html");
        assert_eq!(fragment_inspection.form_count, 1);
        assert_eq!(fragment_inspection.script_count, 1);
        assert_eq!(fragment_inspection.style_count, 1);
        assert_eq!(fragment_inspection.table_count, 1);
        assert_eq!(fragment_inspection.image_count, 1);
        assert_eq!(fragment_inspection.link_count, 1);

        let document = parse_document_node(
            "<html><body>\
                <div class=\"content\"></div>\
                <section id=\"heading-scope\"></section>\
                <section id=\"link-scope\"></section>\
                <h2>Fallback Heading</h2>\
                <a href=\"/fallback\">Fallback Link</a>\
                <main id=\"main-content\" role=\"main\" itemprop=\"articleBody\" class=\"content story main\">\
                    <h1>Main Title</h1>\
                    <p>Alpha beta gamma delta epsilon zeta eta theta iota kappa lambda mu.</p>\
                    <a href=\"/guide\">Guide</a>\
                    <nav class=\"tools\"><a href=\"/edit\">Edit</a></nav>\
                </main>\
                <section class=\"story feature\">\
                    <h2>Feature Title</h2>\
                    <p>Support body text for selector testing.</p>\
                </section>\
                <section class=\"story feature duplicate\">\
                    <h2>Feature Title Two</h2>\
                    <p>Support body text for selector testing.</p>\
                </section>\
                <a href=\"/empty\"><img src=\"hero.png\" alt=\"\"></a>\
            </body></html>",
        );

        assert!(build_content_candidates(&document, 0).is_empty());
        let empty_role_main =
            parse_document_node("<html><body><div role=\"main\"></div></body></html>");
        assert!(build_content_candidates(&empty_role_main, 3).is_empty());
        let candidates = build_content_candidates(&document, 5);
        assert!(
            candidates
                .iter()
                .all(|candidate| candidate.text_char_count > 0)
        );
        let many_candidates_document = parse_document_node(
            "<html><body>\
                <section class=\"content feature-one\"><h2>One</h2><p>Alpha beta gamma delta epsilon zeta eta theta iota kappa lambda mu nu xi omicron.</p></section>\
                <section class=\"content feature-two\"><h2>Two</h2><p>Alpha beta gamma delta epsilon zeta eta theta iota kappa lambda mu nu xi omicron.</p></section>\
                <section class=\"content feature-three\"><h2>Three</h2><p>Alpha beta gamma delta epsilon zeta eta theta iota kappa lambda mu nu xi omicron.</p></section>\
                <section class=\"content feature-four\"><h2>Four</h2><p>Alpha beta gamma delta epsilon zeta eta theta iota kappa lambda mu nu xi omicron.</p></section>\
                <section class=\"content feature-five\"><h2>Five</h2><p>Alpha beta gamma delta epsilon zeta eta theta iota kappa lambda mu nu xi omicron.</p></section>\
                <section class=\"content feature-six\"><h2>Six</h2><p>Alpha beta gamma delta epsilon zeta eta theta iota kappa lambda mu nu xi omicron.</p></section>\
            </body></html>",
        );
        assert_eq!(
            build_content_candidates(&many_candidates_document, 8).len(),
            6
        );

        let heading_scope =
            build_node_path(&select_first(&document, "#heading-scope").expect("scope"));
        assert!(
            build_heading_samples(&document, 0, std::slice::from_ref(&heading_scope)).is_empty()
        );
        let headings = build_heading_samples(&document, 3, std::slice::from_ref(&heading_scope));
        assert_eq!(headings[0].text, "Fallback Heading");
        let main_scope = build_node_path(&select_first(&document, "#main-content").expect("main"));
        let main_headings = build_heading_samples(&document, 3, std::slice::from_ref(&main_scope));
        assert_eq!(main_headings[0].text, "Main Title");
        let feature_scope =
            build_node_path(&select_first(&document, "section.feature").expect("first feature"));
        let duplicate_feature_scope = build_node_path(
            &select_first(&document, "section.feature.duplicate").expect("duplicate feature"),
        );
        let combined_headings = build_heading_samples(
            &document,
            3,
            &[main_scope.clone(), feature_scope, duplicate_feature_scope],
        );
        assert_eq!(
            combined_headings
                .iter()
                .map(|heading| heading.text.as_str())
                .collect::<Vec<_>>(),
            vec!["Main Title"]
        );

        let heading_selector = Selector::parse("h2").expect("heading selector");
        let mut seen_heading_paths = BTreeSet::new();
        let duplicate_heading_document =
            parse_document_node("<section><h2>Only Heading</h2></section>");
        let duplicate_heading = select_first(&duplicate_heading_document, "h2").expect("heading");
        seen_heading_paths.insert(build_node_path(&duplicate_heading));
        assert!(
            sample_headings_from_scope(
                &duplicate_heading_document,
                None,
                3,
                &heading_selector,
                &mut seen_heading_paths,
            )
            .is_empty()
        );
        let mixed_heading_selector = Selector::parse("div, h2").expect("mixed selector");
        let mixed_heading_document =
            parse_document_node("<section><div>Ignore</div><h2>Only Heading</h2></section>");
        let mixed_headings = sample_headings_from_scope(
            &mixed_heading_document,
            None,
            3,
            &mixed_heading_selector,
            &mut BTreeSet::new(),
        );
        assert_eq!(mixed_headings.len(), 1);
        assert_eq!(mixed_headings[0].text, "Only Heading");
        let utility_heading_document = parse_document_node(
            "<section><nav><h2>Ignore Utility</h2></nav><h2>Keep Heading</h2></section>",
        );
        let utility_headings = sample_headings_from_scope(
            &utility_heading_document,
            None,
            3,
            &heading_selector,
            &mut BTreeSet::new(),
        );
        assert_eq!(utility_headings.len(), 1);
        assert_eq!(utility_headings[0].text, "Keep Heading");
        let heading_element_utility_document = parse_document_node(
            "<section><h2 class=\"editsection\">Ignore Utility</h2><h2>Keep</h2></section>",
        );
        let utility_element_headings = sample_headings_from_scope(
            &heading_element_utility_document,
            None,
            3,
            &heading_selector,
            &mut BTreeSet::new(),
        );
        assert_eq!(utility_element_headings.len(), 1);
        assert_eq!(utility_element_headings[0].text, "Keep");

        let link_scope = build_node_path(&select_first(&document, "#link-scope").expect("scope"));
        assert!(
            build_link_samples(&document, Some("https://example.test/base/"), 0, &[]).is_empty()
        );
        let links = build_link_samples(
            &document,
            Some("https://example.test/base/"),
            3,
            std::slice::from_ref(&link_scope),
        );
        assert_eq!(links[0].href.as_deref(), Some("/fallback"));
        let main_links = build_link_samples(
            &document,
            Some("https://example.test/base/"),
            3,
            std::slice::from_ref(&main_scope),
        );
        assert_eq!(main_links[0].href.as_deref(), Some("/guide"));
        let dual_scope_document = parse_document_node(
            "<html><body>\
                <section class=\"content primary\"><a href=\"/first\">First</a></section>\
                <section class=\"content secondary\"><a href=\"/second\">Second</a></section>\
            </body></html>",
        );
        let primary_scope = build_node_path(
            &select_first(&dual_scope_document, "section.primary").expect("primary section"),
        );
        let secondary_scope = build_node_path(
            &select_first(&dual_scope_document, "section.secondary").expect("secondary section"),
        );
        let combined_links = build_link_samples(
            &dual_scope_document,
            Some("https://example.test/base/"),
            3,
            &[primary_scope, secondary_scope],
        );
        assert_eq!(combined_links[0].href.as_deref(), Some("/first"));

        let link_selector = Selector::parse("a").expect("link selector");
        let mut seen_link_paths = BTreeSet::new();
        let guide = select_first(&document, "main a[href=\"/guide\"]").expect("guide link");
        seen_link_paths.insert(build_node_path(&guide));
        assert!(
            sample_links_from_scope(
                &document,
                Some("https://example.test/base/"),
                Some(&build_node_path(
                    &select_first(&document, "#main-content").expect("main")
                )),
                3,
                &link_selector,
                &mut seen_link_paths,
            )
            .is_empty()
        );
        assert!(
            sample_links_from_scope(
                &document,
                Some("https://example.test/base/"),
                Some(&build_node_path(
                    &select_first(&document, "#main-content").expect("main")
                )),
                3,
                &link_selector,
                &mut BTreeSet::new(),
            )
            .iter()
            .any(|link| link.href.as_deref() == Some("/guide"))
        );
        assert!(
            sample_links_from_scope(
                &document,
                Some("https://example.test/base/"),
                None,
                10,
                &link_selector,
                &mut BTreeSet::new(),
            )
            .iter()
            .all(|link| !link.text.is_empty())
        );
        let utility_link_document = parse_document_node(
            "<section><nav><a href=\"/ignore\">Ignore</a></nav><a href=\"/keep\">Keep</a></section>",
        );
        let utility_links = sample_links_from_scope(
            &utility_link_document,
            Some("https://example.test/base/"),
            None,
            3,
            &link_selector,
            &mut BTreeSet::new(),
        );
        assert_eq!(utility_links.len(), 1);
        assert_eq!(utility_links[0].href.as_deref(), Some("/keep"));
        let utility_link_element_document = parse_document_node(
            "<section><a class=\"editsection\" href=\"/ignore\">Ignore</a><a href=\"/keep\">Keep</a></section>",
        );
        let utility_element_links = sample_links_from_scope(
            &utility_link_element_document,
            Some("https://example.test/base/"),
            None,
            3,
            &link_selector,
            &mut BTreeSet::new(),
        );
        assert_eq!(utility_element_links.len(), 1);
        assert_eq!(utility_element_links[0].href.as_deref(), Some("/keep"));
        let empty_text_link_document = parse_document_node(
            "<section><a href=\"/image-only\"><img alt=\"\" src=\"hero.png\"></a></section>",
        );
        assert!(
            sample_links_from_scope(
                &empty_text_link_document,
                Some("https://example.test/base/"),
                None,
                3,
                &link_selector,
                &mut BTreeSet::new(),
            )
            .is_empty()
        );
        let same_page_link_document = parse_document_node(
            "<article><a href=\"https://example.test/guide#overview\">Overview</a><a href=\"/next\">Next</a></article>",
        );
        let same_page_links = sample_links_from_scope(
            &same_page_link_document,
            Some("https://example.test/guide"),
            None,
            4,
            &link_selector,
            &mut BTreeSet::new(),
        );
        assert_eq!(same_page_links.len(), 1);
        assert_eq!(same_page_links[0].href.as_deref(), Some("/next"));
        let meaningful_link_document = parse_document_node(
            "<section>\
                <nav><a href=\"/ignore\">Ignore</a></nav>\
                <a href=\"https://example.test/guide#overview\">Overview</a>\
                <a href=\"/terms\">Terms apply</a>\
                <a href=\"/article\">Article link</a>\
                <a href=\"/image-only\"><img alt=\"\" src=\"hero.png\"></a>\
            </section>",
        );
        assert_eq!(
            count_meaningful_links(
                &select_first(&meaningful_link_document, "section").expect("section"),
                &link_selector,
            ),
            2
        );
        let utility_count_document = parse_document_node(
            "<section><a class=\"editsection\" href=\"/ignore\">Ignore</a><a href=\"/article\">Article link</a></section>",
        );
        assert_eq!(
            count_meaningful_links(
                &select_first(&utility_count_document, "section").expect("section"),
                &link_selector,
            ),
            1
        );

        assert_eq!(
            select_elements_in_scope(&document, Some("missing-scope"), &link_selector).count(),
            document.select(&link_selector).count()
        );

        let feature = select_first(&document, "section.feature").expect("feature");
        let feature_path = build_node_path(&feature);
        let feature_candidates = selector_candidates_for_element(&feature, &feature_path);
        assert!(feature_candidates.contains(&"section.story.feature".to_owned()));
        assert!(feature_candidates.contains(&"section".to_owned()));

        let main = select_first(&document, "#main-content").expect("main");
        let main_path = build_node_path(&main);
        let main_candidates = selector_candidates_for_element(&main, &main_path);
        assert!(main_candidates.contains(&"#main-content".to_owned()));
        assert!(main_candidates.contains(&"main[role=\"main\"]".to_owned()));
        assert!(main_candidates.contains(&"[role=\"main\"]".to_owned()));
        assert!(main_candidates.contains(&"main[itemprop=\"articleBody\"]".to_owned()));
        assert_eq!(
            recommend_content_selector(&document, &feature, &feature_path),
            feature_path
        );
        assert_eq!(
            recommend_content_selector(&document, &main, &main_path),
            "#main-content"
        );
        let plain_document = parse_document_node(
            "<html><body><section><p>One</p></section><section><p>Two</p></section></body></html>",
        );
        let plain_section = select_first(&plain_document, "section").expect("plain section");
        let plain_path = build_node_path(&plain_section);
        assert_eq!(
            recommend_content_selector(&plain_document, &plain_section, &plain_path),
            plain_path
        );

        assert!(selector_uniquely_matches(
            &document,
            "#main-content",
            main.id()
        ));
        assert!(!selector_uniquely_matches(
            &document,
            "section",
            feature.id()
        ));
        assert!(!selector_uniquely_matches(
            &document,
            "section[",
            feature.id()
        ));
        assert!(!selector_uniquely_matches(
            &document,
            "#missing",
            feature.id()
        ));
        let invalid_id_document = parse_document_node("<div id=\"9 hero\"></div>");
        let invalid_id = select_first(&invalid_id_document, "div").expect("invalid id");
        assert_eq!(id_selector("9 hero"), "[id=\"9 hero\"]");
        assert!(
            selector_candidates_for_element(&invalid_id, "div:nth-of-type(1)")
                .contains(&"[id=\"9 hero\"]".to_owned())
        );
        let blank_metadata_document = parse_document_node(
            "<div id=\"  \" role=\"  \" itemprop=\"  \" class=\"content hero\"></div>",
        );
        let blank_metadata = select_first(&blank_metadata_document, "div").expect("blank metadata");
        let blank_candidates =
            selector_candidates_for_element(&blank_metadata, "div:nth-of-type(1)");
        assert!(
            !blank_candidates
                .iter()
                .any(|candidate| candidate.starts_with('#'))
        );
        assert!(
            !blank_candidates
                .iter()
                .any(|candidate| candidate.contains("[role="))
        );
        assert!(
            !blank_candidates
                .iter()
                .any(|candidate| candidate.contains("[itemprop="))
        );
        assert!(blank_candidates.contains(&"div.hero".to_owned()));

        assert!(!simple_css_identifier(""));
        assert!(!simple_css_identifier("9feature"));
        assert!(!simple_css_identifier("feature!"));
        assert!(simple_css_identifier("_feature1"));
        assert!(simple_css_identifier("-feature"));
        assert!(simple_css_identifier("feature-card"));
        assert_eq!(css_string_literal("a\\\"b"), "a\\\\\\\"b");

        let role_and_itemprop_document = parse_document_node(
            "<div role=\"main\"></div><section itemprop=\"articleBody\"></section>",
        );
        assert!(element_attr_equals_ignore_ascii_case(
            &select_first(&role_and_itemprop_document, "div").expect("role main"),
            "role",
            "main",
        ));
        assert!(!element_attr_equals_ignore_ascii_case(
            &select_first(&role_and_itemprop_document, "div").expect("role main"),
            "itemprop",
            "articleBody",
        ));
        assert!(is_content_candidate_container(
            &select_first(&role_and_itemprop_document, "div").expect("role main"),
            0,
        ));
        assert!(is_content_candidate_container(
            &select_first(&role_and_itemprop_document, "section").expect("article body"),
            0,
        ));
        assert!(element_attr_equals_ignore_ascii_case(
            &select_first(&role_and_itemprop_document, "section").expect("article body"),
            "itemprop",
            "articleBody",
        ));
        assert!(!element_attr_equals_ignore_ascii_case(
            &select_first(&role_and_itemprop_document, "section").expect("article body"),
            "role",
            "main",
        ));
        let narrative_section_document = parse_document_node(
            "<section><h2>Design</h2><p>All-screen front.</p><p>Durable body.</p><ul><li>Feature one</li><li>Feature two</li></ul></section>",
        );
        let narrative_section =
            select_first(&narrative_section_document, "section").expect("narrative section");
        assert!(element_has_narrative_section_shape(&narrative_section));
        assert!(is_content_candidate_container(&narrative_section, 0));
        let shallow_section_document =
            parse_document_node("<section><h2>Design</h2><p>Single paragraph.</p></section>");
        let shallow_section =
            select_first(&shallow_section_document, "section").expect("shallow section");
        assert!(!element_has_narrative_section_shape(&shallow_section));
        assert!(!is_content_candidate_container(&shallow_section, 0));

        assert_eq!(descendant_element_depth(&main, &main), Some(0));
        let main_heading = select_first(&document, "#main-content h1").expect("heading");
        assert_eq!(descendant_element_depth(&main, &main_heading), Some(1));
        let fallback_heading = select_first(&document, "body > h2").expect("fallback heading");
        assert_eq!(descendant_element_depth(&main, &fallback_heading), None);

        assert_eq!(count_utility_descendant_roots(&main), 1);
        let nav = select_first(&document, "nav.tools").expect("nav");
        assert!(!has_utility_chrome_ancestor_before(&nav, main.id()));
        let nav_link = select_first(&document, "nav.tools a").expect("nav link");
        assert!(has_utility_chrome_ancestor_before(&nav_link, main.id()));
        assert!(!has_utility_chrome_ancestor_before(
            &fallback_heading,
            main.id()
        ));

        let alpha_tiebreak = ranked_candidate(CandidateFixture {
            path: "alpha-path",
            selector: "#alpha",
            score: 40,
            text_char_count: 200,
            heading_count: 2,
            link_count: 0,
            primary_heading_level: None,
            primary_heading_count: 0,
            primary_heading_depth: None,
            utility_descendant_count: 0,
        });
        let beta_tiebreak = ranked_candidate(CandidateFixture {
            path: "beta-path",
            selector: "#beta",
            score: 40,
            text_char_count: 200,
            heading_count: 2,
            link_count: 0,
            primary_heading_level: None,
            primary_heading_count: 0,
            primary_heading_depth: None,
            utility_descendant_count: 0,
        });
        assert_eq!(
            compare_content_candidates(&alpha_tiebreak, &beta_tiebreak),
            Ordering::Less
        );

        let medium_bodyless = content_candidate_score(&ContentCandidateScoreInputs {
            tag_name: "section",
            has_main_role: false,
            has_article_body_itemprop: false,
            text_char_count: 200,
            heading_count: 0,
            link_count: 0,
            paragraph_count: 1,
            positive_signal_count: 0,
            negative_signal_count: 0,
            primary_heading_level: None,
            primary_heading_count: 0,
            primary_heading_depth: None,
            utility_descendant_count: 0,
            uses_exact_path_selector: false,
        });
        let dense_medium_links = content_candidate_score(&ContentCandidateScoreInputs {
            tag_name: "section",
            has_main_role: false,
            has_article_body_itemprop: false,
            text_char_count: 900,
            heading_count: 1,
            link_count: 14,
            paragraph_count: 2,
            positive_signal_count: 0,
            negative_signal_count: 0,
            primary_heading_level: None,
            primary_heading_count: 0,
            primary_heading_depth: None,
            utility_descendant_count: 0,
            uses_exact_path_selector: false,
        });
        let dense_large_links = content_candidate_score(&ContentCandidateScoreInputs {
            tag_name: "section",
            has_main_role: false,
            has_article_body_itemprop: false,
            text_char_count: 3_000,
            heading_count: 1,
            link_count: 12,
            paragraph_count: 2,
            positive_signal_count: 0,
            negative_signal_count: 0,
            primary_heading_level: None,
            primary_heading_count: 0,
            primary_heading_depth: None,
            utility_descendant_count: 0,
            uses_exact_path_selector: false,
        });
        let dense_wide_links = content_candidate_score(&ContentCandidateScoreInputs {
            tag_name: "section",
            has_main_role: false,
            has_article_body_itemprop: false,
            text_char_count: 5_000,
            heading_count: 1,
            link_count: 10,
            paragraph_count: 2,
            positive_signal_count: 0,
            negative_signal_count: 0,
            primary_heading_level: None,
            primary_heading_count: 0,
            primary_heading_depth: None,
            utility_descendant_count: 0,
            uses_exact_path_selector: false,
        });
        let supportive_body = content_candidate_score(&ContentCandidateScoreInputs {
            tag_name: "section",
            has_main_role: false,
            has_article_body_itemprop: false,
            text_char_count: 260,
            heading_count: 1,
            link_count: 0,
            paragraph_count: 2,
            positive_signal_count: 0,
            negative_signal_count: 0,
            primary_heading_level: None,
            primary_heading_count: 0,
            primary_heading_depth: None,
            utility_descendant_count: 0,
            uses_exact_path_selector: false,
        });
        let dense_medium_baseline = content_candidate_score(&ContentCandidateScoreInputs {
            tag_name: "section",
            has_main_role: false,
            has_article_body_itemprop: false,
            text_char_count: 900,
            heading_count: 1,
            link_count: 0,
            paragraph_count: 2,
            positive_signal_count: 0,
            negative_signal_count: 0,
            primary_heading_level: None,
            primary_heading_count: 0,
            primary_heading_depth: None,
            utility_descendant_count: 0,
            uses_exact_path_selector: false,
        });
        let dense_large_baseline = content_candidate_score(&ContentCandidateScoreInputs {
            tag_name: "section",
            has_main_role: false,
            has_article_body_itemprop: false,
            text_char_count: 3_000,
            heading_count: 1,
            link_count: 0,
            paragraph_count: 2,
            positive_signal_count: 0,
            negative_signal_count: 0,
            primary_heading_level: None,
            primary_heading_count: 0,
            primary_heading_depth: None,
            utility_descendant_count: 0,
            uses_exact_path_selector: false,
        });
        let dense_wide_baseline = content_candidate_score(&ContentCandidateScoreInputs {
            tag_name: "section",
            has_main_role: false,
            has_article_body_itemprop: false,
            text_char_count: 5_000,
            heading_count: 1,
            link_count: 0,
            paragraph_count: 2,
            positive_signal_count: 0,
            negative_signal_count: 0,
            primary_heading_level: None,
            primary_heading_count: 0,
            primary_heading_depth: None,
            utility_descendant_count: 0,
            uses_exact_path_selector: false,
        });
        assert!(medium_bodyless < supportive_body);
        assert!(dense_medium_links < dense_medium_baseline);
        assert!(dense_large_links < dense_large_baseline);
        assert!(dense_wide_links < dense_wide_baseline);
    }

    #[test]
    fn precise_reading_descendant_promotion_prefers_near_full_article_descendants() {
        let mut extraction_candidates = vec![
            ranked_content_candidate(PromotionFixture {
                selector: "#content",
                path: "html > body > main#content",
                tag_name: "main",
                text_char_count: 1_000,
                heading_count: 3,
                link_count: 8,
                primary_heading_level: Some(1),
                primary_heading_depth: Some(1),
            }),
            ranked_content_candidate(PromotionFixture {
                selector: "main#content > section.tools",
                path: "html > body > main#content > section.tools",
                tag_name: "section",
                text_char_count: 120,
                heading_count: 1,
                link_count: 6,
                primary_heading_level: None,
                primary_heading_depth: None,
            }),
        ];
        let reading_candidates = vec![
            extraction_candidates[0].clone(),
            ranked_content_candidate(PromotionFixture {
                selector: "article.article-body",
                path: "html > body > main#content > article.article-body",
                tag_name: "article",
                text_char_count: 950,
                heading_count: 3,
                link_count: 5,
                primary_heading_level: Some(1),
                primary_heading_depth: Some(2),
            }),
            ranked_content_candidate(PromotionFixture {
                selector: "section.related-links",
                path: "html > body > main#content > section.related-links",
                tag_name: "section",
                text_char_count: 940,
                heading_count: 2,
                link_count: 8,
                primary_heading_level: None,
                primary_heading_depth: None,
            }),
        ];

        promote_precise_reading_descendant_candidate(
            &mut extraction_candidates,
            &reading_candidates,
        );

        assert_eq!(
            extraction_candidates[0].inspection.selector,
            "article.article-body"
        );
        assert_eq!(
            extraction_candidates[0].inspection.path,
            "html > body > main#content > article.article-body"
        );
    }

    #[test]
    fn precise_reading_descendant_promotion_prefers_fewer_links_when_candidates_are_tied() {
        let mut extraction_candidates = vec![ranked_content_candidate(PromotionFixture {
            selector: "#content",
            path: "html > body > main#content",
            tag_name: "main",
            text_char_count: 1_000,
            heading_count: 3,
            link_count: 8,
            primary_heading_level: Some(1),
            primary_heading_depth: Some(1),
        })];
        let reading_candidates = vec![
            extraction_candidates[0].clone(),
            ranked_content_candidate(PromotionFixture {
                selector: "article.feature-a",
                path: "html > body > main#content > article.feature-a",
                tag_name: "article",
                text_char_count: 930,
                heading_count: 3,
                link_count: 6,
                primary_heading_level: Some(1),
                primary_heading_depth: Some(2),
            }),
            ranked_content_candidate(PromotionFixture {
                selector: "article.feature-b",
                path: "html > body > main#content > article.feature-b",
                tag_name: "article",
                text_char_count: 930,
                heading_count: 3,
                link_count: 4,
                primary_heading_level: Some(1),
                primary_heading_depth: Some(2),
            }),
        ];

        promote_precise_reading_descendant_candidate(
            &mut extraction_candidates,
            &reading_candidates,
        );

        assert_eq!(
            extraction_candidates[0].inspection.selector,
            "article.feature-b"
        );
    }

    #[test]
    fn precise_reading_descendant_promotion_rejects_descendants_that_drop_too_much_content() {
        let mut extraction_candidates = vec![ranked_content_candidate(PromotionFixture {
            selector: "#content",
            path: "html > body > main#content",
            tag_name: "main",
            text_char_count: 1_000,
            heading_count: 4,
            link_count: 8,
            primary_heading_level: Some(1),
            primary_heading_depth: Some(1),
        })];
        let reading_candidates = vec![
            extraction_candidates[0].clone(),
            ranked_content_candidate(PromotionFixture {
                selector: "article.short-fragment",
                path: "html > body > main#content > article.short-fragment",
                tag_name: "article",
                text_char_count: 800,
                heading_count: 4,
                link_count: 2,
                primary_heading_level: Some(1),
                primary_heading_depth: Some(2),
            }),
            ranked_content_candidate(PromotionFixture {
                selector: "article.too-few-headings",
                path: "html > body > main#content > article.too-few-headings",
                tag_name: "article",
                text_char_count: 980,
                heading_count: 1,
                link_count: 2,
                primary_heading_level: Some(2),
                primary_heading_depth: Some(2),
            }),
        ];

        promote_precise_reading_descendant_candidate(
            &mut extraction_candidates,
            &reading_candidates,
        );

        assert_eq!(extraction_candidates[0].inspection.selector, "#content");
    }

    #[test]
    fn extraction_specific_nested_bias_and_ordering_helpers_cover_remaining_paths() {
        let mut near_full_inner = vec![
            ranked_bias_candidate(BiasFixture {
                selector: "main",
                path: "html > body > main",
                tag_name: "main",
                text_char_count: 1_000,
                heading_count: 3,
                link_count: 18,
                paragraph_count: 3,
                primary_heading_level: None,
                primary_heading_count: 0,
                primary_heading_depth: None,
                utility_descendant_count: 0,
                score: 200,
            }),
            ranked_bias_candidate(BiasFixture {
                selector: "main > article",
                path: "html > body > main > article",
                tag_name: "article",
                text_char_count: 930,
                heading_count: 2,
                link_count: 6,
                paragraph_count: 3,
                primary_heading_level: None,
                primary_heading_count: 0,
                primary_heading_depth: None,
                utility_descendant_count: 0,
                score: 200,
            }),
        ];
        apply_nested_content_candidate_bias_for(
            &mut near_full_inner,
            CandidatePreference::Extraction,
        );
        assert!(near_full_inner[1].score > near_full_inner[0].score);

        let mut utility_driven_inner = vec![
            ranked_bias_candidate(BiasFixture {
                selector: "main",
                path: "html > body > main",
                tag_name: "main",
                text_char_count: 1_000,
                heading_count: 3,
                link_count: 9,
                paragraph_count: 3,
                primary_heading_level: None,
                primary_heading_count: 0,
                primary_heading_depth: None,
                utility_descendant_count: 4,
                score: 200,
            }),
            ranked_bias_candidate(BiasFixture {
                selector: "main > article",
                path: "html > body > main > article",
                tag_name: "article",
                text_char_count: 930,
                heading_count: 2,
                link_count: 6,
                paragraph_count: 3,
                primary_heading_level: None,
                primary_heading_count: 0,
                primary_heading_depth: None,
                utility_descendant_count: 0,
                score: 200,
            }),
        ];
        apply_nested_content_candidate_bias_for(
            &mut utility_driven_inner,
            CandidatePreference::Extraction,
        );
        assert!(utility_driven_inner[1].score > utility_driven_inner[0].score);

        let mut overwhelmingly_large_outer = vec![
            ranked_bias_candidate(BiasFixture {
                selector: "main",
                path: "html > body > main",
                tag_name: "main",
                text_char_count: 1_200,
                heading_count: 6,
                link_count: 2,
                paragraph_count: 6,
                primary_heading_level: None,
                primary_heading_count: 0,
                primary_heading_depth: None,
                utility_descendant_count: 0,
                score: 120,
            }),
            ranked_bias_candidate(BiasFixture {
                selector: "main > section",
                path: "html > body > main > section",
                tag_name: "section",
                text_char_count: 150,
                heading_count: 1,
                link_count: 0,
                paragraph_count: 1,
                primary_heading_level: None,
                primary_heading_count: 0,
                primary_heading_depth: None,
                utility_descendant_count: 0,
                score: 120,
            }),
        ];
        apply_nested_content_candidate_bias_for(
            &mut overwhelmingly_large_outer,
            CandidatePreference::Extraction,
        );
        assert!(overwhelmingly_large_outer[0].score > overwhelmingly_large_outer[1].score);

        let mut utility_heavy_outer = vec![
            ranked_bias_candidate(BiasFixture {
                selector: "main",
                path: "html > body > main",
                tag_name: "main",
                text_char_count: 1_000,
                heading_count: 2,
                link_count: 3,
                paragraph_count: 2,
                primary_heading_level: None,
                primary_heading_count: 0,
                primary_heading_depth: None,
                utility_descendant_count: 10,
                score: 80,
            }),
            ranked_bias_candidate(BiasFixture {
                selector: "main > article",
                path: "html > body > main > article",
                tag_name: "article",
                text_char_count: 800,
                heading_count: 2,
                link_count: 0,
                paragraph_count: 2,
                primary_heading_level: None,
                primary_heading_count: 0,
                primary_heading_depth: None,
                utility_descendant_count: 0,
                score: 80,
            }),
        ];
        apply_nested_content_candidate_bias_for(
            &mut utility_heavy_outer,
            CandidatePreference::Extraction,
        );
        assert!(utility_heavy_outer[1].score > utility_heavy_outer[0].score);

        let mut heading_rich_outer = vec![
            ranked_bias_candidate(BiasFixture {
                selector: "div.wrapper",
                path: "html > body > div.wrapper",
                tag_name: "div",
                text_char_count: 1_000,
                heading_count: 6,
                link_count: 3,
                paragraph_count: 2,
                primary_heading_level: None,
                primary_heading_count: 0,
                primary_heading_depth: None,
                utility_descendant_count: 0,
                score: 160,
            }),
            ranked_bias_candidate(BiasFixture {
                selector: "div.wrapper > section",
                path: "html > body > div.wrapper > section",
                tag_name: "section",
                text_char_count: 850,
                heading_count: 1,
                link_count: 4,
                paragraph_count: 2,
                primary_heading_level: None,
                primary_heading_count: 0,
                primary_heading_depth: None,
                utility_descendant_count: 0,
                score: 160,
            }),
        ];
        apply_nested_content_candidate_bias_for(
            &mut heading_rich_outer,
            CandidatePreference::Extraction,
        );
        assert!(heading_rich_outer[0].score > heading_rich_outer[1].score);

        let mut modest_heading_outer = vec![
            ranked_bias_candidate(BiasFixture {
                selector: "div.wrapper",
                path: "html > body > div.wrapper",
                tag_name: "div",
                text_char_count: 1_000,
                heading_count: 4,
                link_count: 3,
                paragraph_count: 2,
                primary_heading_level: None,
                primary_heading_count: 0,
                primary_heading_depth: None,
                utility_descendant_count: 0,
                score: 210,
            }),
            ranked_bias_candidate(BiasFixture {
                selector: "div.wrapper > section",
                path: "html > body > div.wrapper > section",
                tag_name: "section",
                text_char_count: 920,
                heading_count: 2,
                link_count: 4,
                paragraph_count: 2,
                primary_heading_level: None,
                primary_heading_count: 0,
                primary_heading_depth: None,
                utility_descendant_count: 0,
                score: 210,
            }),
        ];
        apply_nested_content_candidate_bias_for(
            &mut modest_heading_outer,
            CandidatePreference::Extraction,
        );
        assert!(modest_heading_outer[0].score > modest_heading_outer[1].score);

        let extraction_unknown_tag = content_candidate_score_for(
            &ContentCandidateScoreInputs {
                tag_name: "aside",
                has_main_role: false,
                has_article_body_itemprop: false,
                text_char_count: 800,
                heading_count: 1,
                link_count: 0,
                paragraph_count: 2,
                positive_signal_count: 0,
                negative_signal_count: 0,
                primary_heading_level: None,
                primary_heading_count: 0,
                primary_heading_depth: None,
                utility_descendant_count: 0,
                uses_exact_path_selector: false,
            },
            CandidatePreference::Extraction,
        );
        let extraction_dense_short = content_candidate_score_for(
            &ContentCandidateScoreInputs {
                tag_name: "section",
                has_main_role: false,
                has_article_body_itemprop: false,
                text_char_count: 220,
                heading_count: 1,
                link_count: 9,
                paragraph_count: 1,
                positive_signal_count: 0,
                negative_signal_count: 0,
                primary_heading_level: None,
                primary_heading_count: 0,
                primary_heading_depth: None,
                utility_descendant_count: 0,
                uses_exact_path_selector: false,
            },
            CandidatePreference::Extraction,
        );
        let extraction_dense_medium = content_candidate_score_for(
            &ContentCandidateScoreInputs {
                tag_name: "section",
                has_main_role: false,
                has_article_body_itemprop: false,
                text_char_count: 1_500,
                heading_count: 1,
                link_count: 13,
                paragraph_count: 2,
                positive_signal_count: 0,
                negative_signal_count: 0,
                primary_heading_level: None,
                primary_heading_count: 0,
                primary_heading_depth: None,
                utility_descendant_count: 0,
                uses_exact_path_selector: false,
            },
            CandidatePreference::Extraction,
        );
        let extraction_dense_large = content_candidate_score_for(
            &ContentCandidateScoreInputs {
                tag_name: "section",
                has_main_role: false,
                has_article_body_itemprop: false,
                text_char_count: 6_000,
                heading_count: 1,
                link_count: 9,
                paragraph_count: 2,
                positive_signal_count: 0,
                negative_signal_count: 0,
                primary_heading_level: None,
                primary_heading_count: 0,
                primary_heading_depth: None,
                utility_descendant_count: 0,
                uses_exact_path_selector: false,
            },
            CandidatePreference::Extraction,
        );
        let extraction_dense_wide = content_candidate_score_for(
            &ContentCandidateScoreInputs {
                tag_name: "section",
                has_main_role: false,
                has_article_body_itemprop: false,
                text_char_count: 10_000,
                heading_count: 1,
                link_count: 7,
                paragraph_count: 2,
                positive_signal_count: 0,
                negative_signal_count: 0,
                primary_heading_level: None,
                primary_heading_count: 0,
                primary_heading_depth: None,
                utility_descendant_count: 0,
                uses_exact_path_selector: false,
            },
            CandidatePreference::Extraction,
        );
        assert!(extraction_unknown_tag > extraction_dense_short);
        assert!(extraction_dense_medium != extraction_dense_large);
        assert!(extraction_dense_large != extraction_dense_wide);

        let extraction_link_order_left = ranked_bias_candidate(BiasFixture {
            selector: "#alpha",
            path: "html > body > article",
            tag_name: "article",
            text_char_count: 500,
            heading_count: 2,
            link_count: 2,
            paragraph_count: 2,
            primary_heading_level: None,
            primary_heading_count: 0,
            primary_heading_depth: None,
            utility_descendant_count: 0,
            score: 100,
        });
        let extraction_link_order_right = ranked_bias_candidate(BiasFixture {
            selector: "#beta",
            path: "html > body > article",
            tag_name: "article",
            text_char_count: 500,
            heading_count: 2,
            link_count: 4,
            paragraph_count: 2,
            primary_heading_level: None,
            primary_heading_count: 0,
            primary_heading_depth: None,
            utility_descendant_count: 0,
            score: 100,
        });
        assert_eq!(
            compare_content_candidates_for(
                &extraction_link_order_left,
                &extraction_link_order_right,
                CandidatePreference::Extraction,
            ),
            Ordering::Less
        );

        let extraction_depth_left = ranked_bias_candidate(BiasFixture {
            selector: "#deep",
            path: "html > body > main > article",
            tag_name: "article",
            text_char_count: 500,
            heading_count: 2,
            link_count: 2,
            paragraph_count: 2,
            primary_heading_level: None,
            primary_heading_count: 0,
            primary_heading_depth: None,
            utility_descendant_count: 0,
            score: 100,
        });
        let extraction_depth_right = ranked_bias_candidate(BiasFixture {
            selector: "#shallow",
            path: "html > body > main",
            tag_name: "main",
            text_char_count: 500,
            heading_count: 2,
            link_count: 2,
            paragraph_count: 2,
            primary_heading_level: None,
            primary_heading_count: 0,
            primary_heading_depth: None,
            utility_descendant_count: 0,
            score: 100,
        });
        assert_eq!(
            compare_content_candidates_for(
                &extraction_depth_left,
                &extraction_depth_right,
                CandidatePreference::Extraction,
            ),
            Ordering::Less
        );
        assert_eq!(path_depth("html > body > main > article"), 3);

        let extraction_text_left = ranked_bias_candidate(BiasFixture {
            selector: "#longer",
            path: "html > body > main",
            tag_name: "main",
            text_char_count: 700,
            heading_count: 2,
            link_count: 2,
            paragraph_count: 2,
            primary_heading_level: None,
            primary_heading_count: 0,
            primary_heading_depth: None,
            utility_descendant_count: 0,
            score: 100,
        });
        let extraction_text_right = ranked_bias_candidate(BiasFixture {
            selector: "#shorter",
            path: "html > body > navx",
            tag_name: "section",
            text_char_count: 500,
            heading_count: 2,
            link_count: 2,
            paragraph_count: 2,
            primary_heading_level: None,
            primary_heading_count: 0,
            primary_heading_depth: None,
            utility_descendant_count: 0,
            score: 100,
        });
        assert_eq!(
            compare_content_candidates_for(
                &extraction_text_left,
                &extraction_text_right,
                CandidatePreference::Extraction,
            ),
            Ordering::Less
        );

        let extraction_selector_alpha = ranked_bias_candidate(BiasFixture {
            selector: "#alpha",
            path: "html > body > nav1",
            tag_name: "section",
            text_char_count: 500,
            heading_count: 2,
            link_count: 2,
            paragraph_count: 2,
            primary_heading_level: None,
            primary_heading_count: 0,
            primary_heading_depth: None,
            utility_descendant_count: 0,
            score: 100,
        });
        let extraction_selector_beta = ranked_bias_candidate(BiasFixture {
            selector: "#beta",
            path: "html > body > nav2",
            tag_name: "section",
            text_char_count: 500,
            heading_count: 2,
            link_count: 2,
            paragraph_count: 2,
            primary_heading_level: None,
            primary_heading_count: 0,
            primary_heading_depth: None,
            utility_descendant_count: 0,
            score: 100,
        });
        assert_eq!(
            compare_content_candidates_for(
                &extraction_selector_alpha,
                &extraction_selector_beta,
                CandidatePreference::Extraction,
            ),
            Ordering::Less
        );
        assert_eq!(content_tag_rank("div"), 1);
        assert_eq!(content_tag_rank("aside"), 0);
        assert_eq!(content_tag_rank("main"), 3);
    }

    #[test]
    fn promotion_guards_cover_empty_mismatched_and_rejected_candidate_sets() {
        let promoted = ranked_content_candidate(PromotionFixture {
            selector: "article.article-body",
            path: "html > body > main#content > article.article-body",
            tag_name: "article",
            text_char_count: 950,
            heading_count: 3,
            link_count: 5,
            primary_heading_level: Some(1),
            primary_heading_depth: Some(2),
        });

        let mut empty_extraction = Vec::new();
        promote_precise_reading_descendant_candidate(
            &mut empty_extraction,
            std::slice::from_ref(&promoted),
        );
        assert!(empty_extraction.is_empty());

        let mut unchanged_extraction = vec![ranked_content_candidate(PromotionFixture {
            selector: "#content",
            path: "html > body > main#content",
            tag_name: "main",
            text_char_count: 1_000,
            heading_count: 3,
            link_count: 8,
            primary_heading_level: Some(1),
            primary_heading_depth: Some(1),
        })];
        promote_precise_reading_descendant_candidate(&mut unchanged_extraction, &[]);
        assert_eq!(unchanged_extraction[0].inspection.selector, "#content");

        let mismatched_reading = vec![ranked_content_candidate(PromotionFixture {
            selector: "#different",
            path: "html > body > article#different",
            tag_name: "article",
            text_char_count: 1_000,
            heading_count: 3,
            link_count: 8,
            primary_heading_level: Some(1),
            primary_heading_depth: Some(1),
        })];
        promote_precise_reading_descendant_candidate(
            &mut unchanged_extraction,
            &mismatched_reading,
        );
        assert_eq!(unchanged_extraction[0].inspection.selector, "#content");

        let tied_descendants = vec![
            unchanged_extraction[0].clone(),
            ranked_content_candidate(PromotionFixture {
                selector: "article.shallow",
                path: "html > body > main#content > article.shallow",
                tag_name: "article",
                text_char_count: 930,
                heading_count: 3,
                link_count: 4,
                primary_heading_level: Some(1),
                primary_heading_depth: Some(2),
            }),
            ranked_content_candidate(PromotionFixture {
                selector: "article.deep",
                path: "html > body > main#content > section > article.deep",
                tag_name: "article",
                text_char_count: 930,
                heading_count: 3,
                link_count: 4,
                primary_heading_level: Some(1),
                primary_heading_depth: Some(2),
            }),
        ];
        promote_precise_reading_descendant_candidate(&mut unchanged_extraction, &tied_descendants);
        assert_eq!(
            unchanged_extraction[0].inspection.selector,
            "article.shallow"
        );

        let mut chrome_wrapper_extraction = vec![ranked_content_candidate(PromotionFixture {
            selector: "#js-repo-pjax-container",
            path: "html > body > main#js-repo-pjax-container",
            tag_name: "main",
            text_char_count: 23_201,
            heading_count: 215,
            link_count: 619,
            primary_heading_level: Some(1),
            primary_heading_depth: Some(2),
        })];
        let chrome_wrapper_reading = vec![
            ranked_content_candidate(PromotionFixture {
                selector: "div.markdown-body",
                path: "html > body > main#js-repo-pjax-container > div > div > div.markdown-body",
                tag_name: "div",
                text_char_count: 23_026,
                heading_count: 53,
                link_count: 225,
                primary_heading_level: Some(2),
                primary_heading_depth: Some(2),
            }),
            ranked_content_candidate(PromotionFixture {
                selector: "#wiki-body",
                path: "html > body > main#js-repo-pjax-container > div > div > #wiki-body",
                tag_name: "div",
                text_char_count: 23_026,
                heading_count: 53,
                link_count: 225,
                primary_heading_level: Some(2),
                primary_heading_depth: Some(1),
            }),
        ];
        promote_precise_reading_descendant_candidate(
            &mut chrome_wrapper_extraction,
            &chrome_wrapper_reading,
        );
        assert_eq!(
            chrome_wrapper_extraction[0].inspection.selector,
            "#wiki-body"
        );

        let mut link_heavy_extraction = vec![ranked_content_candidate(PromotionFixture {
            selector: "#content",
            path: "html > body > main#content",
            tag_name: "main",
            text_char_count: 192_859,
            heading_count: 41,
            link_count: 2_951,
            primary_heading_level: Some(1),
            primary_heading_depth: Some(1),
        })];
        let link_heavy_reading = vec![
            ranked_content_candidate(PromotionFixture {
                selector: "div.mw-content-ltr.mw-parser-output",
                path: "html > body > main#content > div#bodyContent > div#mw-content-text > div.mw-content-ltr.mw-parser-output",
                tag_name: "div",
                text_char_count: 192_844,
                heading_count: 40,
                link_count: 2_641,
                primary_heading_level: Some(2),
                primary_heading_depth: Some(1),
            }),
            ranked_content_candidate(PromotionFixture {
                selector: "#mw-content-text",
                path: "html > body > main#content > div#bodyContent > div#mw-content-text",
                tag_name: "div",
                text_char_count: 192_844,
                heading_count: 40,
                link_count: 2_642,
                primary_heading_level: Some(2),
                primary_heading_depth: Some(1),
            }),
        ];
        promote_precise_reading_descendant_candidate(
            &mut link_heavy_extraction,
            &link_heavy_reading,
        );
        assert_eq!(
            link_heavy_extraction[0].inspection.selector,
            "#mw-content-text"
        );

        let mut ancestor_empty = Vec::new();
        promote_title_bearing_reading_ancestor_candidate(
            &mut ancestor_empty,
            std::slice::from_ref(&promoted),
        );
        assert!(ancestor_empty.is_empty());

        let mut ancestor_candidate = vec![ranked_content_candidate(PromotionFixture {
            selector: "article.article-body",
            path: "html > body > main#content > article.article-body",
            tag_name: "article",
            text_char_count: 950,
            heading_count: 3,
            link_count: 12,
            primary_heading_level: Some(2),
            primary_heading_depth: Some(3),
        })];
        promote_title_bearing_reading_ancestor_candidate(&mut ancestor_candidate, &[]);
        assert_eq!(
            ancestor_candidate[0].inspection.selector,
            "article.article-body"
        );

        let same_path_reading = vec![ancestor_candidate[0].clone()];
        promote_title_bearing_reading_ancestor_candidate(
            &mut ancestor_candidate,
            &same_path_reading,
        );
        assert_eq!(
            ancestor_candidate[0].inspection.selector,
            "article.article-body"
        );

        let unrelated_reading = vec![ranked_content_candidate(PromotionFixture {
            selector: "#sidebar",
            path: "html > body > aside#sidebar",
            tag_name: "aside",
            text_char_count: 980,
            heading_count: 4,
            link_count: 10,
            primary_heading_level: Some(1),
            primary_heading_depth: Some(2),
        })];
        promote_title_bearing_reading_ancestor_candidate(
            &mut ancestor_candidate,
            &unrelated_reading,
        );
        assert_eq!(
            ancestor_candidate[0].inspection.selector,
            "article.article-body"
        );

        let no_title_drop = vec![ranked_content_candidate(PromotionFixture {
            selector: "#content",
            path: "html > body > main#content",
            tag_name: "main",
            text_char_count: 980,
            heading_count: 4,
            link_count: 18,
            primary_heading_level: Some(2),
            primary_heading_depth: Some(3),
        })];
        promote_title_bearing_reading_ancestor_candidate(&mut ancestor_candidate, &no_title_drop);
        assert_eq!(
            ancestor_candidate[0].inspection.selector,
            "article.article-body"
        );

        let too_small_reading = vec![ranked_content_candidate(PromotionFixture {
            selector: "#content",
            path: "html > body > main#content",
            tag_name: "main",
            text_char_count: 1_500,
            heading_count: 4,
            link_count: 18,
            primary_heading_level: Some(1),
            primary_heading_depth: Some(4),
        })];
        promote_title_bearing_reading_ancestor_candidate(
            &mut ancestor_candidate,
            &too_small_reading,
        );
        assert_eq!(
            ancestor_candidate[0].inspection.selector,
            "article.article-body"
        );

        let too_many_headings = vec![ranked_content_candidate(PromotionFixture {
            selector: "#content",
            path: "html > body > main#content",
            tag_name: "main",
            text_char_count: 980,
            heading_count: 0,
            link_count: 18,
            primary_heading_level: Some(1),
            primary_heading_depth: Some(4),
        })];
        promote_title_bearing_reading_ancestor_candidate(
            &mut ancestor_candidate,
            &too_many_headings,
        );
        assert_eq!(
            ancestor_candidate[0].inspection.selector,
            "article.article-body"
        );

        let too_many_links = vec![ranked_content_candidate(PromotionFixture {
            selector: "#content",
            path: "html > body > main#content",
            tag_name: "main",
            text_char_count: 980,
            heading_count: 4,
            link_count: 100,
            primary_heading_level: Some(1),
            primary_heading_depth: Some(4),
        })];
        promote_title_bearing_reading_ancestor_candidate(&mut ancestor_candidate, &too_many_links);
        assert_eq!(
            ancestor_candidate[0].inspection.selector,
            "article.article-body"
        );

        let mut selector_tiebreak_extraction = vec![ranked_content_candidate(PromotionFixture {
            selector: "#content",
            path: "html > body > main#content",
            tag_name: "main",
            text_char_count: 1_000,
            heading_count: 3,
            link_count: 8,
            primary_heading_level: Some(1),
            primary_heading_depth: Some(1),
        })];
        let selector_tiebreak_reading = vec![
            selector_tiebreak_extraction[0].clone(),
            ranked_content_candidate(PromotionFixture {
                selector: "article.alpha",
                path: "html > body > main#content > article.alpha",
                tag_name: "article",
                text_char_count: 930,
                heading_count: 3,
                link_count: 4,
                primary_heading_level: Some(1),
                primary_heading_depth: Some(2),
            }),
            ranked_content_candidate(PromotionFixture {
                selector: "article.beta",
                path: "html > body > main#content > article.beta",
                tag_name: "article",
                text_char_count: 930,
                heading_count: 3,
                link_count: 4,
                primary_heading_level: Some(1),
                primary_heading_depth: Some(2),
            }),
        ];
        promote_precise_reading_descendant_candidate(
            &mut selector_tiebreak_extraction,
            &selector_tiebreak_reading,
        );
        assert_eq!(
            selector_tiebreak_extraction[0].inspection.selector,
            "article.beta"
        );
    }

    #[test]
    fn false_guard_paths_and_counting_fallbacks_cover_remaining_edges() {
        let comment_document = parse_document_node(
            "<html><body><!--hidden--><p>Hello</p><img alt=\"Hero\"></body></html>",
        );
        let body = select_first(&comment_document, "body").expect("body");
        let mut counted = String::new();
        collect_visible_text_for_count(body.children(), &mut counted);
        assert_eq!(counted, "Hello Hero");

        let bodyless_extraction_second_guard = content_candidate_score_for(
            &ContentCandidateScoreInputs {
                tag_name: "section",
                has_main_role: false,
                has_article_body_itemprop: false,
                text_char_count: 520,
                heading_count: 0,
                link_count: 0,
                paragraph_count: 0,
                positive_signal_count: 0,
                negative_signal_count: 0,
                primary_heading_level: None,
                primary_heading_count: 0,
                primary_heading_depth: None,
                utility_descendant_count: 0,
                uses_exact_path_selector: false,
            },
            CandidatePreference::Extraction,
        );
        let near_bodyless_extraction_second_guard = content_candidate_score_for(
            &ContentCandidateScoreInputs {
                tag_name: "section",
                has_main_role: false,
                has_article_body_itemprop: false,
                text_char_count: 500,
                heading_count: 0,
                link_count: 0,
                paragraph_count: 1,
                positive_signal_count: 0,
                negative_signal_count: 0,
                primary_heading_level: None,
                primary_heading_count: 0,
                primary_heading_depth: None,
                utility_descendant_count: 0,
                uses_exact_path_selector: false,
            },
            CandidatePreference::Extraction,
        );
        let title_fragment_guard = content_candidate_score_for(
            &ContentCandidateScoreInputs {
                tag_name: "section",
                has_main_role: false,
                has_article_body_itemprop: false,
                text_char_count: 450,
                heading_count: 1,
                link_count: 0,
                paragraph_count: 0,
                positive_signal_count: 0,
                negative_signal_count: 0,
                primary_heading_level: Some(1),
                primary_heading_count: 1,
                primary_heading_depth: Some(2),
                utility_descendant_count: 0,
                uses_exact_path_selector: false,
            },
            CandidatePreference::Extraction,
        );
        let extraction_medium_link_guard = content_candidate_score_for(
            &ContentCandidateScoreInputs {
                tag_name: "section",
                has_main_role: false,
                has_article_body_itemprop: false,
                text_char_count: 1_700,
                heading_count: 1,
                link_count: 13,
                paragraph_count: 2,
                positive_signal_count: 0,
                negative_signal_count: 0,
                primary_heading_level: None,
                primary_heading_count: 0,
                primary_heading_depth: None,
                utility_descendant_count: 0,
                uses_exact_path_selector: false,
            },
            CandidatePreference::Extraction,
        );
        let extraction_large_link_guard = content_candidate_score_for(
            &ContentCandidateScoreInputs {
                tag_name: "section",
                has_main_role: false,
                has_article_body_itemprop: false,
                text_char_count: 6_800,
                heading_count: 1,
                link_count: 9,
                paragraph_count: 2,
                positive_signal_count: 0,
                negative_signal_count: 0,
                primary_heading_level: None,
                primary_heading_count: 0,
                primary_heading_depth: None,
                utility_descendant_count: 0,
                uses_exact_path_selector: false,
            },
            CandidatePreference::Extraction,
        );
        let extraction_wide_link_guard = content_candidate_score_for(
            &ContentCandidateScoreInputs {
                tag_name: "section",
                has_main_role: false,
                has_article_body_itemprop: false,
                text_char_count: 12_500,
                heading_count: 1,
                link_count: 7,
                paragraph_count: 2,
                positive_signal_count: 0,
                negative_signal_count: 0,
                primary_heading_level: None,
                primary_heading_count: 0,
                primary_heading_depth: None,
                utility_descendant_count: 0,
                uses_exact_path_selector: false,
            },
            CandidatePreference::Extraction,
        );
        let reading_title_fragment_guard = content_candidate_score(&ContentCandidateScoreInputs {
            tag_name: "section",
            has_main_role: false,
            has_article_body_itemprop: false,
            text_char_count: 350,
            heading_count: 1,
            link_count: 0,
            paragraph_count: 0,
            positive_signal_count: 0,
            negative_signal_count: 0,
            primary_heading_level: Some(1),
            primary_heading_count: 1,
            primary_heading_depth: Some(2),
            utility_descendant_count: 0,
            uses_exact_path_selector: false,
        });
        let reading_medium_link_guard = content_candidate_score(&ContentCandidateScoreInputs {
            tag_name: "section",
            has_main_role: false,
            has_article_body_itemprop: false,
            text_char_count: 4_200,
            heading_count: 1,
            link_count: 9,
            paragraph_count: 2,
            positive_signal_count: 0,
            negative_signal_count: 0,
            primary_heading_level: None,
            primary_heading_count: 0,
            primary_heading_depth: None,
            utility_descendant_count: 0,
            uses_exact_path_selector: false,
        });
        let reading_wide_link_guard = content_candidate_score(&ContentCandidateScoreInputs {
            tag_name: "section",
            has_main_role: false,
            has_article_body_itemprop: false,
            text_char_count: 6_400,
            heading_count: 1,
            link_count: 7,
            paragraph_count: 2,
            positive_signal_count: 0,
            negative_signal_count: 0,
            primary_heading_level: None,
            primary_heading_count: 0,
            primary_heading_depth: None,
            utility_descendant_count: 0,
            uses_exact_path_selector: false,
        });
        assert_ne!(
            bodyless_extraction_second_guard,
            near_bodyless_extraction_second_guard
        );
        assert_ne!(title_fragment_guard, reading_title_fragment_guard);
        assert!(extraction_medium_link_guard != extraction_large_link_guard);
        assert!(extraction_large_link_guard != extraction_wide_link_guard);
        assert!(reading_medium_link_guard != reading_wide_link_guard);
        assert!(drops_outer_title_signal(Some(1), Some(4), Some(1), Some(6)));
        assert!(drops_outer_title_signal(Some(2), Some(2), Some(2), Some(3)));

        let mut false_edge_cases = vec![
            (
                CandidatePreference::Extraction,
                vec![
                    ranked_bias_candidate(BiasFixture {
                        selector: "main",
                        path: "html > body > main",
                        tag_name: "main",
                        text_char_count: 1_000,
                        heading_count: 3,
                        link_count: 12,
                        paragraph_count: 0,
                        primary_heading_level: Some(1),
                        primary_heading_count: 1,
                        primary_heading_depth: Some(1),
                        utility_descendant_count: 2,
                        score: 40,
                    }),
                    ranked_bias_candidate(BiasFixture {
                        selector: "main > article",
                        path: "html > body > main > article",
                        tag_name: "article",
                        text_char_count: 900,
                        heading_count: 2,
                        link_count: 2,
                        paragraph_count: 2,
                        primary_heading_level: None,
                        primary_heading_count: 0,
                        primary_heading_depth: None,
                        utility_descendant_count: 0,
                        score: 40,
                    }),
                ],
                vec![130, 10],
            ),
            (
                CandidatePreference::Extraction,
                vec![
                    ranked_bias_candidate(BiasFixture {
                        selector: "main",
                        path: "html > body > main",
                        tag_name: "main",
                        text_char_count: 1_000,
                        heading_count: 3,
                        link_count: 18,
                        paragraph_count: 5,
                        primary_heading_level: None,
                        primary_heading_count: 0,
                        primary_heading_depth: None,
                        utility_descendant_count: 0,
                        score: 10,
                    }),
                    ranked_bias_candidate(BiasFixture {
                        selector: "main > article",
                        path: "html > body > main > article",
                        tag_name: "article",
                        text_char_count: 930,
                        heading_count: 2,
                        link_count: 6,
                        paragraph_count: 2,
                        primary_heading_level: None,
                        primary_heading_count: 0,
                        primary_heading_depth: None,
                        utility_descendant_count: 0,
                        score: 10,
                    }),
                ],
                vec![100, -50],
            ),
            (
                CandidatePreference::Extraction,
                vec![
                    ranked_bias_candidate(BiasFixture {
                        selector: "main",
                        path: "html > body > main",
                        tag_name: "main",
                        text_char_count: 1_000,
                        heading_count: 6,
                        link_count: 18,
                        paragraph_count: 3,
                        primary_heading_level: None,
                        primary_heading_count: 0,
                        primary_heading_depth: None,
                        utility_descendant_count: 0,
                        score: 10,
                    }),
                    ranked_bias_candidate(BiasFixture {
                        selector: "main > article",
                        path: "html > body > main > article",
                        tag_name: "article",
                        text_char_count: 930,
                        heading_count: 3,
                        link_count: 6,
                        paragraph_count: 3,
                        primary_heading_level: None,
                        primary_heading_count: 0,
                        primary_heading_depth: None,
                        utility_descendant_count: 0,
                        score: 10,
                    }),
                ],
                vec![100, -50],
            ),
            (
                CandidatePreference::Extraction,
                vec![
                    ranked_bias_candidate(BiasFixture {
                        selector: "main",
                        path: "html > body > main",
                        tag_name: "main",
                        text_char_count: 1_000,
                        heading_count: 3,
                        link_count: 10,
                        paragraph_count: 3,
                        primary_heading_level: None,
                        primary_heading_count: 0,
                        primary_heading_depth: None,
                        utility_descendant_count: 1,
                        score: 10,
                    }),
                    ranked_bias_candidate(BiasFixture {
                        selector: "main > article",
                        path: "html > body > main > article",
                        tag_name: "article",
                        text_char_count: 930,
                        heading_count: 2,
                        link_count: 6,
                        paragraph_count: 3,
                        primary_heading_level: None,
                        primary_heading_count: 0,
                        primary_heading_depth: None,
                        utility_descendant_count: 0,
                        score: 10,
                    }),
                ],
                vec![100, -50],
            ),
            (
                CandidatePreference::Extraction,
                vec![
                    ranked_bias_candidate(BiasFixture {
                        selector: "main",
                        path: "html > body > main",
                        tag_name: "main",
                        text_char_count: 1_200,
                        heading_count: 6,
                        link_count: 2,
                        paragraph_count: 4,
                        primary_heading_level: None,
                        primary_heading_count: 0,
                        primary_heading_depth: None,
                        utility_descendant_count: 0,
                        score: 0,
                    }),
                    ranked_bias_candidate(BiasFixture {
                        selector: "main > section",
                        path: "html > body > main > section",
                        tag_name: "section",
                        text_char_count: 150,
                        heading_count: 1,
                        link_count: 0,
                        paragraph_count: 1,
                        primary_heading_level: None,
                        primary_heading_count: 0,
                        primary_heading_depth: None,
                        utility_descendant_count: 0,
                        score: 0,
                    }),
                ],
                vec![0, 0],
            ),
            (
                CandidatePreference::Extraction,
                vec![
                    ranked_bias_candidate(BiasFixture {
                        selector: "main",
                        path: "html > body > main",
                        tag_name: "main",
                        text_char_count: 1_200,
                        heading_count: 4,
                        link_count: 2,
                        paragraph_count: 5,
                        primary_heading_level: None,
                        primary_heading_count: 0,
                        primary_heading_depth: None,
                        utility_descendant_count: 0,
                        score: 0,
                    }),
                    ranked_bias_candidate(BiasFixture {
                        selector: "main > section",
                        path: "html > body > main > section",
                        tag_name: "section",
                        text_char_count: 150,
                        heading_count: 1,
                        link_count: 0,
                        paragraph_count: 1,
                        primary_heading_level: None,
                        primary_heading_count: 0,
                        primary_heading_depth: None,
                        utility_descendant_count: 0,
                        score: 0,
                    }),
                ],
                vec![0, 0],
            ),
            (
                CandidatePreference::Reading,
                vec![
                    ranked_bias_candidate(BiasFixture {
                        selector: "main",
                        path: "html > body > main",
                        tag_name: "main",
                        text_char_count: 1_000,
                        heading_count: 3,
                        link_count: 10,
                        paragraph_count: 2,
                        primary_heading_level: Some(1),
                        primary_heading_count: 1,
                        primary_heading_depth: Some(1),
                        utility_descendant_count: 0,
                        score: 0,
                    }),
                    ranked_bias_candidate(BiasFixture {
                        selector: "main > article",
                        path: "html > body > main > article",
                        tag_name: "article",
                        text_char_count: 650,
                        heading_count: 2,
                        link_count: 2,
                        paragraph_count: 2,
                        primary_heading_level: None,
                        primary_heading_count: 0,
                        primary_heading_depth: None,
                        utility_descendant_count: 0,
                        score: 0,
                    }),
                ],
                vec![0, 0],
            ),
            (
                CandidatePreference::Reading,
                vec![
                    ranked_bias_candidate(BiasFixture {
                        selector: "main",
                        path: "html > body > main",
                        tag_name: "main",
                        text_char_count: 1_000,
                        heading_count: 3,
                        link_count: 10,
                        paragraph_count: 0,
                        primary_heading_level: Some(1),
                        primary_heading_count: 1,
                        primary_heading_depth: Some(1),
                        utility_descendant_count: 0,
                        score: 0,
                    }),
                    ranked_bias_candidate(BiasFixture {
                        selector: "main > article",
                        path: "html > body > main > article",
                        tag_name: "article",
                        text_char_count: 900,
                        heading_count: 2,
                        link_count: 2,
                        paragraph_count: 2,
                        primary_heading_level: None,
                        primary_heading_count: 0,
                        primary_heading_depth: None,
                        utility_descendant_count: 0,
                        score: 0,
                    }),
                ],
                vec![90, -60],
            ),
            (
                CandidatePreference::Reading,
                vec![
                    ranked_bias_candidate(BiasFixture {
                        selector: "main",
                        path: "html > body > main",
                        tag_name: "main",
                        text_char_count: 1_000,
                        heading_count: 3,
                        link_count: 6,
                        paragraph_count: 2,
                        primary_heading_level: Some(1),
                        primary_heading_count: 1,
                        primary_heading_depth: Some(1),
                        utility_descendant_count: 0,
                        score: 0,
                    }),
                    ranked_bias_candidate(BiasFixture {
                        selector: "main > article",
                        path: "html > body > main > article",
                        tag_name: "article",
                        text_char_count: 900,
                        heading_count: 2,
                        link_count: 2,
                        paragraph_count: 2,
                        primary_heading_level: Some(1),
                        primary_heading_count: 1,
                        primary_heading_depth: Some(2),
                        utility_descendant_count: 0,
                        score: 0,
                    }),
                ],
                vec![145, -95],
            ),
        ];

        for (preference, mut candidates, expected_scores) in false_edge_cases.drain(..) {
            let original_paths = candidates
                .iter()
                .map(|candidate| candidate.inspection.path.clone())
                .collect::<Vec<_>>();
            apply_nested_content_candidate_bias_for(&mut candidates, preference);
            assert_eq!(
                candidates
                    .iter()
                    .map(|candidate| candidate.inspection.path.clone())
                    .collect::<Vec<_>>(),
                original_paths
            );
            assert_eq!(candidates.len(), expected_scores.len());
            assert_eq!(
                candidates
                    .iter()
                    .map(|candidate| candidate.score)
                    .collect::<Vec<_>>()
                    .len(),
                expected_scores.len()
            );
        }
    }

    #[test]
    fn title_bearing_reading_ancestor_promotion_restores_near_full_wrappers_that_keep_the_title() {
        let mut extraction_candidates = vec![ranked_content_candidate(PromotionFixture {
            selector: "article.article-body",
            path: "html > body > main#content > article.article-body",
            tag_name: "article",
            text_char_count: 950,
            heading_count: 3,
            link_count: 12,
            primary_heading_level: Some(2),
            primary_heading_depth: Some(3),
        })];
        let reading_candidates = vec![
            ranked_content_candidate(PromotionFixture {
                selector: "#content",
                path: "html > body > main#content",
                tag_name: "main",
                text_char_count: 980,
                heading_count: 4,
                link_count: 22,
                primary_heading_level: Some(1),
                primary_heading_depth: Some(4),
            }),
            extraction_candidates[0].clone(),
        ];

        promote_title_bearing_reading_ancestor_candidate(
            &mut extraction_candidates,
            &reading_candidates,
        );

        assert_eq!(extraction_candidates[0].inspection.selector, "#content");
    }

    #[test]
    fn selector_rank_and_nested_bias_helpers_cover_new_extraction_branches() {
        assert_eq!(selector_stability_rank("article:nth-of-type(2)"), 0);
        assert_eq!(selector_stability_rank("#main"), 5);
        assert_eq!(selector_stability_rank("[role=\"main\"]"), 4);
        assert_eq!(selector_stability_rank("article.card"), 3);
        assert_eq!(selector_stability_rank(".card"), 2);
        assert_eq!(selector_stability_rank("article"), 1);

        let mut heading_and_link_heavy_outer = vec![
            ranked_bias_candidate(BiasFixture {
                selector: "main.wrapper",
                path: "html > body > main.wrapper",
                tag_name: "main",
                text_char_count: 1_000,
                heading_count: 20,
                link_count: 40,
                paragraph_count: 3,
                primary_heading_level: None,
                primary_heading_count: 0,
                primary_heading_depth: None,
                utility_descendant_count: 0,
                score: 0,
            }),
            ranked_bias_candidate(BiasFixture {
                selector: "main.wrapper > article.story",
                path: "html > body > main.wrapper > article.story",
                tag_name: "article",
                text_char_count: 900,
                heading_count: 4,
                link_count: 12,
                paragraph_count: 3,
                primary_heading_level: None,
                primary_heading_count: 0,
                primary_heading_depth: None,
                utility_descendant_count: 0,
                score: 0,
            }),
        ];
        apply_nested_content_candidate_bias_for(
            &mut heading_and_link_heavy_outer,
            CandidatePreference::Extraction,
        );
        assert!(heading_and_link_heavy_outer[1].score > heading_and_link_heavy_outer[0].score);

        let mut stable_inner_with_fewer_links = vec![
            ranked_bias_candidate(BiasFixture {
                selector: "#content",
                path: "html > body > main#content",
                tag_name: "main",
                text_char_count: 1_000,
                heading_count: 3,
                link_count: 36,
                paragraph_count: 3,
                primary_heading_level: Some(1),
                primary_heading_count: 1,
                primary_heading_depth: Some(1),
                utility_descendant_count: 0,
                score: 0,
            }),
            ranked_bias_candidate(BiasFixture {
                selector: "article.story",
                path: "html > body > main#content > article.story",
                tag_name: "article",
                text_char_count: 970,
                heading_count: 4,
                link_count: 10,
                paragraph_count: 3,
                primary_heading_level: Some(1),
                primary_heading_count: 1,
                primary_heading_depth: Some(2),
                utility_descendant_count: 0,
                score: 0,
            }),
        ];
        apply_nested_content_candidate_bias_for(
            &mut stable_inner_with_fewer_links,
            CandidatePreference::Extraction,
        );
        assert!(stable_inner_with_fewer_links[1].score > stable_inner_with_fewer_links[0].score);

        let mut near_equal_inner = vec![
            ranked_bias_candidate(BiasFixture {
                selector: "main.wrapper",
                path: "html > body > main.wrapper",
                tag_name: "main",
                text_char_count: 1_000,
                heading_count: 5,
                link_count: 30,
                paragraph_count: 3,
                primary_heading_level: None,
                primary_heading_count: 0,
                primary_heading_depth: None,
                utility_descendant_count: 0,
                score: 0,
            }),
            ranked_bias_candidate(BiasFixture {
                selector: "article.story",
                path: "html > body > main.wrapper > article.story",
                tag_name: "article",
                text_char_count: 980,
                heading_count: 5,
                link_count: 10,
                paragraph_count: 3,
                primary_heading_level: None,
                primary_heading_count: 0,
                primary_heading_depth: None,
                utility_descendant_count: 0,
                score: 0,
            }),
        ];
        apply_nested_content_candidate_bias_for(
            &mut near_equal_inner,
            CandidatePreference::Extraction,
        );
        assert!(near_equal_inner[1].score > near_equal_inner[0].score);

        let mut stable_selector_inner = vec![
            ranked_bias_candidate(BiasFixture {
                selector: "#content",
                path: "html > body > main#content",
                tag_name: "main",
                text_char_count: 1_000,
                heading_count: 4,
                link_count: 30,
                paragraph_count: 3,
                primary_heading_level: Some(2),
                primary_heading_count: 1,
                primary_heading_depth: Some(1),
                utility_descendant_count: 0,
                score: 0,
            }),
            ranked_bias_candidate(BiasFixture {
                selector: "article.story",
                path: "html > body > main#content > article.story",
                tag_name: "article",
                text_char_count: 950,
                heading_count: 5,
                link_count: 10,
                paragraph_count: 3,
                primary_heading_level: Some(2),
                primary_heading_count: 1,
                primary_heading_depth: Some(2),
                utility_descendant_count: 0,
                score: 0,
            }),
        ];
        apply_nested_content_candidate_bias_for(
            &mut stable_selector_inner,
            CandidatePreference::Extraction,
        );
        assert!(stable_selector_inner[1].score > stable_selector_inner[0].score);

        let mut reading_preference_link_shell = vec![
            ranked_bias_candidate(BiasFixture {
                selector: "#content",
                path: "html > body > main#content",
                tag_name: "main",
                text_char_count: 1_000,
                heading_count: 3,
                link_count: 200,
                paragraph_count: 3,
                primary_heading_level: None,
                primary_heading_count: 0,
                primary_heading_depth: None,
                utility_descendant_count: 0,
                score: 0,
            }),
            ranked_bias_candidate(BiasFixture {
                selector: "article.story",
                path: "html > body > main#content > article.story",
                tag_name: "article",
                text_char_count: 980,
                heading_count: 3,
                link_count: 20,
                paragraph_count: 3,
                primary_heading_level: None,
                primary_heading_count: 0,
                primary_heading_depth: None,
                utility_descendant_count: 0,
                score: 0,
            }),
        ];
        apply_nested_content_candidate_bias_for(
            &mut reading_preference_link_shell,
            CandidatePreference::Reading,
        );
        assert!(reading_preference_link_shell[1].score > reading_preference_link_shell[0].score);

        let mut extraction_preference_link_shell = vec![
            ranked_bias_candidate(BiasFixture {
                selector: "#content",
                path: "html > body > main#content",
                tag_name: "main",
                text_char_count: 1_000,
                heading_count: 3,
                link_count: 200,
                paragraph_count: 3,
                primary_heading_level: None,
                primary_heading_count: 0,
                primary_heading_depth: None,
                utility_descendant_count: 0,
                score: 0,
            }),
            ranked_bias_candidate(BiasFixture {
                selector: "article.story",
                path: "html > body > main#content > article.story",
                tag_name: "article",
                text_char_count: 980,
                heading_count: 3,
                link_count: 20,
                paragraph_count: 3,
                primary_heading_level: None,
                primary_heading_count: 0,
                primary_heading_depth: None,
                utility_descendant_count: 0,
                score: 0,
            }),
        ];
        apply_nested_content_candidate_bias_for(
            &mut extraction_preference_link_shell,
            CandidatePreference::Extraction,
        );
        assert!(
            extraction_preference_link_shell[1].score > extraction_preference_link_shell[0].score
        );
    }

    #[test]
    fn promotion_helpers_cover_path_depth_empty_inputs_and_cleaner_descendants() {
        let mut empty = Vec::new();
        promote_cleaner_reading_descendant_candidate(&mut empty, &[]);
        promote_title_bearing_reading_ancestor_candidate(&mut empty, &[]);

        let mut cleaner_descendants = vec![ranked_content_candidate(PromotionFixture {
            selector: "#content",
            path: "html > body > main#content",
            tag_name: "main",
            text_char_count: 1_000,
            heading_count: 3,
            link_count: 80,
            primary_heading_level: Some(1),
            primary_heading_depth: Some(1),
        })];
        let reading_candidates = vec![
            cleaner_descendants[0].clone(),
            ranked_content_candidate(PromotionFixture {
                selector: ".story",
                path: "html > body > main#content > section > article.story",
                tag_name: "article",
                text_char_count: 930,
                heading_count: 3,
                link_count: 40,
                primary_heading_level: Some(1),
                primary_heading_depth: Some(2),
            }),
            ranked_content_candidate(PromotionFixture {
                selector: ".story-body",
                path: "html > body > main#content > article.story-body",
                tag_name: "article",
                text_char_count: 930,
                heading_count: 3,
                link_count: 40,
                primary_heading_level: Some(1),
                primary_heading_depth: Some(2),
            }),
        ];
        promote_cleaner_reading_descendant_candidate(&mut cleaner_descendants, &reading_candidates);
        assert_eq!(cleaner_descendants[0].inspection.selector, ".story-body");

        let mut precise_descendants = vec![ranked_content_candidate(PromotionFixture {
            selector: "main.layout",
            path: "html > body > main.layout",
            tag_name: "main",
            text_char_count: 1_000,
            heading_count: 3,
            link_count: 100,
            primary_heading_level: None,
            primary_heading_depth: None,
        })];
        let precise_reading = vec![
            ranked_content_candidate(PromotionFixture {
                selector: "article.story:nth-of-type(1)",
                path: "html > body > main.layout > section > article.story:nth-of-type(1)",
                tag_name: "article",
                text_char_count: 920,
                heading_count: 3,
                link_count: 60,
                primary_heading_level: None,
                primary_heading_depth: None,
            }),
            ranked_content_candidate(PromotionFixture {
                selector: "article.story",
                path: "html > body > main.layout > article.story",
                tag_name: "article",
                text_char_count: 920,
                heading_count: 3,
                link_count: 60,
                primary_heading_level: None,
                primary_heading_depth: None,
            }),
        ];
        promote_precise_reading_descendant_candidate(&mut precise_descendants, &precise_reading);
        assert_eq!(precise_descendants[0].inspection.selector, "article.story");

        let mut precise_path_depth_tie = vec![ranked_content_candidate(PromotionFixture {
            selector: "main.layout",
            path: "html > body > main.layout",
            tag_name: "main",
            text_char_count: 1_000,
            heading_count: 3,
            link_count: 100,
            primary_heading_level: None,
            primary_heading_depth: None,
        })];
        let precise_path_depth_reading = vec![
            ranked_content_candidate(PromotionFixture {
                selector: "article.story-a",
                path: "html > body > main.layout > section > article.story-a",
                tag_name: "article",
                text_char_count: 920,
                heading_count: 3,
                link_count: 60,
                primary_heading_level: None,
                primary_heading_depth: None,
            }),
            ranked_content_candidate(PromotionFixture {
                selector: "article.story-b",
                path: "html > body > main.layout > article.story-b",
                tag_name: "article",
                text_char_count: 920,
                heading_count: 3,
                link_count: 60,
                primary_heading_level: None,
                primary_heading_depth: None,
            }),
        ];
        promote_precise_reading_descendant_candidate(
            &mut precise_path_depth_tie,
            &precise_path_depth_reading,
        );
        assert_eq!(
            precise_path_depth_tie[0].inspection.selector,
            "article.story-b"
        );
    }

    #[test]
    fn selector_rank_and_link_preview_helpers_cover_attribute_and_reference_edges() {
        assert_eq!(selector_stability_rank("[itemprop=\"articleBody\"]"), 4);
        assert_eq!(selector_stability_rank("[data-surface=\"story\"]"), 4);

        assert!(!link_preview_is_low_signal(
            "#cite-note",
            "   ",
            "article > p"
        ));
        assert!(link_preview_is_low_signal(
            "#cite-note",
            "[12]",
            "article > p"
        ));
        assert!(link_preview_is_low_signal(
            "#cite-note",
            "*",
            "article > sup:nth-of-type(2)"
        ));
        assert!(link_preview_is_low_signal(
            "/privacy/terms",
            "Guide",
            "article > p > a"
        ));
        assert!(link_preview_is_low_signal(
            "/guide",
            "Terms apply",
            "article > p > a"
        ));
        assert!(link_preview_is_low_signal(
            "/guide",
            "Guide",
            "article > footer.related > a"
        ));
        assert!(same_page_url(
            "https://example.test/guide#fragment",
            "https://example.test/guide"
        ));
        assert!(!same_page_url(
            "https://example.test/guide",
            "not a valid url"
        ));
    }

    #[test]
    fn density_and_heading_helpers_cover_link_penalties_and_title_insertion() {
        assert!(!candidate_has_readable_density(
            "section", 1_500, 1, 25, 3, 3
        ));
        assert!(!candidate_has_readable_density("section", 100, 1, 0, 0, 1));
        assert!(candidate_has_readable_density("section", 180, 1, 1, 1, 0));
        assert!(!candidate_has_readable_density("section", 180, 8, 5, 1, 0));
        assert!(!candidate_has_readable_density("section", 180, 1, 20, 1, 0));
        assert!(candidate_has_readable_density("section", 300, 1, 1, 3, 0));
        assert!(candidate_has_readable_density("section", 220, 1, 1, 1, 0));
        assert!(!candidate_has_readable_density("section", 500, 20, 0, 2, 3));
        assert!(!candidate_has_readable_density("section", 500, 1, 30, 2, 3));
        assert!(candidate_has_readable_density(
            "section", 4_000, 20, 0, 2, 3
        ));
        assert!(candidate_has_readable_density(
            "section", 4_000, 1, 30, 2, 3
        ));
        let section_with_itemprop =
            parse_document_node("<section itemprop=\"articleBody\"><p>Alpha</p></section>");
        assert!(is_content_candidate_container(
            &select_first(&section_with_itemprop, "section").expect("section"),
            0,
        ));
        let section_with_role_main =
            parse_document_node("<section role=\"main\"><p>Alpha</p></section>");
        assert!(is_content_candidate_container(
            &select_first(&section_with_role_main, "section").expect("section"),
            0,
        ));
        let div_with_role = parse_document_node("<div role=\"main\"><p>Alpha</p></div>");
        assert!(is_content_candidate_container(
            &select_first(&div_with_role, "div").expect("div"),
            0,
        ));
        let div_with_itemprop =
            parse_document_node("<div itemprop=\"articleBody\"><p>Alpha</p></div>");
        assert!(is_content_candidate_container(
            &select_first(&div_with_itemprop, "div").expect("div"),
            0,
        ));
        let section_with_three_paragraphs =
            parse_document_node("<section><p>Alpha</p><p>Beta</p><p>Gamma</p></section>");
        assert!(element_has_narrative_section_shape(
            &select_first(&section_with_three_paragraphs, "section").expect("section"),
        ));
        let section_with_heading_and_list = parse_document_node(
            "<section><h2>Body</h2><p>Alpha</p><p>Beta</p><ul><li>One</li><li>Two</li></ul></section>",
        );
        assert!(element_has_narrative_section_shape(
            &select_first(&section_with_heading_and_list, "section").expect("section"),
        ));
        let section_with_list_shape = parse_document_node(
            "<section><p>Alpha</p><p>Beta</p><ul><li>One</li><li>Two</li></ul></section>",
        );
        assert!(element_has_narrative_section_shape(
            &select_first(&section_with_list_shape, "section").expect("section"),
        ));

        let document = parse_document_node(
            "<html><body><h1>Document Title</h1><section><h2>Body</h2><p>Alpha beta gamma delta epsilon zeta eta theta iota.</p></section></body></html>",
        );
        let mut zero_limit_headings = vec![HeadingInspection {
            level: 2,
            text: "Body".to_owned(),
            path: "html > body > section > h2".to_owned(),
        }];
        prepend_document_title_heading_if_missing(&document, 0, &mut zero_limit_headings);
        assert_eq!(zero_limit_headings[0].level, 2);
        let mut existing_h1 = vec![HeadingInspection {
            level: 1,
            text: "Document Title".to_owned(),
            path: "html > body > h1".to_owned(),
        }];
        prepend_document_title_heading_if_missing(&document, 3, &mut existing_h1);
        assert_eq!(existing_h1.len(), 1);
        let mut headings = vec![HeadingInspection {
            level: 2,
            text: "Body".to_owned(),
            path: "html > body > section > h2".to_owned(),
        }];
        let mut unconstrained_headings = headings.clone();
        prepend_document_title_heading_if_missing(&document, 3, &mut unconstrained_headings);
        assert_eq!(unconstrained_headings.len(), 2);
        assert_eq!(unconstrained_headings[0].level, 1);
        prepend_document_title_heading_if_missing(&document, 1, &mut headings);
        assert_eq!(headings.len(), 1);
        assert_eq!(headings[0].level, 1);
        assert_eq!(headings[0].text, "Document Title");

        let extraction_penalized = content_candidate_score_for(
            &ContentCandidateScoreInputs {
                tag_name: "section",
                has_main_role: false,
                has_article_body_itemprop: false,
                text_char_count: 300,
                heading_count: 1,
                link_count: 0,
                paragraph_count: 0,
                positive_signal_count: 0,
                negative_signal_count: 0,
                primary_heading_level: Some(1),
                primary_heading_count: 1,
                primary_heading_depth: Some(1),
                utility_descendant_count: 0,
                uses_exact_path_selector: false,
            },
            CandidatePreference::Extraction,
        );
        let reading_penalized = content_candidate_score_for(
            &ContentCandidateScoreInputs {
                tag_name: "section",
                has_main_role: false,
                has_article_body_itemprop: false,
                text_char_count: 220,
                heading_count: 1,
                link_count: 0,
                paragraph_count: 0,
                positive_signal_count: 0,
                negative_signal_count: 0,
                primary_heading_level: Some(1),
                primary_heading_count: 1,
                primary_heading_depth: Some(1),
                utility_descendant_count: 0,
                uses_exact_path_selector: false,
            },
            CandidatePreference::Reading,
        );
        let article_baseline = content_candidate_score_for(
            &ContentCandidateScoreInputs {
                tag_name: "article",
                has_main_role: false,
                has_article_body_itemprop: false,
                text_char_count: 220,
                heading_count: 1,
                link_count: 0,
                paragraph_count: 2,
                positive_signal_count: 0,
                negative_signal_count: 0,
                primary_heading_level: Some(1),
                primary_heading_count: 1,
                primary_heading_depth: Some(1),
                utility_descendant_count: 0,
                uses_exact_path_selector: false,
            },
            CandidatePreference::Reading,
        );
        assert!(extraction_penalized < article_baseline);
        assert!(reading_penalized < article_baseline);
    }

    #[test]
    fn extraction_nested_bias_guard_branches_cover_false_link_and_title_thresholds() {
        let mut link_gap_guard = vec![
            ranked_bias_candidate(BiasFixture {
                selector: "#content",
                path: "html > body > main#content",
                tag_name: "main",
                text_char_count: 1_000,
                heading_count: 20,
                link_count: 29,
                paragraph_count: 5,
                primary_heading_level: None,
                primary_heading_count: 0,
                primary_heading_depth: None,
                utility_descendant_count: 13,
                score: 100,
            }),
            ranked_bias_candidate(BiasFixture {
                selector: ".story",
                path: "html > body > main#content > article.story",
                tag_name: "article",
                text_char_count: 890,
                heading_count: 8,
                link_count: 6,
                paragraph_count: 1,
                primary_heading_level: None,
                primary_heading_count: 0,
                primary_heading_depth: None,
                utility_descendant_count: 0,
                score: 80,
            }),
        ];
        apply_nested_content_candidate_bias_for(
            &mut link_gap_guard,
            CandidatePreference::Extraction,
        );
        assert_eq!(link_gap_guard[0].score, 100);
        assert_eq!(link_gap_guard[1].score, 80);

        let mut equal_heading_guard = vec![
            ranked_bias_candidate(BiasFixture {
                selector: "#content",
                path: "html > body > main#content",
                tag_name: "main",
                text_char_count: 1_000,
                heading_count: 4,
                link_count: 40,
                paragraph_count: 10,
                primary_heading_level: None,
                primary_heading_count: 0,
                primary_heading_depth: None,
                utility_descendant_count: 13,
                score: 100,
            }),
            ranked_bias_candidate(BiasFixture {
                selector: "article.story",
                path: "html > body > main#content > article.story",
                tag_name: "article",
                text_char_count: 990,
                heading_count: 5,
                link_count: 10,
                paragraph_count: 1,
                primary_heading_level: None,
                primary_heading_count: 0,
                primary_heading_depth: None,
                utility_descendant_count: 0,
                score: 80,
            }),
        ];
        apply_nested_content_candidate_bias_for(
            &mut equal_heading_guard,
            CandidatePreference::Extraction,
        );
        assert_eq!(equal_heading_guard[0].score, 100);
        assert_eq!(equal_heading_guard[1].score, 80);

        let mut dropped_title_guard = vec![
            ranked_bias_candidate(BiasFixture {
                selector: "#content",
                path: "html > body > main#content",
                tag_name: "main",
                text_char_count: 1_000,
                heading_count: 10,
                link_count: 30,
                paragraph_count: 0,
                primary_heading_level: Some(1),
                primary_heading_count: 1,
                primary_heading_depth: Some(1),
                utility_descendant_count: 13,
                score: 100,
            }),
            ranked_bias_candidate(BiasFixture {
                selector: ".story",
                path: "html > body > main#content > article.story",
                tag_name: "article",
                text_char_count: 960,
                heading_count: 7,
                link_count: 5,
                paragraph_count: 1,
                primary_heading_level: None,
                primary_heading_count: 0,
                primary_heading_depth: None,
                utility_descendant_count: 0,
                score: 80,
            }),
        ];
        apply_nested_content_candidate_bias_for(
            &mut dropped_title_guard,
            CandidatePreference::Extraction,
        );
        assert_eq!(dropped_title_guard[0].score, 100);
        assert_eq!(dropped_title_guard[1].score, 80);

        let mut heading_gap_guard = vec![
            ranked_bias_candidate(BiasFixture {
                selector: "#content",
                path: "html > body > main#content",
                tag_name: "main",
                text_char_count: 1_000,
                heading_count: 10,
                link_count: 30,
                paragraph_count: 10,
                primary_heading_level: None,
                primary_heading_count: 0,
                primary_heading_depth: None,
                utility_descendant_count: 13,
                score: 100,
            }),
            ranked_bias_candidate(BiasFixture {
                selector: ".story",
                path: "html > body > main#content > article.story",
                tag_name: "article",
                text_char_count: 960,
                heading_count: 5,
                link_count: 5,
                paragraph_count: 1,
                primary_heading_level: None,
                primary_heading_count: 0,
                primary_heading_depth: None,
                utility_descendant_count: 0,
                score: 80,
            }),
        ];
        apply_nested_content_candidate_bias_for(
            &mut heading_gap_guard,
            CandidatePreference::Extraction,
        );
        assert_eq!(heading_gap_guard[0].score, 100);
        assert_eq!(heading_gap_guard[1].score, 80);
    }

    #[test]
    fn promotion_filter_guards_cover_remaining_false_threshold_paths() {
        let mut precise_text_threshold = vec![ranked_content_candidate(PromotionFixture {
            selector: "main.layout",
            path: "html > body > main.layout",
            tag_name: "main",
            text_char_count: 1_000,
            heading_count: 3,
            link_count: 100,
            primary_heading_level: None,
            primary_heading_depth: None,
        })];
        let precise_text_reading = vec![
            ranked_content_candidate(PromotionFixture {
                selector: "#other",
                path: "html > body > aside#other",
                tag_name: "aside",
                text_char_count: 200,
                heading_count: 1,
                link_count: 1,
                primary_heading_level: None,
                primary_heading_depth: None,
            }),
            ranked_content_candidate(PromotionFixture {
                selector: "article.story",
                path: "html > body > main.layout > article.story",
                tag_name: "article",
                text_char_count: 899,
                heading_count: 3,
                link_count: 60,
                primary_heading_level: None,
                primary_heading_depth: None,
            }),
        ];
        promote_precise_reading_descendant_candidate(
            &mut precise_text_threshold,
            &precise_text_reading,
        );
        assert_eq!(precise_text_threshold[0].inspection.selector, "main.layout");

        let mut precise_link_gap = vec![ranked_content_candidate(PromotionFixture {
            selector: "main.layout",
            path: "html > body > main.layout",
            tag_name: "main",
            text_char_count: 1_000,
            heading_count: 3,
            link_count: 79,
            primary_heading_level: None,
            primary_heading_depth: None,
        })];
        let precise_link_reading = vec![
            ranked_content_candidate(PromotionFixture {
                selector: "#other",
                path: "html > body > aside#other",
                tag_name: "aside",
                text_char_count: 200,
                heading_count: 1,
                link_count: 1,
                primary_heading_level: None,
                primary_heading_depth: None,
            }),
            ranked_content_candidate(PromotionFixture {
                selector: "article.story",
                path: "html > body > main.layout > article.story",
                tag_name: "article",
                text_char_count: 920,
                heading_count: 3,
                link_count: 60,
                primary_heading_level: None,
                primary_heading_depth: None,
            }),
        ];
        promote_precise_reading_descendant_candidate(&mut precise_link_gap, &precise_link_reading);
        assert_eq!(precise_link_gap[0].inspection.selector, "main.layout");

        let mut cleaner_heading_gap = vec![ranked_content_candidate(PromotionFixture {
            selector: "#content",
            path: "html > body > main#content",
            tag_name: "main",
            text_char_count: 1_000,
            heading_count: 4,
            link_count: 80,
            primary_heading_level: Some(1),
            primary_heading_depth: Some(1),
        })];
        let cleaner_heading_reading = vec![
            cleaner_heading_gap[0].clone(),
            ranked_content_candidate(PromotionFixture {
                selector: ".story-fragment",
                path: "html > body > main#content > article.story-fragment",
                tag_name: "article",
                text_char_count: 930,
                heading_count: 1,
                link_count: 40,
                primary_heading_level: Some(1),
                primary_heading_depth: Some(2),
            }),
        ];
        promote_cleaner_reading_descendant_candidate(
            &mut cleaner_heading_gap,
            &cleaner_heading_reading,
        );
        assert_eq!(cleaner_heading_gap[0].inspection.selector, "#content");
    }

    #[test]
    fn heading_shell_bias_helper_requires_extreme_heading_noise() {
        assert!(outer_wrapper_adds_heading_shell(
            HeadingShellCandidate {
                text_char_count: 23_197,
                heading_count: 215,
                link_count: 174,
                selector: "#repo-content-pjax-container",
            },
            HeadingShellCandidate {
                text_char_count: 23_026,
                heading_count: 53,
                link_count: 172,
                selector: "#wiki-body",
            },
        ));
        assert!(!outer_wrapper_adds_heading_shell(
            HeadingShellCandidate {
                text_char_count: 1_000,
                heading_count: 10,
                link_count: 8,
                selector: "#content",
            },
            HeadingShellCandidate {
                text_char_count: 930,
                heading_count: 5,
                link_count: 6,
                selector: "article.story",
            },
        ));
        assert!(!outer_wrapper_adds_heading_shell(
            HeadingShellCandidate {
                text_char_count: 1_000,
                heading_count: 64,
                link_count: 8,
                selector: "#content",
            },
            HeadingShellCandidate {
                text_char_count: 950,
                heading_count: 40,
                link_count: 6,
                selector: "article.story",
            },
        ));
        assert!(!outer_wrapper_adds_heading_shell(
            HeadingShellCandidate {
                text_char_count: 1_000,
                heading_count: 80,
                link_count: 24,
                selector: "#content",
            },
            HeadingShellCandidate {
                text_char_count: 950,
                heading_count: 40,
                link_count: 6,
                selector: "article.story",
            },
        ));
    }

    #[test]
    fn nested_bias_prefers_inner_candidate_when_outer_wrapper_only_adds_heading_shell() {
        let extraction_outer = ranked_bias_candidate(BiasFixture {
            selector: "#repo-content-pjax-container",
            path: "html > body > main#repo-content-pjax-container",
            tag_name: "main",
            text_char_count: 23_197,
            heading_count: 215,
            link_count: 174,
            paragraph_count: 7,
            primary_heading_level: Some(1),
            primary_heading_count: 1,
            primary_heading_depth: Some(1),
            utility_descendant_count: 0,
            score: 120,
        });
        let extraction_inner = ranked_bias_candidate(BiasFixture {
            selector: "#wiki-body",
            path: "html > body > main#repo-content-pjax-container > div#wiki-body",
            tag_name: "div",
            text_char_count: 23_026,
            heading_count: 53,
            link_count: 172,
            paragraph_count: 7,
            primary_heading_level: None,
            primary_heading_count: 0,
            primary_heading_depth: None,
            utility_descendant_count: 0,
            score: 80,
        });
        let mut extraction_candidates = vec![extraction_outer.clone(), extraction_inner.clone()];
        apply_nested_content_candidate_bias_for(
            &mut extraction_candidates,
            CandidatePreference::Extraction,
        );
        assert!(extraction_candidates[1].score > extraction_candidates[0].score);

        let mut reading_candidates = vec![extraction_outer, extraction_inner];
        apply_nested_content_candidate_bias_for(
            &mut reading_candidates,
            CandidatePreference::Reading,
        );
        assert!(reading_candidates[1].score > reading_candidates[0].score);
    }

    #[test]
    fn precise_descendant_promotion_uses_heading_shell_signal_when_link_gap_is_small() {
        let mut extraction_candidates = vec![ranked_content_candidate(PromotionFixture {
            selector: "#js-repo-pjax-container",
            path: "html > body > main#js-repo-pjax-container",
            tag_name: "main",
            text_char_count: 23_197,
            heading_count: 215,
            link_count: 174,
            primary_heading_level: Some(1),
            primary_heading_depth: Some(1),
        })];
        let reading_candidates = vec![
            ranked_content_candidate(PromotionFixture {
                selector: "#wiki-body",
                path: "html > body > main#js-repo-pjax-container > div#repo-content-pjax-container > div#wiki-wrapper > div#wiki-content > div#wiki-body",
                tag_name: "div",
                text_char_count: 23_026,
                heading_count: 53,
                link_count: 172,
                primary_heading_level: None,
                primary_heading_depth: None,
            }),
            extraction_candidates[0].clone(),
        ];

        promote_precise_reading_descendant_candidate(
            &mut extraction_candidates,
            &reading_candidates,
        );

        assert_eq!(extraction_candidates[0].inspection.selector, "#wiki-body");
    }

    #[test]
    fn precise_descendant_promotion_uses_heading_shell_signal_when_reading_top_is_current() {
        let extraction = ranked_content_candidate(PromotionFixture {
            selector: "#repo-content-pjax-container",
            path: "html > body > main#repo-content-pjax-container",
            tag_name: "main",
            text_char_count: 23_049,
            heading_count: 214,
            link_count: 172,
            primary_heading_level: Some(1),
            primary_heading_depth: Some(1),
        });
        let mut extraction_candidates = vec![extraction.clone()];
        let reading_candidates = vec![
            extraction,
            ranked_content_candidate(PromotionFixture {
                selector: "#wiki-body",
                path: "html > body > main#repo-content-pjax-container > div#wiki-wrapper > div#wiki-content > div#wiki-body",
                tag_name: "div",
                text_char_count: 23_026,
                heading_count: 53,
                link_count: 172,
                primary_heading_level: None,
                primary_heading_depth: None,
            }),
        ];

        promote_precise_reading_descendant_candidate(
            &mut extraction_candidates,
            &reading_candidates,
        );

        assert_eq!(extraction_candidates[0].inspection.selector, "#wiki-body");
    }

    #[test]
    fn precise_descendant_promotion_rejects_unstable_heading_shell_descendants() {
        let extraction = ranked_content_candidate(PromotionFixture {
            selector: "#repo-content-pjax-container",
            path: "html > body > main#repo-content-pjax-container",
            tag_name: "main",
            text_char_count: 23_049,
            heading_count: 214,
            link_count: 172,
            primary_heading_level: Some(1),
            primary_heading_depth: Some(1),
        });
        let mut extraction_candidates = vec![extraction.clone()];
        let reading_candidates = vec![
            extraction,
            ranked_content_candidate(PromotionFixture {
                selector: "div.markdown-body:nth-of-type(2)",
                path: "html > body > main#repo-content-pjax-container > div#wiki-wrapper > div#wiki-content > div#wiki-body > div.markdown-body:nth-of-type(2)",
                tag_name: "div",
                text_char_count: 23_026,
                heading_count: 53,
                link_count: 172,
                primary_heading_level: None,
                primary_heading_depth: None,
            }),
        ];

        promote_precise_reading_descendant_candidate(
            &mut extraction_candidates,
            &reading_candidates,
        );

        assert_eq!(
            extraction_candidates[0].inspection.selector,
            "#repo-content-pjax-container"
        );
    }

    #[test]
    fn precise_descendant_promotion_rejects_descendants_with_extra_links_without_shell_signal() {
        let extraction = ranked_content_candidate(PromotionFixture {
            selector: "#content",
            path: "html > body > main#content",
            tag_name: "main",
            text_char_count: 1_000,
            heading_count: 10,
            link_count: 10,
            primary_heading_level: Some(1),
            primary_heading_depth: Some(1),
        });
        let mut extraction_candidates = vec![extraction.clone()];
        let reading_candidates = vec![
            extraction,
            ranked_content_candidate(PromotionFixture {
                selector: "article.story",
                path: "html > body > main#content > article.story",
                tag_name: "article",
                text_char_count: 960,
                heading_count: 8,
                link_count: 11,
                primary_heading_level: Some(2),
                primary_heading_depth: Some(2),
            }),
        ];

        promote_precise_reading_descendant_candidate(
            &mut extraction_candidates,
            &reading_candidates,
        );

        assert_eq!(extraction_candidates[0].inspection.selector, "#content");
    }
}
