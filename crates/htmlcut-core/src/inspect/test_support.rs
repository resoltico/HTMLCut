use super::ContentCandidateScoreInputs;

/// Test-only inputs for the content-candidate ranking policy.
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
#[derive(Clone, Copy, Debug)]
pub(crate) enum ContentCandidateTestPreference {
    Extraction,
    Reading,
}

pub(super) fn content_candidate_score_inputs_for_tests(
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
