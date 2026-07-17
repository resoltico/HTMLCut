use super::super::samples::{
    count_meaningful_headings, count_meaningful_links, first_meaningful_heading,
};
use super::super::selectors::recommend_content_selector;
use super::super::*;
use super::promotion::{
    count_utility_descendant_roots, descendant_element_depth, drops_outer_title_signal,
    outer_wrapper_adds_heading_shell, path_depth, selector_stability_rank,
};
use super::scoring::{
    candidate_has_readable_density, content_candidate_score_for, is_content_candidate_container,
    narrative_block_count,
};

pub(in super::super) fn build_ranked_content_candidates_for(
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
pub(in super::super) fn apply_nested_content_candidate_bias(
    candidates: &mut [RankedContentCandidate],
) {
    apply_nested_content_candidate_bias_for(candidates, CandidatePreference::Reading);
}

pub(in super::super) fn apply_nested_content_candidate_bias_for(
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
pub(in super::super) fn compare_content_candidates(
    left: &RankedContentCandidate,
    right: &RankedContentCandidate,
) -> Ordering {
    compare_content_candidates_for(left, right, CandidatePreference::Reading)
}

pub(in super::super) fn compare_content_candidates_for(
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
