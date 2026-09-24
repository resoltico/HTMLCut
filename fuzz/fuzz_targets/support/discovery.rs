use std::num::NonZeroU32;

use arbitrary::Arbitrary;
use htmlcut_core::interop::v2::{
    ElementName, ElementTargetHint, ExplorationAttribute, ExplorationOptions, HtmlInput,
    PreparationLimits, TargetResolutionOptions, explore, normalized_dom_text_digest,
    prepare_document, resolve_target_and_propose,
};

#[derive(Arbitrary, Debug)]
pub struct DiscoveryInput {
    text: String,
    max_elements: u16,
    max_work_units: u16,
    max_proposals: u8,
    preview_bytes: u16,
    mismatch_digest: bool,
    add_semantic_attribute: bool,
}

pub fn drive(input: DiscoveryInput) {
    let text = bounded_text(&input.text, 4_096);
    let html = format!(
        "<main><div data-fuzz-anchor=\"stable\">{}</div></main>",
        escape_html(&text)
    );
    let Ok(html_input) = HtmlInput::new("prepared-discovery-fuzz", html) else {
        return;
    };
    let Ok(document) = prepare_document(html_input, PreparationLimits::default()) else {
        return;
    };

    let options = ExplorationOptions {
        cursor: None,
        max_elements: non_zero(input.max_elements as u32 % 32 + 1),
        max_work_units: non_zero(input.max_work_units as u32 % 20_000 + 1),
        max_proposals_per_element: non_zero(input.max_proposals as u32 % 16 + 1),
        preview_bytes: non_zero(input.preview_bytes as u32 % 512 + 1),
    };
    let Ok(result) = explore(&document, &options) else {
        return;
    };
    let Some(descriptor) = result
        .elements
        .iter()
        .find(|descriptor| descriptor.element_name.local_name == "div")
    else {
        return;
    };

    let mut digest = normalized_dom_text_digest(&text);
    if input.mismatch_digest {
        digest.replace_range(0..1, "0");
    }
    let semantic_attributes = input.add_semantic_attribute.then(|| {
        vec![ExplorationAttribute {
            name: "data-fuzz-anchor".to_owned(),
            value: "stable".to_owned(),
            value_truncated: false,
        }]
    });
    let hint = ElementTargetHint {
        path_profile: "htmlcut.dom_path.v1".to_owned(),
        path: descriptor.path.clone(),
        element_name: ElementName {
            namespace: descriptor.element_name.namespace,
            local_name: descriptor.element_name.local_name.clone(),
        },
        normalized_text_digest_sha256: digest,
        semantic_attributes: semantic_attributes.unwrap_or_default(),
    };
    let target_options = TargetResolutionOptions {
        max_work_units: non_zero(input.max_work_units as u32 % 20_000 + 1),
        max_proposals: non_zero(input.max_proposals as u32 % 16 + 1),
    };
    let _ = resolve_target_and_propose(&document, &hint, &target_options);
}

fn non_zero(value: u32) -> NonZeroU32 {
    NonZeroU32::new(value).expect("bounded fuzz value is non-zero")
}

fn bounded_text(value: &str, maximum_bytes: usize) -> String {
    let mut end = value.len().min(maximum_bytes);
    while end > 0 && !value.is_char_boundary(end) {
        end -= 1;
    }
    value[..end].to_owned()
}

fn escape_html(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}
