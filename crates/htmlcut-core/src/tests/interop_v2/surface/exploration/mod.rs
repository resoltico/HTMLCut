use super::*;
use crate::interop::v2::{
    ElementNamespace, ElementTargetHint, ExplorationAttribute, ExplorationErrorCode,
    ExplorationOptions, ExplorationTruncationReason, SelectorStability, SelectorStabilityReason,
    TargetResolutionOptions, explore, normalized_dom_text_digest, resolve_target_and_propose,
};

#[test]
fn exploration_is_paged_snapshot_bound_and_deterministic() {
    let source = HtmlInput::new(
        "target-exploration",
        "<main id=\"content\"><p class=\"lead\">One</p><p>Two</p></main>",
    )
    .expect("source");
    let document = prepare_document(source, PreparationLimits::default()).expect("prepared");
    let first_options = ExplorationOptions {
        max_elements: NonZeroU32::new(2).expect("non-zero"),
        ..ExplorationOptions::default()
    };

    let first = explore(&document, &first_options).expect("first page");
    assert_eq!(first.schema_name, "htmlcut.exploration_result");
    assert_eq!(first.elements.len(), 2);
    assert_eq!(
        first.truncation_reason,
        Some(ExplorationTruncationReason::ElementLimit)
    );
    assert!(first.next_cursor.is_some());
    assert!(first.elements[0].path.starts_with("html:html[1]"));
    assert_eq!(
        first.elements[0].element_name.namespace,
        ElementNamespace::Html
    );

    let mut second_options = first_options.clone();
    second_options.cursor = first.next_cursor.clone();
    let second = explore(&document, &second_options).expect("second page");
    assert!(!second.elements.is_empty());
    assert_ne!(first.elements[0].path, second.elements[0].path);

    let mut changed_options = second_options.clone();
    changed_options.preview_bytes = NonZeroU32::new(128).expect("non-zero");
    let changed_error = explore(&document, &changed_options).expect_err("changed cursor options");
    assert_eq!(
        changed_error.error_code,
        ExplorationErrorCode::CursorOptionsMismatch
    );

    let other_document = prepare_document(
        HtmlInput::new("other", "<main><p>Other</p></main>").expect("source"),
        PreparationLimits::default(),
    )
    .expect("prepared");
    let other_error = explore(&other_document, &second_options).expect_err("cross snapshot cursor");
    assert_eq!(
        other_error.error_code,
        ExplorationErrorCode::CursorSnapshotMismatch
    );
}

#[test]
fn target_resolution_requires_path_name_fingerprint_and_budget_to_agree() {
    let source = HtmlInput::new(
        "target-resolution",
        "<main data-surface=\"story\"><p>One</p></main><aside>Other</aside>",
    )
    .expect("source");
    let document = prepare_document(source, PreparationLimits::default()).expect("prepared");
    let exploration = explore(&document, &ExplorationOptions::default()).expect("exploration");
    let main = exploration
        .elements
        .iter()
        .find(|element| element.path.ends_with("html:main[1]"))
        .expect("main descriptor");
    let hint = ElementTargetHint {
        path_profile: "htmlcut.dom_path.v1".to_owned(),
        path: main.path.clone(),
        element_name: main.element_name.clone(),
        normalized_text_digest_sha256: normalized_dom_text_digest("One"),
        semantic_attributes: vec![ExplorationAttribute {
            name: "data-surface".to_owned(),
            value: "story".to_owned(),
            value_truncated: false,
        }],
    };
    let resolved =
        resolve_target_and_propose(&document, &hint, &TargetResolutionOptions::default())
            .expect("resolved target");
    assert_eq!(resolved.path, hint.path);
    assert!(!resolved.selector_proposals.is_empty());

    let mut wrong_name = hint.clone();
    wrong_name.element_name.local_name = "aside".to_owned();
    assert_eq!(
        resolve_target_and_propose(&document, &wrong_name, &TargetResolutionOptions::default())
            .expect_err("wrong name")
            .error_code,
        ExplorationErrorCode::TargetNotFound
    );

    let mut wrong_digest = hint.clone();
    wrong_digest.normalized_text_digest_sha256 = normalized_dom_text_digest("Other");
    assert_eq!(
        resolve_target_and_propose(
            &document,
            &wrong_digest,
            &TargetResolutionOptions::default()
        )
        .expect_err("wrong digest")
        .error_code,
        ExplorationErrorCode::TargetNotFound
    );

    let exhausted = TargetResolutionOptions {
        max_work_units: NonZeroU32::new(1).expect("non-zero"),
        ..TargetResolutionOptions::default()
    };
    assert_eq!(
        resolve_target_and_propose(&document, &hint, &exhausted)
            .expect_err("work exhaustion")
            .error_code,
        ExplorationErrorCode::TargetResolutionLimitExceeded
    );
}

