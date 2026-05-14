use arbitrary::Arbitrary;
use htmlcut_core::interop::v1::{
    AttributeName, HttpUrl, Output, Rendering, Selection, StrategyKind, TextWhitespace,
};

#[derive(Arbitrary, Clone, Copy, Debug)]
pub enum FuzzValueKind {
    Text,
    InnerHtml,
    OuterHtml,
    Structured,
    AttributeHref,
}

impl FuzzValueKind {
    pub fn to_output(self, strategy_kind: StrategyKind) -> Output {
        match self {
            Self::Text => Output::text(),
            Self::InnerHtml => Output::inner_html(),
            Self::OuterHtml => Output::outer_html(),
            Self::Structured => Output::structured(),
            Self::AttributeHref => Output::attribute(
                AttributeName::new(match strategy_kind {
                    StrategyKind::CssSelector => "href",
                    StrategyKind::DelimiterPair => "data-id",
                })
                .expect("static attribute name"),
            ),
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
            Self::All => Selection::all(),
            Self::Nth(raw) => Selection::nth(non_zero_index(raw)),
        }
    }
}

#[derive(Arbitrary, Clone, Copy, Debug)]
pub struct FuzzRendering {
    rewrite_urls: bool,
    normalize_whitespace: bool,
}

impl FuzzRendering {
    pub fn to_interop_rendering(self) -> Rendering {
        Rendering::new(
            if self.normalize_whitespace {
                TextWhitespace::Normalize
            } else {
                TextWhitespace::Rendered
            },
            self.rewrite_urls,
        )
    }
}

pub fn sample_base_url() -> HttpUrl {
    HttpUrl::parse("https://example.com/fuzz/index.html").expect("static URL")
}

fn non_zero_index(raw: u8) -> std::num::NonZeroUsize {
    std::num::NonZeroUsize::new((raw as usize % 4) + 1).expect("non-zero index")
}
