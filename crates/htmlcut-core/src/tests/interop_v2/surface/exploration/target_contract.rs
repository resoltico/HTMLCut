use std::num::NonZeroU32;

use super::*;
use crate::interop::v2::{
    ElementNamespace, ElementTargetHint, ExplorationAttribute, ExplorationErrorCode,
    ExplorationOptions, TargetResolutionOptions, is_lower_sha256_for_tests,
    normalized_dom_text_digest, normalized_dom_text_nodes_for_tests, resolve_target_and_propose,
};

#[test]
fn target_resolution_rejects_invalid_contract_and_semantic_attribute_mismatch() {
    let document = prepare_document(
        HtmlInput::new("target-contract", "<main data-surface=\"story\">One</main>")
            .expect("source"),
        PreparationLimits::default(),
    )
    .expect("prepared");
    let valid_hint = ElementTargetHint {
        path_profile: "htmlcut.dom_path.v1".to_owned(),
        path: "html:html[1]/html:body[1]/html:main[1]".to_owned(),
        element_name: crate::interop::v2::ElementName {
            namespace: ElementNamespace::Html,
            local_name: "main".to_owned(),
        },
        normalized_text_digest_sha256: normalized_dom_text_digest("One"),
        semantic_attributes: Vec::new(),
    };
    let mut invalid_profile = valid_hint.clone();
    invalid_profile.path_profile = "browser.path.v1".to_owned();
    assert_eq!(
        resolve_target_and_propose(
            &document,
            &invalid_profile,
            &TargetResolutionOptions::default()
        )
        .expect_err("invalid target contract")
        .error_code,
        ExplorationErrorCode::TargetNotFound
    );

    let mut conflicting_attribute = valid_hint.clone();
    conflicting_attribute.semantic_attributes = vec![ExplorationAttribute {
        name: "data-surface".to_owned(),
        value: "other".to_owned(),
        value_truncated: false,
    }];
    assert_eq!(
        resolve_target_and_propose(
            &document,
            &conflicting_attribute,
            &TargetResolutionOptions::default(),
        )
        .expect_err("semantic attribute mismatch")
        .error_code,
        ExplorationErrorCode::TargetNotFound
    );

    let mut wrong_path = conflicting_attribute;
    wrong_path.semantic_attributes.clear();
    wrong_path.path = "html:html[1]/html:body[1]/html:aside[1]".to_owned();
    assert_eq!(
        resolve_target_and_propose(&document, &wrong_path, &TargetResolutionOptions::default())
            .expect_err("path mismatch")
            .error_code,
        ExplorationErrorCode::TargetNotFound
    );

    let mut truncated_attribute = valid_hint;
    truncated_attribute.semantic_attributes = vec![ExplorationAttribute {
        name: "data-surface".to_owned(),
        value: "story".to_owned(),
        value_truncated: true,
    }];
    assert_eq!(
        resolve_target_and_propose(
            &document,
            &truncated_attribute,
            &TargetResolutionOptions::default(),
        )
        .expect_err("truncated target attribute")
        .error_code,
        ExplorationErrorCode::TargetNotFound
    );
}

#[test]
fn target_resolution_rejects_invalid_options_and_proposal_proof_exhaustion() {
    let document = prepare_document(
        HtmlInput::new("target-proof-limit", "<main id=\"content\">One</main>").expect("source"),
        PreparationLimits::default(),
    )
    .expect("prepared");
    let hint = ElementTargetHint {
        path_profile: "htmlcut.dom_path.v1".to_owned(),
        path: "html:html[1]/html:body[1]/html:main[1]".to_owned(),
        element_name: crate::interop::v2::ElementName {
            namespace: ElementNamespace::Html,
            local_name: "main".to_owned(),
        },
        normalized_text_digest_sha256: normalized_dom_text_digest("One"),
        semantic_attributes: Vec::new(),
    };
    let invalid_options = TargetResolutionOptions {
        max_work_units: NonZeroU32::new(5_000_001).expect("non-zero"),
        max_proposals: NonZeroU32::new(1).expect("non-zero"),
    };
    assert_eq!(
        resolve_target_and_propose(&document, &hint, &invalid_options)
            .expect_err("invalid target options")
            .error_code,
        ExplorationErrorCode::InvalidLimits
    );

    let proof_limited = TargetResolutionOptions {
        max_work_units: NonZeroU32::new(4).expect("non-zero"),
        max_proposals: NonZeroU32::new(1).expect("non-zero"),
    };
    assert_eq!(
        resolve_target_and_propose(&document, &hint, &proof_limited)
            .expect_err("proposal proof work exhaustion")
            .error_code,
        ExplorationErrorCode::TargetResolutionLimitExceeded
    );
}