#[test]
fn target_resolution_rejects_oversized_text_before_fingerprinting() {
    let source = HtmlInput::new(
        "target-fingerprint-limit",
        format!("<main>{}</main>", "x".repeat(1_048_577)),
    )
    .expect("source");
    let document = prepare_document(source, PreparationLimits::default()).expect("prepared");
    let hint = ElementTargetHint {
        path_profile: "htmlcut.dom_path.v1".to_owned(),
        path: "html:html[1]/html:body[1]/html:main[1]".to_owned(),
        element_name: crate::interop::v2::ElementName {
            namespace: ElementNamespace::Html,
            local_name: "main".to_owned(),
        },
        normalized_text_digest_sha256: "0".repeat(64),
        semantic_attributes: Vec::new(),
    };
    assert_eq!(
        resolve_target_and_propose(&document, &hint, &TargetResolutionOptions::default())
            .expect_err("oversized target text")
            .error_code,
        ExplorationErrorCode::TargetFingerprintLimitExceeded
    );
}

#[test]
fn exploration_proposals_are_unique_and_explain_generated_identifier_downgrades() {
    let source = HtmlInput::new(
        "target-proposals",
        "<main id=\"content\"><p class=\"story lead\">One</p></main><aside id=\"css-991827364551\">Side</aside>",
    )
    .expect("source");
    let document = prepare_document(source, PreparationLimits::default()).expect("prepared");
    let result = explore(&document, &ExplorationOptions::default()).expect("exploration");
    let content = result
        .elements
        .iter()
        .find(|element| element.path.ends_with("html:main[1]"))
        .expect("main descriptor");
    assert!(content.selector_proposals.iter().any(|proposal| {
        proposal.selector.as_str() == "#content"
            && proposal.stability == SelectorStability::Semantic
            && proposal
                .reasons
                .contains(&SelectorStabilityReason::UniqueId)
            && proposal
                .reasons
                .contains(&SelectorStabilityReason::UniqueOnSnapshot)
    }));

    let generated = result
        .elements
        .iter()
        .find(|element| element.path.ends_with("html:aside[1]"))
        .expect("aside descriptor");
    let generated_id = generated
        .selector_proposals
        .iter()
        .find(|proposal| proposal.selector.as_str() == "#css-991827364551")
        .expect("generated ID proposal");
    assert_ne!(generated_id.stability, SelectorStability::Semantic);
    assert!(
        generated_id
            .reasons
            .contains(&SelectorStabilityReason::GeneratedLookingIdentifier)
    );
}

