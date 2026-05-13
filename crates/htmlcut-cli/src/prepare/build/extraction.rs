use htmlcut_core::{
    BoundaryRetention, ExtractionRequest, ExtractionSpec, OutputOptions, RenderingOptions,
    SelectorQuery, SliceBoundary, SlicePatternSpec, SliceSpec,
};

use crate::args::{CliPatternMode, SelectionArgs, SourceArgs};
use crate::error::{CliError, usage_error};
use crate::model::CliErrorCode;

use super::output::resolve_regex_flags;
use super::selection::resolve_selection_spec;
use super::source::build_source_request;
use crate::prepare::RequestBuildOptions;

pub(crate) enum StrategyArgs {
    Select {
        css: String,
    },
    Slice {
        from: String,
        to: String,
        pattern: CliPatternMode,
        regex_flags: Option<String>,
        boundary_retention: BoundaryRetention,
    },
}

pub(crate) fn build_extraction_request(
    strategy_args: StrategyArgs,
    source_args: &SourceArgs,
    selection_args: &SelectionArgs,
    options: RequestBuildOptions,
) -> Result<ExtractionRequest, CliError> {
    let source = build_source_request(source_args)?;
    let selection = resolve_selection_spec(selection_args)?;
    let extraction = match strategy_args {
        StrategyArgs::Select { css } => ExtractionSpec::selector(parse_selector_query(css)?),
        StrategyArgs::Slice {
            from,
            to,
            pattern,
            regex_flags,
            boundary_retention,
        } => {
            let from = parse_slice_boundary(from)?;
            let to = parse_slice_boundary(to)?;
            let pattern = build_slice_pattern(pattern, regex_flags, from, to)?;
            ExtractionSpec::slice(SliceSpec {
                pattern,
                boundary_retention,
            })
        }
    }
    .with_selection(selection)
    .with_value(options.value);

    let mut request = ExtractionRequest::new(source, extraction);
    request.output = OutputOptions {
        rendering: RenderingOptions {
            whitespace: options.whitespace,
            rewrite_urls: options.rewrite_urls,
        },
        include_source_text: options.include_source_text,
        include_html: true,
        include_text: true,
        preview_chars: options.preview_chars,
    };
    Ok(request)
}

fn build_slice_pattern(
    pattern: CliPatternMode,
    regex_flags: Option<String>,
    from: SliceBoundary,
    to: SliceBoundary,
) -> Result<SlicePatternSpec, CliError> {
    match resolve_regex_flags(pattern, regex_flags)? {
        Some(flags) => Ok(SlicePatternSpec::regex(from, to, flags)),
        None => Ok(SlicePatternSpec::literal(from, to)),
    }
}

fn parse_selector_query(selector: String) -> Result<SelectorQuery, CliError> {
    SelectorQuery::new(selector)
        .map_err(|error| usage_error(CliErrorCode::SelectorInvalid, error.to_string()))
}

fn parse_slice_boundary(boundary: String) -> Result<SliceBoundary, CliError> {
    SliceBoundary::new(boundary)
        .map_err(|error| usage_error(CliErrorCode::SliceBoundaryInvalid, error.to_string()))
}
