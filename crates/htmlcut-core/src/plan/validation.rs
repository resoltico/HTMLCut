// SPDX-License-Identifier: MPL-2.0
use super::*;
use crate::limits::{MAX_CHECKS, MAX_PATTERN_BYTES};

fn invalid() -> ExtractionError {
    ExtractionError::new(
        ErrorCode::InvalidPlan,
        "validation",
        "The plan contains incompatible or invalid options.",
    )
}

pub(crate) fn pattern(value: &str) -> Result<(), ExtractionError> {
    if value.is_empty() {
        return Err(invalid());
    }
    if value.len() > MAX_PATTERN_BYTES {
        return Err(ExtractionError::limit("plan"));
    }
    Ok(())
}

pub(crate) fn attribute(value: &str) -> Result<(), ExtractionError> {
    if value.is_empty()
        || value.len() > 256
        || value
            .chars()
            .any(|c| c.is_whitespace() || "\"'<>/=\0".contains(c))
    {
        return Err(invalid());
    }
    Ok(())
}

impl ExtractionPlan {
    /// Checks shared constructor/wire invariants and all incompatible option combinations.
    pub fn validate(&self) -> Result<(), ExtractionError> {
        if self.schema != "htmlcut.extraction.plan" || self.version != SCHEMA_VERSION {
            return Err(ExtractionError::new(
                ErrorCode::InvalidSchema,
                "validation",
                "Unsupported extraction plan schema or version.",
            ));
        }
        self.limits.validate()?;
        if self.exclude.len() > MAX_CHECKS || self.guards.len() > MAX_CHECKS {
            return Err(invalid());
        }
        match &self.projection {
            Projection::Value(value) => validate_transforms(value, &self.transforms)?,
            _ if !self.transforms.is_empty() => return Err(invalid()),
            _ => (),
        }
        // Debit remaining materialized-string capacity before serialization can copy
        // caller-owned strings. checked_sub handles both oversize and arithmetic safety.
        let mut remaining = crate::limits::MAX_PLAN_BYTES
            .checked_sub(self.schema.len())
            .expect("the validated fixed schema name fits the plan byte allowance");
        let mut add = |value: &str| -> Result<(), ExtractionError> {
            remaining = remaining
                .checked_sub(value.len())
                .ok_or_else(|| ExtractionError::limit("plan"))?;
            Ok(())
        };
        match &self.strategy {
            Strategy::Css { selector } => add(selector)?,
            Strategy::Slice { start, end, .. } => {
                for boundary in [start, end] {
                    match boundary {
                        Boundary::Literal { value } => add(value)?,
                        Boundary::Regex { pattern, flags } => {
                            add(pattern)?;
                            add(flags)?;
                        }
                    }
                }
            }
        }
        if let Projection::Value(ValueProjection::Attribute { name }) = &self.projection {
            add(name)?;
        }
        if let Projection::Records {
            fields,
            following_siblings,
        } = &self.projection
        {
            if *following_siblings > crate::limits::MAX_FOLLOWING_SIBLINGS
                || fields.is_empty()
                || fields.len() > crate::limits::MAX_RECORD_FIELDS
                || !self.exclude.is_empty()
            {
                return Err(invalid());
            }
            let mut names = std::collections::HashSet::new();
            let mut excluded_count = self.exclude.len();
            for field in fields {
                if !field_name(&field.name) || !names.insert(&field.name) {
                    return Err(invalid());
                }
                add(&field.name)?;
                add(&field.selector)?;
                pattern(&field.selector)?;
                if let ValueProjection::Attribute { name } = &field.projection {
                    add(name)?;
                }
                for exclusion in &field.exclude {
                    add(exclusion)?;
                }
                if field.exclude.len() > MAX_CHECKS - excluded_count {
                    return Err(invalid());
                }
                excluded_count += field.exclude.len();
                validate_value(&field.projection, &field.exclude)?;
                validate_transforms(&field.projection, &field.transforms)?;
                match &field.selection {
                    FieldSelection::Single {} | FieldSelection::Optional {} => (),
                    FieldSelection::All { min, max }
                        if *min <= max.unwrap_or(self.limits.max_selected)
                            && max.unwrap_or(self.limits.max_selected)
                                <= self.limits.max_selected => {}
                    FieldSelection::Nth { index }
                        if *index > 0 && *index <= self.limits.max_candidates => {}
                    _ => return Err(invalid()),
                }
            }
        }
        for value in &self.exclude {
            add(value)?;
        }
        for guard in &self.guards {
            add(&guard.selector)?;
            if let GuardRead::Attribute { name } = &guard.read {
                add(name)?;
            }
            match &guard.predicate {
                Some(Predicate::Exact { value }) => add(value)?,
                Some(Predicate::Regex { pattern, flags }) => {
                    add(pattern)?;
                    add(flags)?;
                }
                None => (),
            }
        }
        crate::identity::canonical_json_bounded(self, crate::limits::MAX_PLAN_BYTES)?;
        match &self.strategy {
            Strategy::Css { selector } => {
                pattern(selector)?;
                self.projection.dom()?;
            }
            Strategy::Slice { start, end, .. } => {
                for boundary in [start, end] {
                    match boundary {
                        Boundary::Literal { value } => pattern(value)?,
                        Boundary::Regex {
                            pattern: value,
                            flags,
                        } => {
                            pattern(value)?;
                            validate_flags(flags)?;
                        }
                    }
                }
                if !matches!(self.projection, Projection::Source {})
                    || !self.exclude.is_empty()
                    || !self.guards.is_empty()
                {
                    return Err(invalid());
                }
            }
        }
        match &self.selection {
            Selection::Single {} => (),
            Selection::Nth { index } if *index > 0 && *index <= self.limits.max_candidates => (),
            Selection::All { min, max }
                if *min <= max.unwrap_or(self.limits.max_selected)
                    && max.unwrap_or(self.limits.max_selected) <= self.limits.max_selected => {}
            _ => return Err(invalid()),
        }
        if let Projection::Value(value) = &self.projection {
            validate_value(value, &self.exclude)?;
        }
        for exclusion in &self.exclude {
            pattern(exclusion)?;
        }
        for guard in &self.guards {
            pattern(&guard.selector)?;
            let max = guard.max.unwrap_or(self.limits.max_candidates);
            if guard.min > max || max > self.limits.max_candidates {
                return Err(invalid());
            }
            match &guard.read {
                GuardRead::Attribute { name } => attribute(name)?,
                GuardRead::DomText {} if guard.predicate.is_none() => return Err(invalid()),
                GuardRead::DomText {} => (),
            }
            if let Some(Predicate::Regex {
                pattern: value,
                flags,
            }) = &guard.predicate
            {
                pattern(value)?;
                validate_flags(flags)?;
            }
        }
        Ok(())
    }
}

