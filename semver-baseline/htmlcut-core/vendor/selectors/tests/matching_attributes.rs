//! Namespace defaulting for explicit attribute selector representations.
use super::*;
use crate::attr::{AttrSelectorOperator, AttrSelectorWithOptionalNamespace};

#[test]
fn omitted_attribute_namespace_means_no_namespace_and_keeps_case_policy() {
    let tree = Tree::new(vec![
        Record {
            name: "p",
            attrs: &[("", "code", "AB")],
            ..Record::default()
        },
        Record {
            name: "p",
            namespace: "urn:fixture",
            attrs: &[("urn:fixture", "CODE", "AB"), ("", "CODE", "CD")],
            ..Record::default()
        },
    ]);
    let exists = AttrSelectorWithOptionalNamespace::<DummySelectorImpl> {
        namespace: None,
        local_name: "CODE".into(),
        local_name_lower: "code".into(),
        operation: ParsedAttrSelectorOperation::Exists,
    };
    assert!(matches_rare_attribute_selector(&tree.element(0), &exists));
    assert!(matches_rare_attribute_selector(&tree.element(1), &exists));
    let value = AttrSelectorWithOptionalNamespace::<DummySelectorImpl> {
        namespace: None,
        local_name: "CODE".into(),
        local_name_lower: "code".into(),
        operation: ParsedAttrSelectorOperation::WithValue {
            operator: AttrSelectorOperator::Equal,
            case_sensitivity: ParsedCaseSensitivity::ExplicitCaseSensitive,
            value: "AB".into(),
        },
    };
    assert!(matches_rare_attribute_selector(&tree.element(0), &value));
    assert!(
        !matches_rare_attribute_selector(&tree.element(1), &value),
        "a namespaced AB value cannot satisfy the unqualified attribute"
    );
}
