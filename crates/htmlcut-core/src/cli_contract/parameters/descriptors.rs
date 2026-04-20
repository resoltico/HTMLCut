use super::{
    CliParameterDescriptor, CliParameterId, CliParameterKind, CliParameterRequirement,
    CliParameterSection, CliValue,
};

pub(super) fn param_positional(
    section: CliParameterSection,
    id: CliParameterId,
    requirement: CliParameterRequirement,
    summary: &'static str,
) -> CliParameterDescriptor {
    CliParameterDescriptor {
        section,
        id,
        kind: CliParameterKind::Positional,
        requirement,
        value_hint: None,
        default: None,
        allowed_values: Vec::new(),
        summary,
    }
}

pub(super) fn param_option(
    section: CliParameterSection,
    id: CliParameterId,
    requirement: CliParameterRequirement,
    value_hint: &'static str,
    default: Option<CliValue>,
    allowed_values: Vec<CliValue>,
    summary: &'static str,
) -> CliParameterDescriptor {
    CliParameterDescriptor {
        section,
        id,
        kind: CliParameterKind::Option,
        requirement,
        value_hint: Some(value_hint),
        default,
        allowed_values,
        summary,
    }
}

pub(super) fn param_flag(
    section: CliParameterSection,
    id: CliParameterId,
    summary: &'static str,
) -> CliParameterDescriptor {
    CliParameterDescriptor {
        section,
        id,
        kind: CliParameterKind::Flag,
        requirement: CliParameterRequirement::Optional,
        value_hint: None,
        default: Some(CliValue::Boolean(false)),
        allowed_values: Vec::new(),
        summary,
    }
}
