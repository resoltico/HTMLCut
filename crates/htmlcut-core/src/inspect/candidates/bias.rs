use super::super::CandidatePreference;
use super::promotion::selector_stability_rank;

#[derive(Clone, Copy)]
pub(in super::super) struct CandidateBiasInput<'a> {
    pub(in super::super) selector: &'a str,
    pub(in super::super) text_char_count: usize,
    pub(in super::super) heading_count: usize,
    pub(in super::super) link_count: usize,
    pub(in super::super) paragraph_count: usize,
    pub(in super::super) primary_heading_count: usize,
    pub(in super::super) utility_descendant_count: usize,
}

pub(in super::super) fn extraction_prefers_utility_light_descendant(
    preference: CandidatePreference,
    outer: CandidateBiasInput<'_>,
    inner: CandidateBiasInput<'_>,
) -> bool {
    preference == CandidatePreference::Extraction
        && inner.text_char_count * 100 >= outer.text_char_count * 92
        && inner.paragraph_count + 1 >= outer.paragraph_count
        && outer.heading_count <= inner.heading_count + 2
        && (outer.link_count >= inner.link_count + 8
            || outer.utility_descendant_count >= inner.utility_descendant_count + 2)
}

pub(in super::super) fn extraction_preserves_title_bearing_outer_wrapper(
    preference: CandidatePreference,
    drops_outer_title_signal: bool,
    outer: CandidateBiasInput<'_>,
    inner: CandidateBiasInput<'_>,
) -> bool {
    preference == CandidatePreference::Extraction
        && drops_outer_title_signal
        && inner.text_char_count * 100 >= outer.text_char_count * 85
        && outer.paragraph_count > 0
}

pub(in super::super) fn extraction_prefers_heading_and_link_light_descendant(
    preference: CandidatePreference,
    outer: CandidateBiasInput<'_>,
    inner: CandidateBiasInput<'_>,
) -> bool {
    preference == CandidatePreference::Extraction
        && inner.text_char_count * 100 >= outer.text_char_count * 88
        && outer.heading_count >= inner.heading_count + 12
        && outer.link_count >= inner.link_count + 24
}

pub(in super::super) fn extraction_prefers_near_complete_link_light_descendant(
    preference: CandidatePreference,
    outer: CandidateBiasInput<'_>,
    inner: CandidateBiasInput<'_>,
) -> bool {
    preference == CandidatePreference::Extraction
        && inner.text_char_count * 100 >= outer.text_char_count * 98
        && outer.heading_count >= inner.heading_count
        && outer.link_count >= inner.link_count + 20
}

pub(in super::super) fn prefers_heavy_link_descendant(
    outer: CandidateBiasInput<'_>,
    inner: CandidateBiasInput<'_>,
) -> bool {
    inner.text_char_count * 100 >= outer.text_char_count * 98
        && outer.link_count >= inner.link_count + 120
}

pub(in super::super) fn extraction_prefers_stable_link_light_descendant(
    preference: CandidatePreference,
    drops_outer_title_signal: bool,
    outer: CandidateBiasInput<'_>,
    inner: CandidateBiasInput<'_>,
) -> bool {
    preference == CandidatePreference::Extraction
        && inner.text_char_count * 100 >= outer.text_char_count * 95
        && !drops_outer_title_signal
        && outer.heading_count <= inner.heading_count + 4
        && outer.link_count >= inner.link_count + 20
        && selector_stability_rank(inner.selector) >= selector_stability_rank(outer.selector)
}

pub(in super::super) fn extraction_preserves_large_outer_candidate(
    preference: CandidatePreference,
    outer: CandidateBiasInput<'_>,
    inner: CandidateBiasInput<'_>,
) -> bool {
    preference == CandidatePreference::Extraction
        && outer.text_char_count >= inner.text_char_count.saturating_mul(6)
        && outer.paragraph_count >= inner.paragraph_count + 4
        && outer.heading_count >= inner.heading_count + 4
}

pub(in super::super) fn prefers_utility_light_descendant(
    outer: CandidateBiasInput<'_>,
    inner: CandidateBiasInput<'_>,
) -> bool {
    inner.text_char_count * 100 >= outer.text_char_count * 78
        && inner.paragraph_count + 1 >= outer.paragraph_count
        && (outer.utility_descendant_count >= inner.utility_descendant_count + 8
            || (outer.utility_descendant_count > inner.utility_descendant_count
                && outer.link_count > inner.link_count + 8))
        && outer.heading_count <= inner.heading_count + 2
}

pub(in super::super) fn preserves_title_bearing_outer_candidate(
    drops_outer_title_signal: bool,
    outer: CandidateBiasInput<'_>,
    inner: CandidateBiasInput<'_>,
) -> bool {
    outer.paragraph_count > 0
        && drops_outer_title_signal
        && inner.text_char_count * 100 >= outer.text_char_count * 70
        && outer.link_count <= inner.link_count + 70
}

pub(in super::super) fn preserves_primary_heading_outer_candidate(
    outer: CandidateBiasInput<'_>,
    inner: CandidateBiasInput<'_>,
) -> bool {
    outer.paragraph_count > 0
        && outer.primary_heading_count > inner.primary_heading_count
        && inner.text_char_count * 100 >= outer.text_char_count * 80
        && outer.link_count <= inner.link_count + 20
        && outer.utility_descendant_count <= inner.utility_descendant_count + 6
}

pub(in super::super) fn preserves_heading_rich_outer_candidate(
    outer: CandidateBiasInput<'_>,
    inner: CandidateBiasInput<'_>,
) -> bool {
    outer.paragraph_count > 0
        && inner.text_char_count * 100 >= outer.text_char_count * 80
        && outer.heading_count >= inner.heading_count + 4
        && outer.link_count <= inner.link_count + 20
        && outer.utility_descendant_count <= inner.utility_descendant_count + 6
}

pub(in super::super) fn inner_link_density_exceeds_outer(
    outer_link_count: usize,
    inner_link_count: usize,
) -> bool {
    outer_link_count > 0 && inner_link_count * 100 > outer_link_count * 80
}
