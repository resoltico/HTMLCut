use arbitrary::Arbitrary;
use htmlcut_core::{
    ExtractionRequest, ExtractionSpec, SliceBoundary, SliceSpec, SourceRequest, extract,
    preview_extraction,
};

use crate::request_common::{
    FuzzNormalization, FuzzSelection, FuzzValueKind, runtime_for_html, sample_base_url,
};

#[derive(Arbitrary, Debug)]
pub struct SliceInput {
    html: String,
    start: String,
    end: String,
    regex_mode: bool,
    flags: FuzzRegexFlags,
    include_start: bool,
    include_end: bool,
    value_kind: FuzzValueKind,
    selection: FuzzSelection,
    normalization: FuzzNormalization,
}

#[derive(Arbitrary, Clone, Copy, Debug)]
struct FuzzRegexFlags {
    case_insensitive: bool,
    multi_line: bool,
    dot_matches_new_line: bool,
    swap_greed: bool,
    ignore_whitespace: bool,
}

pub fn drive(input: SliceInput) {
    let Ok(start) = SliceBoundary::new(input.start) else {
        return;
    };
    let Ok(end) = SliceBoundary::new(input.end) else {
        return;
    };

    let slice = if input.regex_mode {
        SliceSpec::regex(start, end, regex_flags_string(input.flags))
    } else {
        SliceSpec::new(start, end)
    }
    .with_boundary_inclusion(input.include_start, input.include_end);

    let mut request = ExtractionRequest::new(
        SourceRequest::memory("fuzz", &input.html).with_base_url(sample_base_url()),
        ExtractionSpec::slice(slice),
    );
    request.extraction = request
        .extraction
        .clone()
        .with_selection(input.selection.to_selection_spec())
        .with_value(input.value_kind.to_value_spec());
    input.normalization.apply_to_request(&mut request);

    let runtime = runtime_for_html(&input.html);
    let _ = preview_extraction(&request, &runtime);
    let _ = extract(&request, &runtime);
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