#[test]
fn exploration_proposals_classify_semantic_attributes_and_generated_classes() {
    let document = prepare_document(
        HtmlInput::new(
            "proposal-ranking",
            "<main><section itemprop=\"articleBody\" role=\"main\" aria-label=\"Story\" data-hook=\"content\" class=\"story lead\">One</section><aside class=\"css-991827364551\">Two</aside></main>",
        )
        .expect("source"),
        PreparationLimits::default(),
    )
    .expect("prepared");
    let result = explore(
        &document,
        &ExplorationOptions {
            max_proposals_per_element: NonZeroU32::new(8).expect("non-zero"),
            ..ExplorationOptions::default()
        },
    )
    .expect("exploration");
    let section = result
        .elements
        .iter()
        .find(|element| element.element_name.local_name == "section")
        .expect("section descriptor");
    assert!(section.selector_proposals.iter().any(|proposal| {
        proposal
            .reasons
            .contains(&SelectorStabilityReason::MicrodataAttribute)
    }));
    assert!(section.selector_proposals.iter().any(|proposal| {
        proposal
            .reasons
            .contains(&SelectorStabilityReason::AriaAttribute)
    }));
    assert!(section.selector_proposals.iter().any(|proposal| {
        proposal.selector.as_str().contains("data-hook")
            && proposal.stability == SelectorStability::StableAttribute
    }));
    assert!(section.selector_proposals.iter().any(|proposal| {
        proposal
            .reasons
            .contains(&SelectorStabilityReason::StableClass)
            && proposal.stability == SelectorStability::ClassBased
    }));

    let aside = result
        .elements
        .iter()
        .find(|element| element.element_name.local_name == "aside")
        .expect("aside descriptor");
    assert!(aside.selector_proposals.iter().any(|proposal| {
        proposal
            .reasons
            .contains(&SelectorStabilityReason::GeneratedLookingClass)
            && proposal.stability == SelectorStability::Structural
    }));
}

#[test]
fn exploration_returns_partial_proof_evidence_and_advancing_cursor_when_work_exhausts() {
    let source = HtmlInput::new(
        "target-work",
        "<main id=\"content\">One</main><aside>Two</aside>",
    )
    .expect("source");
    let document = prepare_document(source, PreparationLimits::default()).expect("prepared");
    let options = ExplorationOptions {
        cursor: Some(crate::interop::v2::exploration_cursor_for_tests(
            &document,
            &ExplorationOptions {
                max_work_units: NonZeroU32::new(16).expect("non-zero"),
                ..ExplorationOptions::default()
            },
            NonZeroU32::new(4).expect("main element ordinal"),
        )),
        max_work_units: NonZeroU32::new(16).expect("non-zero"),
        ..ExplorationOptions::default()
    };
    let result = explore(&document, &options).expect("exploration");

    assert_eq!(
        result.truncation_reason,
        Some(ExplorationTruncationReason::WorkLimit)
    );
    assert!(result.next_cursor.is_some());
    assert!(
        result
            .elements
            .iter()
            .any(|element| element.proposal_work_truncated)
    );
}

#[test]
fn exploration_filters_sensitive_attributes_and_marks_value_truncation() {
    let long_value = "x".repeat(600);
    let source = HtmlInput::new(
        "target-attributes",
        format!(
            "<form><input value=\"secret\" nonce=\"nonce-value\" data-long=\"{long_value}\" aria-label=\"Search\"></form>"
        ),
    )
    .expect("source");
    let document = prepare_document(source, PreparationLimits::default()).expect("prepared");
    let result = explore(&document, &ExplorationOptions::default()).expect("exploration");
    let input = result
        .elements
        .iter()
        .find(|element| element.element_name.local_name == "input")
        .expect("input descriptor");

    assert!(
        input
            .attributes
            .iter()
            .any(|attribute| attribute.name == "aria-label")
    );
    assert!(
        input
            .attributes
            .iter()
            .all(|attribute| attribute.name != "value")
    );
    assert!(
        input
            .attributes
            .iter()
            .all(|attribute| attribute.name != "nonce")
    );
    let data_long = input
        .attributes
        .iter()
        .find(|attribute| attribute.name == "data-long")
        .expect("data attribute");
    assert_eq!(data_long.value.len(), 512);
    assert!(data_long.value_truncated);
}

