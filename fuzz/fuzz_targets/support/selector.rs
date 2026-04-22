use arbitrary::Arbitrary;
use htmlcut_core::{
    ExtractionRequest, ExtractionSpec, SelectorQuery, SourceRequest, extract, preview_extraction,
};

use crate::request_common::{
    FuzzNormalization, FuzzSelection, FuzzValueKind, runtime_for_html, sample_base_url,
};

#[derive(Arbitrary, Debug)]
pub struct SelectorInput {
    html: String,
    selector: String,
    value_kind: FuzzValueKind,
    selection: FuzzSelection,
    normalization: FuzzNormalization,
}

pub fn drive(input: SelectorInput) {
    let Ok(selector) = SelectorQuery::new(input.selector) else {
        return;
    };

    let mut request = ExtractionRequest::new(
        SourceRequest::memory("fuzz", &input.html).with_base_url(sample_base_url()),
        ExtractionSpec::selector(selector),
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
