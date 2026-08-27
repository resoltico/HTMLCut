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
    headings.truncate(sample_limit);
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
    let utility_multiplier =
        content_candidate_utility_multiplier(preference, inputs.tag_name, shallow_primary_heading);
    let body_absence_penalty = content_candidate_body_absence_penalty(
        preference,
        inputs.paragraph_count,
        inputs.text_char_count,
    );
    let title_fragment_penalty = content_candidate_title_fragment_penalty(
        preference,
        inputs.tag_name,
        shallow_primary_heading,
        inputs.paragraph_count,
        inputs.text_char_count,
    );
    let link_density_penalty = content_candidate_link_density_penalty(
        preference,
        inputs.text_char_count,
        inputs.link_count,
        inputs.paragraph_count,
    );
    let (
        tag_bonus,
        role_bonus,
        itemprop_bonus,
        text_divisor,
        heading_multiplier,
        paragraph_multiplier,
        positive_multiplier,
        negative_multiplier,
        exact_path_penalty,
        heading_absence_penalty,
        short_text_penalty,
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
            (tag_bonus, 28, 55, 105, 10, 7, 22, 34, 220, 55, 35, 0, 0)
        }
        CandidatePreference::Reading => {
            let tag_bonus = match inputs.tag_name {
                "main" => 100,
                "article" => 90,
                "section" => 30,
                "div" => 15,
                _ => 0,
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
            (
                tag_bonus,
                45,
                35,
                90,
                12,
                7,
                20,
                28,
                220,
                45,
                30,
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

pub(in super::super) fn content_candidate_utility_multiplier(
    preference: CandidatePreference,
    tag_name: &str,
    shallow_primary_heading: bool,
) -> i32 {
    let primary_content_surface = matches!(tag_name, "article" | "main") && shallow_primary_heading;
    match preference {
        CandidatePreference::Extraction => {
            if primary_content_surface {
                18
            } else {
                24
            }
        }
        CandidatePreference::Reading => {
            if primary_content_surface {
                12
            } else {
                18
            }
        }
    }
}

pub(in super::super) fn content_candidate_body_absence_penalty(
    preference: CandidatePreference,
    paragraph_count: usize,
    text_char_count: usize,
) -> i32 {
    let (no_body_limit, no_body_penalty, sparse_body_limit, sparse_body_penalty) = match preference
    {
        CandidatePreference::Extraction => (500, 200, 420, 95),
        CandidatePreference::Reading => (500, 180, 320, 75),
    };
    if paragraph_count == 0 && text_char_count < no_body_limit {
        no_body_penalty
    } else if paragraph_count <= 1 && text_char_count < sparse_body_limit {
        sparse_body_penalty
    } else {
        0
    }
}

pub(in super::super) fn content_candidate_title_fragment_penalty(
    preference: CandidatePreference,
    tag_name: &str,
    shallow_primary_heading: bool,
    paragraph_count: usize,
    text_char_count: usize,
) -> i32 {
    if matches!(tag_name, "article" | "main") || !shallow_primary_heading || paragraph_count > 0 {
        return 0;
    }
    let (text_limit, penalty) = match preference {
        CandidatePreference::Extraction => (420, 200),
        CandidatePreference::Reading => (300, 170),
    };
    if text_char_count < text_limit {
        penalty
    } else {
        0
    }
}

pub(in super::super) fn content_candidate_link_density_penalty(
    preference: CandidatePreference,
    text_char_count: usize,
    link_count: usize,
    paragraph_count: usize,
) -> i32 {
    let (
        short_text_penalty,
        first_text_limit,
        first_penalty,
        second_text_limit,
        second_penalty,
        third_text_limit,
        third_penalty,
    ) = match preference {
        CandidatePreference::Extraction => (30, 1_600, 25, 6_500, 60, 12_000, 34),
        CandidatePreference::Reading => (25, 1_200, 15, 4_000, 40, 6_000, 22),
    };
    if text_char_count < 240 && link_count > 8 {
        short_text_penalty
    } else if link_count > paragraph_count.saturating_mul(6) && text_char_count < first_text_limit {
        first_penalty
    } else if link_count > paragraph_count.saturating_mul(4) && text_char_count < second_text_limit
    {
        second_penalty
    } else if link_count > paragraph_count.saturating_mul(3) && text_char_count < third_text_limit {
        third_penalty
    } else {
        0
    }
}
