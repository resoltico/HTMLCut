use arbitrary::Arbitrary;
use htmlcut_core::interop::v1::{Normalization, OutputKind, Selection, TextWhitespace};
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
    pub fn to_output_kind(self) -> OutputKind {
        match self {
            Self::Text | Self::Structured | Self::AttributeHref => OutputKind::Text,
            Self::InnerHtml => OutputKind::InnerHtml,
            Self::OuterHtml => OutputKind::OuterHtml,
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
    pub fn to_interop_selection(self) -> Selection {
        match self {
            Self::First => Selection::first(),
            Self::Single => Selection::single(),
            Self::All | Self::Nth(_) => Selection::nth(non_zero_index(match self {
                Self::Nth(raw) => raw,
                Self::All => 1,
                Self::First | Self::Single => 1,
            })),
        }
    }
}

#[derive(Arbitrary, Clone, Copy, Debug)]
pub struct FuzzNormalization {
    rewrite_urls: bool,
    normalize_whitespace: bool,
}

impl FuzzNormalization {
    pub fn to_interop_normalization(self) -> Normalization {
        Normalization::new(
            if self.normalize_whitespace {
                TextWhitespace::Normalize
            } else {
                TextWhitespace::Preserve
            },
            self.rewrite_urls,
        )
    }
}

pub fn sample_base_url() -> Url {
    Url::parse("https://example.com/fuzz/index.html").expect("static URL")
}

fn non_zero_index(raw: u8) -> std::num::NonZeroUsize {
    std::num::NonZeroUsize::new((raw as usize % 4) + 1).expect("non-zero index")
}