#[test]
fn target_text_normalization_preserves_the_published_crlf_boundary_rule() {
    assert_eq!(
        normalized_dom_text_nodes_for_tests(["one\r\ntwo\rthree", "\r\nfour"]),
        Some("one\ntwo\nthree\nfour".to_owned())
    );
    assert_eq!(
        normalized_dom_text_nodes_for_tests(["tail\r"]),
        Some("tail\n".to_owned())
    );
    assert_eq!(
        normalized_dom_text_digest("one\r\ntwo\rthree\r\nfour"),
        normalized_dom_text_digest("one\ntwo\nthree\nfour")
    );
}

#[test]
fn target_fingerprint_matches_independent_domain_separated_golden_vectors() {
    assert_eq!(
        normalized_dom_text_digest("One"),
        "e163d4661874af8285350875496da6d42afb63bbe2c9e6f7d390c6c632aadfc6"
    );
    assert_eq!(
        normalized_dom_text_digest("Grüße 🌍"),
        "58ba7b57090f5b38b12cf279e885e786fd06f2c023fa0a4d0e3fb549dabfd50a"
    );
    let across_nodes = normalized_dom_text_nodes_for_tests(["one\r", "\ntwo\rthree", "\r\nfour"])
        .expect("bounded text");
    assert_eq!(across_nodes, "one\ntwo\nthree\nfour");
    assert_eq!(
        normalized_dom_text_digest(&across_nodes),
        "2cd4fdaa583dbc50c2f7719898423803c6f169a23ce713b300f73f9014759fb6"
    );
}

#[test]
fn target_text_normalization_refuses_each_carriage_return_path_beyond_the_byte_limit() {
    let at_limit = "x".repeat(1_048_576);
    assert_eq!(
        normalized_dom_text_nodes_for_tests([format!("{at_limit}\r\n")]),
        None,
        "a CRLF normalization step must enforce the bound before continuing"
    );
    assert_eq!(
        normalized_dom_text_nodes_for_tests([format!("{at_limit}\rx")]),
        None,
        "a lone CR normalization step must enforce the bound before continuing"
    );
    assert_eq!(
        normalized_dom_text_nodes_for_tests([format!("{at_limit}\r")]),
        None,
        "a final CR beyond the byte limit must be refused"
    );
    let multibyte_prefix = "x".repeat(1_048_574);
    assert_eq!(
        normalized_dom_text_nodes_for_tests([format!("{multibyte_prefix}\ré")]),
        None,
        "a normalized newline and following multibyte character must be budgeted as one unit"
    );
}

#[test]
fn target_text_normalization_accepts_each_path_at_the_exact_byte_limit() {
    const TARGET_TEXT_LIMIT: usize = 1_048_576;
    let at_limit = "x".repeat(TARGET_TEXT_LIMIT);
    assert!(
        normalized_dom_text_nodes_for_tests([at_limit.as_str()]).is_some(),
        "ordinary text at the public byte limit must remain representable"
    );

    let crlf_prefix = "x".repeat(TARGET_TEXT_LIMIT - 1);
    assert!(
        normalized_dom_text_nodes_for_tests([format!("{crlf_prefix}\r\n")]).is_some(),
        "a CRLF normalization step that lands exactly at the limit must be accepted"
    );

    let lone_cr_prefix = "x".repeat(TARGET_TEXT_LIMIT - 2);
    assert!(
        normalized_dom_text_nodes_for_tests([format!("{lone_cr_prefix}\rx")]).is_some(),
        "a lone-CR normalization step that lands exactly at the limit must be accepted"
    );

    let trailing_cr_prefix = "x".repeat(TARGET_TEXT_LIMIT - 1);
    assert!(
        normalized_dom_text_nodes_for_tests([format!("{trailing_cr_prefix}\r")]).is_some(),
        "a final carriage return that lands exactly at the limit must be accepted"
    );
}

#[test]
fn exploration_options_accept_each_published_maximum() {
    let options = ExplorationOptions {
        cursor: None,
        max_elements: NonZeroU32::new(1_000).expect("non-zero"),
        max_work_units: NonZeroU32::new(5_000_000).expect("non-zero"),
        max_proposals_per_element: NonZeroU32::new(16).expect("non-zero"),
        preview_bytes: NonZeroU32::new(512).expect("non-zero"),
    };
    assert!(options.validate().is_ok());
}