#[test]
fn exploration_marks_attribute_count_truncation_without_leaking_disallowed_attributes() {
    let attributes = (0..33)
        .map(|index| format!("data-key-{index}=\"value-{index}\""))
        .collect::<Vec<_>>()
        .join(" ");
    let document = prepare_document(
        HtmlInput::new(
            "attribute-count-limit",
            format!("<main {attributes} nonce=\"secret\" value=\"secret\">One</main>"),
        )
        .expect("source"),
        PreparationLimits::default(),
    )
    .expect("prepared");
    let result = explore(&document, &ExplorationOptions::default()).expect("exploration");
    let main = result
        .elements
        .iter()
        .find(|element| element.element_name.local_name == "main")
        .expect("main descriptor");
    assert_eq!(main.attributes.len(), 32);
    assert!(main.attributes_truncated);
    assert!(
        main.attributes
            .iter()
            .all(|attribute| attribute.name.starts_with("data-key-"))
    );
}

#[test]
fn exploration_omits_an_overlong_element_name_without_truncating_its_identity() {
    let tag_name = "x".repeat(257);
    let document = prepare_document(
        HtmlInput::new(
            "overlong-element-name",
            format!("<{tag_name}>One</{tag_name}>"),
        )
        .expect("source"),
        PreparationLimits::default(),
    )
    .expect("prepared");
    let result = explore(&document, &ExplorationOptions::default()).expect("exploration");
    assert!(result.omitted_element_counts.iter().any(|omission| {
        omission.reason == crate::interop::v2::ExplorationOmissionReason::ElementNameTooLong
            && omission.count > 0
    }));
    assert!(
        result
            .elements
            .iter()
            .all(|element| element.element_name.local_name.len() <= 256)
    );
}

#[test]
fn exploration_preview_stops_at_its_declared_byte_bound() {
    let source = HtmlInput::new(
        "target-preview",
        format!("<main>{}</main>", "x".repeat(8_192)),
    )
    .expect("source");
    let document = prepare_document(source, PreparationLimits::default()).expect("prepared");
    let options = ExplorationOptions {
        preview_bytes: NonZeroU32::new(32).expect("non-zero"),
        ..ExplorationOptions::default()
    };
    let result = explore(&document, &options).expect("exploration");
    let main = result
        .elements
        .iter()
        .find(|element| element.element_name.local_name == "main")
        .expect("main descriptor");
    assert_eq!(main.text_preview.len(), 32);
    assert!(main.text_preview_truncated);
}

#[test]
fn exploration_and_target_options_reject_published_hard_maximums() {
    let source = HtmlInput::new("invalid-exploration-limits", "<main>One</main>").expect("source");
    let document = prepare_document(source, PreparationLimits::default()).expect("prepared");
    let exploration_options = ExplorationOptions {
        max_elements: NonZeroU32::new(1_001).expect("non-zero"),
        ..ExplorationOptions::default()
    };
    assert_eq!(
        explore(&document, &exploration_options)
            .expect_err("exploration limits")
            .error_code,
        ExplorationErrorCode::InvalidLimits
    );

    for invalid_options in [
        ExplorationOptions {
            max_work_units: NonZeroU32::new(5_000_001).expect("non-zero"),
            ..ExplorationOptions::default()
        },
        ExplorationOptions {
            max_proposals_per_element: NonZeroU32::new(17).expect("non-zero"),
            ..ExplorationOptions::default()
        },
        ExplorationOptions {
            preview_bytes: NonZeroU32::new(513).expect("non-zero"),
            ..ExplorationOptions::default()
        },
    ] {
        assert!(invalid_options.validate().is_err());
    }

    let oversized_target_options = TargetResolutionOptions {
        max_work_units: NonZeroU32::new(5_000_001).expect("non-zero"),
        max_proposals: NonZeroU32::new(1).expect("non-zero"),
    };
    assert!(oversized_target_options.validate().is_err());
    assert!(
        TargetResolutionOptions {
            max_work_units: NonZeroU32::new(1).expect("non-zero"),
            max_proposals: NonZeroU32::new(17).expect("non-zero"),
        }
        .validate()
        .is_err()
    );
}

