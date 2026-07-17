use super::*;

pub(super) struct CandidateFixture<'a> {
    pub(super) path: &'a str,
    pub(super) selector: &'a str,
    pub(super) text_char_count: usize,
    pub(super) heading_count: usize,
    pub(super) link_count: usize,
    pub(super) primary_heading_level: Option<u8>,
    pub(super) primary_heading_count: usize,
    pub(super) primary_heading_depth: Option<usize>,
    pub(super) utility_descendant_count: usize,
    pub(super) score: i32,
}

pub(super) struct PromotionFixture<'a> {
    pub(super) selector: &'a str,
    pub(super) path: &'a str,
    pub(super) tag_name: &'a str,
    pub(super) text_char_count: usize,
    pub(super) heading_count: usize,
    pub(super) link_count: usize,
    pub(super) primary_heading_level: Option<u8>,
    pub(super) primary_heading_depth: Option<usize>,
}

pub(super) struct BiasFixture<'a> {
    pub(super) selector: &'a str,
    pub(super) path: &'a str,
    pub(super) tag_name: &'a str,
    pub(super) text_char_count: usize,
    pub(super) heading_count: usize,
    pub(super) link_count: usize,
    pub(super) paragraph_count: usize,
    pub(super) primary_heading_level: Option<u8>,
    pub(super) primary_heading_count: usize,
    pub(super) primary_heading_depth: Option<usize>,
    pub(super) utility_descendant_count: usize,
    pub(super) score: i32,
}

pub(super) fn ranked_candidate(fixture: CandidateFixture<'_>) -> RankedContentCandidate {
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

pub(super) fn content_candidate(
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

pub(super) fn ranked_content_candidate(fixture: PromotionFixture<'_>) -> RankedContentCandidate {
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

pub(super) fn ranked_bias_candidate(fixture: BiasFixture<'_>) -> RankedContentCandidate {
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
