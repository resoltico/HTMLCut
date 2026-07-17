use super::super::*;

pub(in super::super) fn path_depth(path: &str) -> usize {
    path.matches(" > ").count()
}

pub(in super::super) fn selector_stability_rank(selector: &str) -> u8 {
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

pub(in super::super) fn promote_precise_reading_descendant_candidate(
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

pub(in super::super) fn promote_title_bearing_reading_ancestor_candidate(
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

pub(in super::super) fn promote_cleaner_reading_descendant_candidate(
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

pub(in super::super) fn content_tag_rank(tag_name: &str) -> u8 {
    match tag_name {
        "article" => 4,
        "main" => 3,
        "section" => 2,
        "div" => 1,
        _ => 0,
    }
}

pub(in super::super) fn primary_heading_bonus(level: u8) -> i32 {
    match level {
        1 => 130,
        2 => 78,
        3 => 20,
        _ => 0,
    }
}

pub(in super::super) fn has_shallow_primary_heading(
    level: Option<u8>,
    depth: Option<usize>,
) -> bool {
    match (level, depth) {
        (Some(1), Some(primary_heading_depth)) => primary_heading_depth <= 5,
        (Some(2), Some(primary_heading_depth)) => primary_heading_depth <= 2,
        _ => false,
    }
}

pub(in super::super) fn drops_outer_title_signal(
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

pub(in super::super) fn outer_wrapper_adds_heading_shell(
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

pub(in super::super) fn descendant_element_depth(
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

pub(in super::super) fn count_utility_descendant_roots(element: &ElementRef<'_>) -> usize {
    element
        .descendants()
        .filter_map(ElementRef::wrap)
        .filter(|descendant| descendant.id() != element.id())
        .filter(|descendant| element_looks_like_utility_chrome(descendant))
        .filter(|descendant| !has_utility_chrome_ancestor_before(descendant, element.id()))
        .count()
}

pub(in super::super) fn has_utility_chrome_ancestor_before(
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