fn field_name(name: &str) -> bool {
    let mut chars = name.bytes();
    name.len() <= 64
        && chars
            .next()
            .is_some_and(|c| c.is_ascii_alphabetic() || c == b'_')
        && chars.all(|c| c.is_ascii_alphanumeric() || c == b'_')
}

fn validate_value(value: &ValueProjection, excluded: &[String]) -> Result<(), ExtractionError> {
    if let ValueProjection::Attribute { name } = value {
        attribute(name)?;
        if !excluded.is_empty() {
            return Err(invalid());
        }
    }
    for selector in excluded {
        pattern(selector)?;
    }
    Ok(())
}

fn validate_transforms(
    value: &ValueProjection,
    transforms: &[Transform],
) -> Result<(), ExtractionError> {
    match transforms {
        [] => Ok(()),
        [Transform::NormalizeWhitespace {}] if matches!(value, ValueProjection::DomText {}) => {
            Ok(())
        }
        [Transform::ResolveUrls {}] => match value {
            ValueProjection::Markdown {} => Ok(()),
            ValueProjection::Attribute { name }
                if matches!(
                    name.as_str(),
                    "href" | "src" | "action" | "poster" | "cite" | "formaction" | "data"
                ) =>
            {
                Ok(())
            }
            _ => Err(invalid()),
        },
        _ => Err(invalid()),
    }
}

pub(crate) fn validate_flags(flags: &str) -> Result<(), ExtractionError> {
    let mut seen = String::new();
    for flag in flags.chars() {
        if !"imsUx".contains(flag) || seen.contains(flag) {
            return Err(invalid());
        }
        seen.push(flag);
    }
    Ok(())
}