mod pagination;
mod proposal_proofs;
mod resource_limits;
mod target_contract;

#[test]
fn exploration_preserves_svg_and_mathml_namespaced_paths() {
    let document = prepare_document(
        HtmlInput::new(
            "namespaced-paths",
            "<main><svg><linearGradient></linearGradient><linearGradient id=\"paint\"></linearGradient></svg><math><mi>x</mi></math></main>",
        )
        .expect("source"),
        PreparationLimits::default(),
    )
    .expect("prepared");
    let result = explore(&document, &ExplorationOptions::default()).expect("exploration");
    let gradient = result
        .elements
        .iter()
        .find(|element| element.element_name.local_name == "linearGradient")
        .expect("SVG descriptor");
    assert_eq!(gradient.element_name.namespace, ElementNamespace::Svg);
    assert_eq!(
        result
            .elements
            .iter()
            .filter(|element| element.element_name.local_name == "linearGradient")
            .map(|element| element.path.as_str())
            .collect::<Vec<_>>(),
        vec![
            "html:html[1]/html:body[1]/html:main[1]/svg:svg[1]/svg:linearGradient[1]",
            "html:html[1]/html:body[1]/html:main[1]/svg:svg[1]/svg:linearGradient[2]",
        ]
    );

    let identifier = result
        .elements
        .iter()
        .find(|element| element.element_name.local_name == "mi")
        .expect("MathML descriptor");
    assert_eq!(identifier.element_name.namespace, ElementNamespace::Mathml);
    assert!(identifier.path.contains("mathml:math[1]/mathml:mi[1]"));
}

#[test]
fn exploration_omits_an_overlong_path_without_truncating_its_identity() {
    let depth = 1_000usize;
    let tag_name = "abcdefghij";
    let mut markup = format!("<{tag_name}>").repeat(depth);
    markup.push_str("leaf");
    markup.push_str(&format!("</{tag_name}>").repeat(depth));
    let limits = PreparationLimits::new(
        NonZeroU32::new(50 * 1024 * 1024).expect("non-zero"),
        NonZeroU32::new(4_096).expect("non-zero"),
        NonZeroU32::new(4_096).expect("non-zero"),
    )
    .expect("limits");
    let document = prepare_document(
        HtmlInput::new("overlong-path", markup).expect("source"),
        limits,
    )
    .expect("prepared");
    let result = explore(
        &document,
        &ExplorationOptions {
            max_elements: NonZeroU32::new(1_000).expect("non-zero"),
            max_work_units: NonZeroU32::new(5_000_000).expect("non-zero"),
            max_proposals_per_element: NonZeroU32::new(1).expect("non-zero"),
            preview_bytes: NonZeroU32::new(16).expect("non-zero"),
            cursor: Some(crate::interop::v2::exploration_cursor_for_tests(
                &document,
                &ExplorationOptions {
                    max_elements: NonZeroU32::new(1_000).expect("non-zero"),
                    max_work_units: NonZeroU32::new(5_000_000).expect("non-zero"),
                    max_proposals_per_element: NonZeroU32::new(1).expect("non-zero"),
                    preview_bytes: NonZeroU32::new(16).expect("non-zero"),
                    cursor: None,
                },
                NonZeroU32::new(1_003).expect("deep element ordinal"),
            )),
        },
    )
    .expect("exploration page");
    assert!(
        result.omitted_element_counts.iter().any(|omission| {
            omission.reason == crate::interop::v2::ExplorationOmissionReason::PathTooLong
                && omission.count > 0
        }),
        "omissions: {:?}; truncation: {:?}; elements: {:?}",
        result.omitted_element_counts,
        result.truncation_reason,
        result
            .elements
            .iter()
            .map(|element| &element.path)
            .collect::<Vec<_>>()
    );
}
