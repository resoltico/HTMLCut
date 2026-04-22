use arbitrary::Arbitrary;
use htmlcut_core::{
    AttributeName, ExtractionRequest, RuntimeOptions, SelectionSpec, ValueSpec, WhitespaceMode,
};
use url::Url;

#[derive(Arbitrary, Clone, Copy, Debug)]
pub enum FuzzValueKind {
    Text,
    InnerHtml,
    OuterHtml,
    Structured,
    AttributeHref,
}

impl FuzzValueKind {
    pub fn to_value_spec(self) -> ValueSpec {
        match self {
            Self::Text => ValueSpec::Text,
            Self::InnerHtml => ValueSpec::InnerHtml,
            Self::OuterHtml => ValueSpec::OuterHtml,
            Self::Structured => ValueSpec::Structured,
            Self::AttributeHref => ValueSpec::Attribute {
                name: AttributeName::new("href").expect("static attribute name"),
            },
        }
    }
}

#[derive(Arbitrary, Clone, Copy, Debug)]
pub enum FuzzSelection {
    First,
    Single,
    All,
    Nth(u8),
}

impl FuzzSelection {
    pub fn to_selection_spec(self) -> SelectionSpec {
        match self {
            Self::First => SelectionSpec::First,
            Self::Single => SelectionSpec::Single,
            Self::All => SelectionSpec::All,
            Self::Nth(raw) => SelectionSpec::nth(non_zero_index(raw)),
        }
    }
}

#[derive(Arbitrary, Clone, Copy, Debug)]
pub struct FuzzNormalization {
    rewrite_urls: bool,
    normalize_whitespace: bool,
}

impl FuzzNormalization {
    pub fn apply_to_request(self, request: &mut ExtractionRequest) {
        request.normalization.rewrite_urls = self.rewrite_urls;
        request.normalization.whitespace = if self.normalize_whitespace {
            WhitespaceMode::Normalize
        } else {
            WhitespaceMode::Preserve
        };
    }
}

pub fn runtime_for_html(html: &str) -> RuntimeOptions {
    RuntimeOptions {
        max_bytes: html.len().max(1),
        ..RuntimeOptions::default()
    }
}

pub fn sample_base_url() -> Url {
    Url::parse("https://example.com/fuzz/index.html").expect("static URL")
}

fn non_zero_index(raw: u8) -> std::num::NonZeroUsize {
    std::num::NonZeroUsize::new((raw as usize % 4) + 1).expect("non-zero index")
}