#[test]
fn target_resolution_options_accept_each_published_maximum() {
    let options = TargetResolutionOptions {
        max_work_units: NonZeroU32::new(5_000_000).expect("non-zero"),
        max_proposals: NonZeroU32::new(16).expect("non-zero"),
    };
    assert!(options.validate().is_ok());
}

#[test]
fn target_digest_validation_requires_exact_lowercase_sha256() {
    assert!(is_lower_sha256_for_tests(&normalized_dom_text_digest(
        "target"
    )));
    assert!(!is_lower_sha256_for_tests(&"f".repeat(63)));
    assert!(!is_lower_sha256_for_tests(&"F".repeat(64)));
    assert!(!is_lower_sha256_for_tests(&format!("{}g", "f".repeat(63))));
}

#[test]
fn target_resolution_refuses_an_element_outside_the_bounded_public_name_contract() {
    let local_name = "x".repeat(257);
    let document = prepare_document(
        HtmlInput::new(
            "overlong-target-name",
            format!("<{local_name}>One</{local_name}>"),
        )
        .expect("source"),
        PreparationLimits::default(),
    )
    .expect("prepared");
    let hint = ElementTargetHint {
        path_profile: "htmlcut.dom_path.v1".to_owned(),
        path: format!("html:html[1]/html:body[1]/html:{local_name}[1]"),
        element_name: crate::interop::v2::ElementName {
            namespace: ElementNamespace::Html,
            local_name,
        },
        normalized_text_digest_sha256: normalized_dom_text_digest("One"),
        semantic_attributes: Vec::new(),
    };
    assert_eq!(
        resolve_target_and_propose(&document, &hint, &TargetResolutionOptions::default())
            .expect_err("overlong public target name")
            .error_code,
        ExplorationErrorCode::TargetNotFound
    );
}

#[test]
fn target_resolution_handles_path_omission_and_each_shared_work_exhaustion_boundary() {
    let deep = format!(
        "{}<target>target</target>{}",
        "<div>".repeat(1_500),
        "</div>".repeat(1_500)
    );
    let document = prepare_document(
        HtmlInput::new("overlong-path", deep).expect("source"),
        PreparationLimits::new(
            NonZeroU32::new(50 * 1024 * 1024).expect("non-zero"),
            NonZeroU32::new(4_096).expect("non-zero"),
            NonZeroU32::new(4_096).expect("non-zero"),
        )
        .expect("limits"),
    )
    .expect("prepared");
    let path = format!(
        "html:html[1]/html:body[1]/{}html:target[1]",
        "html:div[1]/".repeat(1_500)
    );
    let omitted = ElementTargetHint {
        path_profile: "htmlcut.dom_path.v1".to_owned(),
        path,
        element_name: crate::interop::v2::ElementName {
            namespace: ElementNamespace::Html,
            local_name: "target".to_owned(),
        },
        normalized_text_digest_sha256: normalized_dom_text_digest("target"),
        semantic_attributes: Vec::new(),
    };
    assert_eq!(
        resolve_target_and_propose(&document, &omitted, &TargetResolutionOptions::default())
            .expect_err("overlong internal path cannot retarget")
            .error_code,
        ExplorationErrorCode::TargetNotFound
    );

    let document = prepare_document(
        HtmlInput::new(
            "work-boundaries",
            "<main id=\"content\"><span>One</span></main>",
        )
        .expect("source"),
        PreparationLimits::default(),
    )
    .expect("prepared");
    let hint = ElementTargetHint {
        path_profile: "htmlcut.dom_path.v1".to_owned(),
        path: "html:html[1]/html:body[1]/html:main[1]".to_owned(),
        element_name: crate::interop::v2::ElementName {
            namespace: ElementNamespace::Html,
            local_name: "main".to_owned(),
        },
        normalized_text_digest_sha256: normalized_dom_text_digest("One"),
        semantic_attributes: Vec::new(),
    };
    for limit in 1..=64 {
        let result = resolve_target_and_propose(
            &document,
            &hint,
            &TargetResolutionOptions {
                max_work_units: NonZeroU32::new(limit).expect("non-zero"),
                max_proposals: NonZeroU32::new(1).expect("non-zero"),
            },
        );
        if let Err(error) = result {
            assert_eq!(
                error.error_code,
                ExplorationErrorCode::TargetResolutionLimitExceeded
            );
        }
    }
}
