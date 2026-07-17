use super::super::samples::sample_headings_from_scope;
use super::super::*;
use super::promotion::{has_shallow_primary_heading, primary_heading_bonus};

pub(in super::super) fn narrative_block_count(
    prose_paragraph_count: usize,
    list_item_count: usize,
) -> usize {
    prose_paragraph_count + list_item_count.div_ceil(3).min(6)
}

pub(in super::super) fn candidate_has_readable_density(
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

pub(in super::super) fn same_page_url(candidate: &str, current: &str) -> bool {
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

pub(in super::super) fn prepend_document_title_heading_if_missing(
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

pub(in super::super) fn select_elements_in_scope<'a>(
    document: &'a Html,
    scope_path: Option<&str>,
    selector: &'a Selector,
) -> Box<dyn Iterator<Item = ElementRef<'a>> + 'a> {
    if let Some(scope) = scope_path.and_then(|path| select_first(document, path)) {
        return Box::new(scope.select(selector));
    }

    Box::new(document.select(selector))
}

pub(in super::super) fn element_attr_equals_ignore_ascii_case(
    element: &ElementRef<'_>,
    attribute_name: &str,
    expected_value: &str,
) -> bool {
    match element.value().attr(attribute_name) {
        Some(value) => value.eq_ignore_ascii_case(expected_value),
        None => false,
    }
}

pub(in super::super) fn is_content_candidate_container(
    element: &ElementRef<'_>,
    positive_signal_count: usize,
) -> bool {
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

pub(in super::super) fn element_has_narrative_section_shape(element: &ElementRef<'_>) -> bool {
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
pub(in super::super) fn content_candidate_score(inputs: &ContentCandidateScoreInputs<'_>) -> i32 {
    content_candidate_score_for(inputs, CandidatePreference::Reading)
}

pub(in super::super) fn content_candidate_score_for(
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
