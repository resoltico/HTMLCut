use arbitrary::Arbitrary;
use htmlcut_core::{
    AttributeName, ExtractionRequest, HttpUrl, MaxBytes, RuntimeOptions, SelectionSpec, ValueSpec,
    WhitespaceMode,
};

#[derive(Arbitrary, Clone, Copy, Debug)]
pub enum FuzzValueKind {
    Text,
    SelectedHtml,
    InnerHtml,
    OuterHtml,
    Structured,
    AttributeHref,
}

impl FuzzValueKind {
    pub fn to_value_spec(self) -> ValueSpec {
        match self {
            Self::Text => ValueSpec::Text,
            Self::SelectedHtml => ValueSpec::SelectedHtml,
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
pub struct FuzzRendering {
    rewrite_urls: bool,
    normalize_whitespace: bool,
}

impl FuzzRendering {
    pub fn apply_to_request(self, request: &mut ExtractionRequest) {
        request.output.rendering.rewrite_urls = self.rewrite_urls;
        request.output.rendering.whitespace = if self.normalize_whitespace {
            WhitespaceMode::Normalize
        } else {
            WhitespaceMode::Rendered
        };
    }
}

pub fn runtime_for_html(html: &str) -> RuntimeOptions {
    RuntimeOptions {
        max_bytes: MaxBytes::new(html.len().max(1)).expect("non-zero fuzz byte limit"),
        ..RuntimeOptions::default()
    }
}

pub fn sample_base_url() -> HttpUrl {
    HttpUrl::parse("https://example.com/fuzz/index.html").expect("static URL")
}

fn non_zero_index(raw: u8) -> std::num::NonZeroUsize {
    std::num::NonZeroUsize::new((raw as usize % 4) + 1).expect("non-zero index")
}
