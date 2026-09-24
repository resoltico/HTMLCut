use std::num::NonZeroUsize;

use crate::{
    Diagnostic, ExtractionRequest, ExtractionSpec, OutputOptions, RenderingOptions, SelectionSpec,
    SelectorQuery, SlicePatternSpec, SliceSpec, SourceRequest, ValueSpec, WhitespaceMode,
    extract::{CompiledSlicePatterns, validate_selector_query},
};

use super::super::stable_json::digest_stable_json;
use super::super::{
    ContractError, DelimiterMode, Plan, PlanStrategy, RegexFlag, Selection, TextWhitespace,
};

/// Source-independent grammar compiled from one declarative plan.
pub(super) enum CompiledStrategy {
    /// CSS grammar prepared by the maintained selector engine.
    CssSelector(scraper::Selector),
    /// Delimiter grammar prepared from literal or regex boundaries.
    DelimiterPair {
        /// Exact declarative slice contract used for result projection.
        slice: SliceSpec,
        /// Compiled literal or regex boundary matchers.
        patterns: CompiledSlicePatterns,
    },
}

impl CompiledStrategy {
    /// Returns the strategy family retained by this prepared grammar.
    pub(super) fn kind(&self) -> super::super::StrategyKind {
        match self {
            Self::CssSelector(selector) => {
                let _ = selector;
                super::super::StrategyKind::CssSelector
            }
            Self::DelimiterPair { slice, patterns } => {
                let _ = (slice, patterns);
                super::super::StrategyKind::DelimiterPair
            }
        }
    }
}

/// Compiles all source-independent CSS and regex grammar for one plan.
pub(super) fn compile_strategy(
    plan: &Plan,
    request: &ExtractionRequest,
) -> Result<CompiledStrategy, Diagnostic> {
    match &plan.strategy {
        PlanStrategy::CssSelector { selector } => {
            let query = SelectorQuery::new(selector.as_str().to_owned())
                .expect("validated interop selector text must remain non-empty");
            validate_selector_query(&query).map(CompiledStrategy::CssSelector)
        }
        PlanStrategy::DelimiterPair { .. } => {
            let slice = request
                .extraction
                .slice_spec()
                .expect("delimiter plan must compile into a core slice request");
            let patterns = CompiledSlicePatterns::compile(slice)?;
            Ok(CompiledStrategy::DelimiterPair {
                slice: slice.clone(),
                patterns,
            })
        }
    }
}

pub(super) fn exact_plan_digest_sha256(plan: &Plan) -> Result<String, ContractError> {
    digest_stable_json(plan)
}

pub(super) fn compile_request(plan: &Plan) -> ExtractionRequest {
    let extraction = match &plan.strategy {
        PlanStrategy::CssSelector { selector } => ExtractionSpec::selector(
            crate::SelectorQuery::new(selector.as_str().to_owned())
                .expect("validated interop selector must compile into a core selector query"),
        ),
        PlanStrategy::DelimiterPair {
            start,
            end,
            mode,
            boundary_retention,
            flags,
        } => ExtractionSpec::slice(SliceSpec {
            pattern: match mode {
                DelimiterMode::Literal => SlicePatternSpec::literal(
                    crate::SliceBoundary::new(start.as_str().to_owned()).expect(
                        "validated interop start boundary must compile into a core slice boundary",
                    ),
                    crate::SliceBoundary::new(end.as_str().to_owned()).expect(
                        "validated interop end boundary must compile into a core slice boundary",
                    ),
                ),
                DelimiterMode::Regex => SlicePatternSpec::regex(
                    crate::SliceBoundary::new(start.as_str().to_owned()).expect(
                        "validated interop start boundary must compile into a core slice boundary",
                    ),
                    crate::SliceBoundary::new(end.as_str().to_owned()).expect(
                        "validated interop end boundary must compile into a core slice boundary",
                    ),
                    compile_regex_flags(flags),
                ),
            },
            boundary_retention: crate::BoundaryRetention::from_flags(
                boundary_retention.includes_start(),
                boundary_retention.includes_end(),
            ),
        }),
    }
    .with_selection(compile_selection(&plan.selection))
    .with_value(ValueSpec::Structured);

    let mut request = ExtractionRequest::new(SourceRequest::memory("prepared", ""), extraction);
    request.output = OutputOptions {
        rendering: RenderingOptions {
            whitespace: match plan.rendering.whitespace {
                TextWhitespace::Rendered => WhitespaceMode::Rendered,
                TextWhitespace::Normalize => WhitespaceMode::Normalize,
            },
            rewrite_urls: plan.rendering.rewrite_urls,
        },
        include_html: false,
        include_text: false,
        ..OutputOptions::default()
    };
    request
}

fn compile_selection(selection: &Selection) -> SelectionSpec {
    // Every supported Rust target represents at least 32 bits in `usize`, while the v2 wire
    // contract admits only non-zero `u32` ordinals. This proof makes the internal cast exact.
    const _: () = assert!(usize::BITS >= u32::BITS);
    match selection {
        Selection::Single => SelectionSpec::single(),
        Selection::First => SelectionSpec::First,
        Selection::Nth { index } => SelectionSpec::nth(
            NonZeroUsize::new(index.get() as usize)
                .expect("non-zero v2 selection index must remain non-zero"),
        ),
        Selection::All => SelectionSpec::All,
    }
}

pub(super) fn compile_regex_flags(flags: &[RegexFlag]) -> String {
    let mut compiled = String::new();
    for flag in flags {
        compiled.push(match flag {
            RegexFlag::CaseInsensitive => 'i',
            RegexFlag::MultiLine => 'm',
            RegexFlag::DotMatchesNewLine => 's',
            RegexFlag::SwapGreed => 'U',
            RegexFlag::IgnoreWhitespace => 'x',
        });
    }
    compiled
}
