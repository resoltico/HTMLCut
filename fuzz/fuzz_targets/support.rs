#![allow(dead_code)]

// Shared fuzz helpers are compiled into multiple target binaries, each of which
// exercises only a subset of the common builders and drivers.

use arbitrary::Arbitrary;
use htmlcut_core::interop::v1::{
    DelimiterMode, HtmlInput, Normalization, Output, OutputKind, Plan, PlanStrategy, RegexFlag,
    Selection, TextWhitespace, execute_plan,
};
use htmlcut_core::{
    AttributeName, ExtractionRequest, ExtractionSpec, InspectionOptions, RuntimeOptions,
    SelectionSpec, SelectorQuery, SliceBoundary, SliceSpec, SourceRequest, ValueSpec, extract,
    inspect_source, parse_document, preview_extraction,
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
    pub fn to_selection_spec(self) -> SelectionSpec {
        match self {
            Self::First => SelectionSpec::First,
            Self::Single => SelectionSpec::Single,
            Self::All => SelectionSpec::All,
            Self::Nth(raw) => SelectionSpec::nth(non_zero_index(raw)),
        }
    }

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
pub struct FuzzRegexFlags {
    case_insensitive: bool,
    multi_line: bool,
    dot_matches_new_line: bool,
    swap_greed: bool,
    ignore_whitespace: bool,
}

impl FuzzRegexFlags {
    pub fn to_interop_flags(self) -> Vec<RegexFlag> {
        let mut flags = Vec::new();
        if self.case_insensitive {
            flags.push(RegexFlag::CaseInsensitive);
        }
        if self.multi_line {
            flags.push(RegexFlag::MultiLine);
        }
        if self.dot_matches_new_line {
            flags.push(RegexFlag::DotMatchesNewLine);
        }
        if self.swap_greed {
            flags.push(RegexFlag::SwapGreed);
        }
        if self.ignore_whitespace {
            flags.push(RegexFlag::IgnoreWhitespace);
        }
        flags
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
            htmlcut_core::WhitespaceMode::Normalize
        } else {
            htmlcut_core::WhitespaceMode::Preserve
        };
    }

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

pub fn runtime_for_html(html: &str) -> RuntimeOptions {
    RuntimeOptions {
        max_bytes: html.len().max(1),
        ..RuntimeOptions::default()
    }
}

pub fn sample_base_url() -> Url {
    Url::parse("https://example.com/fuzz/index.html").expect("static URL")
}

pub fn drive_parse_surfaces(html: &str) {
    let source = SourceRequest::memory("fuzz", html);
    let runtime = runtime_for_html(html);
    let _ = parse_document(&source, &runtime);
    let _ = inspect_source(&source, &runtime, &InspectionOptions::default());
}

pub fn drive_selector_request(
    html: &str,
    selector: &str,
    value_kind: FuzzValueKind,
    selection: FuzzSelection,
    normalization: FuzzNormalization,
) {
    let Ok(selector) = SelectorQuery::new(selector) else {
        return;
    };

    let mut request = ExtractionRequest::new(
        SourceRequest::memory("fuzz", html).with_base_url(sample_base_url()),
        ExtractionSpec::selector(selector),
    );
    request.extraction = request
        .extraction
        .clone()
        .with_selection(selection.to_selection_spec())
        .with_value(value_kind.to_value_spec());
    normalization.apply_to_request(&mut request);

    let runtime = runtime_for_html(html);
    let _ = preview_extraction(&request, &runtime);
    let _ = extract(&request, &runtime);
}

pub fn drive_slice_request(
    html: &str,
    start: &str,
    end: &str,
    regex_mode: bool,
    flags: FuzzRegexFlags,
    include_start: bool,
    include_end: bool,
    value_kind: FuzzValueKind,
    selection: FuzzSelection,
    normalization: FuzzNormalization,
) {
    let Ok(start) = SliceBoundary::new(start) else {
        return;
    };
    let Ok(end) = SliceBoundary::new(end) else {
        return;
    };

    let slice = if regex_mode {
        SliceSpec::regex(start, end, regex_flags_string(flags))
    } else {
        SliceSpec::new(start, end)
    }
    .with_boundary_inclusion(include_start, include_end);

    let mut request = ExtractionRequest::new(
        SourceRequest::memory("fuzz", html).with_base_url(sample_base_url()),
        ExtractionSpec::slice(slice),
    );
    request.extraction = request
        .extraction
        .clone()
        .with_selection(selection.to_selection_spec())
        .with_value(value_kind.to_value_spec());
    normalization.apply_to_request(&mut request);

    let runtime = runtime_for_html(html);
    let _ = preview_extraction(&request, &runtime);
    let _ = extract(&request, &runtime);
}

pub fn drive_interop_request(
    html: &str,
    strategy: FuzzInteropStrategy,
    value_kind: FuzzValueKind,
    selection: FuzzSelection,
    normalization: FuzzNormalization,
) {
    let Ok(source) = HtmlInput::new("fuzz", html).map(|input| input.with_input_base_url(sample_base_url())) else {
        return;
    };

    let Some(strategy) = strategy.to_plan_strategy() else {
        return;
    };

    let plan = Plan::new(
        strategy,
        selection.to_interop_selection(),
        Output::new(value_kind.to_output_kind()),
        normalization.to_interop_normalization(),
    );

    let _ = plan.stable_json();
    let _ = execute_plan(&source, &plan);
}

#[derive(Arbitrary, Debug)]
pub enum FuzzInteropStrategy {
    Selector {
        selector: String,
    },
    DelimiterPair {
        start: String,
        end: String,
        regex_mode: bool,
        include_start: bool,
        include_end: bool,
        flags: FuzzRegexFlags,
    },
}

impl FuzzInteropStrategy {
    pub fn to_plan_strategy(&self) -> Option<PlanStrategy> {
        match self {
            Self::Selector { selector } => SelectorQuery::new(selector.clone())
                .ok()
                .map(PlanStrategy::css_selector),
            Self::DelimiterPair {
                start,
                end,
                regex_mode,
                include_start,
                include_end,
                flags,
            } => {
                let start = SliceBoundary::new(start.clone()).ok()?;
                let end = SliceBoundary::new(end.clone()).ok()?;
                let mode = if *regex_mode {
                    DelimiterMode::Regex
                } else {
                    DelimiterMode::Literal
                };
                let flags = if matches!(mode, DelimiterMode::Regex) {
                    flags.to_interop_flags()
                } else {
                    Vec::new()
                };

                Some(PlanStrategy::delimiter_pair(
                    start,
                    end,
                    mode,
                    *include_start,
                    *include_end,
                    flags,
                ))
            }
        }
    }
}

fn non_zero_index(raw: u8) -> std::num::NonZeroUsize {
    std::num::NonZeroUsize::new((raw as usize % 4) + 1).expect("non-zero index")
}

fn regex_flags_string(flags: FuzzRegexFlags) -> String {
    let mut rendered = String::new();
    if flags.case_insensitive {
        rendered.push('i');
    }
    if flags.multi_line {
        rendered.push('m');
    }
    if flags.dot_matches_new_line {
        rendered.push('s');
    }
    if flags.swap_greed {
        rendered.push('U');
    }
    if flags.ignore_whitespace {
        rendered.push('x');
    }
    rendered
}
