use htmlcut_core::{FetchPreflightMode, PatternMode, ValueType, WhitespaceMode};

#[cfg(test)]
use super::OperationCliContract;
use super::{
    CliCondition, CliConditionalDefault, CliConstraint, CliInputForm, CliOutputMode,
    CliParameterDescriptor, CliParameterId, CliParameterKind, CliParameterRequirement,
    CliParameterSection, CliSelectionMode, CliTlsTrustMode, CliValue,
};

mod commands;
mod common;
mod descriptors;

pub(super) fn common_input_forms() -> Vec<CliInputForm> {
    common::common_input_forms()
}

pub(super) fn common_selection_modes() -> Vec<CliSelectionMode> {
    common::common_selection_modes()
}

pub(super) fn inspect_output_modes() -> Vec<CliOutputMode> {
    common::inspect_output_modes()
}

pub(super) fn extract_output_modes() -> Vec<CliOutputMode> {
    common::extract_output_modes()
}

pub(super) fn select_extract_value_modes() -> Vec<ValueType> {
    common::select_extract_value_modes()
}

pub(super) fn slice_extract_value_modes() -> Vec<ValueType> {
    common::slice_extract_value_modes()
}

pub(super) fn inspect_source_parameters() -> Vec<CliParameterDescriptor> {
    commands::inspect_source_parameters()
}

pub(super) fn inspect_select_parameters() -> Vec<CliParameterDescriptor> {
    commands::inspect_select_parameters()
}

pub(super) fn inspect_slice_parameters() -> Vec<CliParameterDescriptor> {
    commands::inspect_slice_parameters()
}

pub(super) fn select_extract_parameters() -> Vec<CliParameterDescriptor> {
    commands::select_extract_parameters()
}

pub(super) fn slice_extract_parameters() -> Vec<CliParameterDescriptor> {
    commands::slice_extract_parameters()
}

pub(super) fn selection_mode_values(modes: &[CliSelectionMode]) -> Vec<CliValue> {
    modes.iter().copied().map(CliValue::SelectionMode).collect()
}

pub(super) fn output_mode_values(modes: &[CliOutputMode]) -> Vec<CliValue> {
    modes.iter().copied().map(CliValue::OutputMode).collect()
}

pub(super) fn value_type_values(modes: &[ValueType]) -> Vec<CliValue> {
    modes.iter().copied().map(CliValue::ValueType).collect()
}

fn whitespace_values() -> Vec<CliValue> {
    vec![
        CliValue::WhitespaceMode(WhitespaceMode::Rendered),
        CliValue::WhitespaceMode(WhitespaceMode::Normalize),
    ]
}

fn fetch_preflight_values() -> Vec<CliValue> {
    vec![
        CliValue::FetchPreflightMode(FetchPreflightMode::HeadFirst),
        CliValue::FetchPreflightMode(FetchPreflightMode::GetOnly),
    ]
}

fn tls_trust_values() -> Vec<CliValue> {
    vec![
        CliValue::TlsTrustMode(CliTlsTrustMode::WebPki),
        CliValue::TlsTrustMode(CliTlsTrustMode::Platform),
        CliValue::TlsTrustMode(CliTlsTrustMode::CustomCaBundle),
    ]
}

fn pattern_values() -> Vec<CliValue> {
    vec![
        CliValue::PatternMode(PatternMode::Literal),
        CliValue::PatternMode(PatternMode::Regex),
    ]
}

fn boundary_retention_values() -> Vec<CliValue> {
    vec![
        CliValue::BoundaryRetentionMode(super::CliBoundaryRetentionMode::ExcludeBoth),
        CliValue::BoundaryRetentionMode(super::CliBoundaryRetentionMode::IncludeStart),
        CliValue::BoundaryRetentionMode(super::CliBoundaryRetentionMode::IncludeEnd),
        CliValue::BoundaryRetentionMode(super::CliBoundaryRetentionMode::IncludeBoth),
    ]
}

pub(super) fn condition(parameter: CliParameterId, values: Vec<CliValue>) -> CliCondition {
    CliCondition { parameter, values }
}

pub(super) fn conditional_default(value: CliValue, when: CliCondition) -> CliConditionalDefault {
    CliConditionalDefault { value, when }
}

pub(super) fn restricts_parameter_values(
    parameter: CliParameterId,
    allowed_values: Vec<CliValue>,
    when: CliCondition,
) -> CliConstraint {
    CliConstraint::RestrictsParameterValues {
        parameter,
        allowed_values,
        when,
    }
}

pub(super) fn requires_parameter(parameter: CliParameterId, when: CliCondition) -> CliConstraint {
    CliConstraint::RequiresParameter { parameter, when }
}

pub(super) fn constraints_with_parameter_rules(
    parameters: &[CliParameterDescriptor],
    mut extra_constraints: Vec<CliConstraint>,
) -> Vec<CliConstraint> {
    let mut constraints = Vec::new();

    for parameter in parameters {
        match &parameter.requirement {
            CliParameterRequirement::RequiredWhen(when) => {
                constraints.push(CliConstraint::RequiresParameter {
                    parameter: parameter.id,
                    when: when.clone(),
                });
            }
            CliParameterRequirement::AllowedOnlyWhen(when) => {
                constraints.push(CliConstraint::AllowedOnlyWhen {
                    parameter: parameter.id,
                    when: when.clone(),
                });
            }
            CliParameterRequirement::Required | CliParameterRequirement::Optional => {}
            CliParameterRequirement::RequiredUnless(_) => {}
        }
    }

    constraints.append(&mut extra_constraints);
    constraints
}

#[cfg(test)]
pub(super) fn parameter_descriptor(
    contract: &OperationCliContract,
    parameter_id: CliParameterId,
) -> Option<&CliParameterDescriptor> {
    contract
        .parameters
        .iter()
        .find(|parameter| parameter.id == parameter_id)
}
