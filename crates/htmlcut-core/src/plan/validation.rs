// SPDX-License-Identifier: MPL-2.0
//! Query invariants shared by JSON, CLI and Rust constructors.
use super::*;
use crate::limits::{MAX_CHECKS, MAX_PATTERN_BYTES};

pub(super) fn invalid() -> ExtractionError {
    ExtractionError::new(
        ErrorCode::InvalidPlan,
        "validation",
        "The query contains incompatible or invalid members.",
    )
}
pub(crate) fn pattern(value: &str) -> Result<(), ExtractionError> {
    if value.is_empty() {
        return Err(invalid());
    }
    if value.len() > MAX_PATTERN_BYTES {
        return Err(ExtractionError::resource(
            "plan",
            "pattern_bytes",
            MAX_PATTERN_BYTES as u64,
        ));
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
pub(super) fn field_name(name: &str) -> bool {
    let mut chars = name.bytes();
    name.len() <= 64
        && chars
            .next()
            .is_some_and(|c| c.is_ascii_alphabetic() || c == b'_')
        && chars.all(|c| c.is_ascii_alphanumeric() || c == b'_')
}
pub(super) fn reading(read: &Reading, exclude: &[String]) -> Result<(), ExtractionError> {
    if let Reading::Attribute(name) | Reading::Url(name) = read {
        attribute(name)?;
        if !exclude.is_empty() {
            return Err(invalid());
        }
        if matches!(read, Reading::Url(_))
            && !matches!(
                name.as_str(),
                "href" | "src" | "action" | "poster" | "cite" | "formaction" | "data"
            )
        {
            return Err(invalid());
        }
    }
    for selector in exclude {
        pattern(selector)?;
    }
    Ok(())
}
fn cardinality(
    mode: Match,
    min: Option<u32>,
    max: Option<u32>,
    index: Option<u32>,
    candidates: u32,
) -> Result<(), ExtractionError> {
    match mode {
        Match::One if min.is_none() && max.is_none() && index.is_none() => Ok(()),
        Match::All
            if index.is_none()
                && min.unwrap_or(1) <= max.unwrap_or(crate::limits::MAX_CANDIDATES)
                && max.unwrap_or(crate::limits::MAX_CANDIDATES)
                    <= crate::limits::MAX_CANDIDATES =>
        {
            Ok(())
        }
        Match::Nth
            if min.is_none()
                && max.is_none()
                && index.is_some_and(|i| i > 0 && i <= candidates) =>
        {
            Ok(())
        }
        _ => Err(invalid()),
    }
}
impl ExtractionPlan {
    /// Validates member applicability and retained finite syntax/resource bounds.
    pub fn validate(&self) -> Result<(), ExtractionError> {
        if self.version != SCHEMA_VERSION {
            return Err(ExtractionError::new(
                ErrorCode::InvalidSchema,
                "validation",
                "Unsupported query version.",
            ));
        }
        self.limits.validate()?;
        // Reject excessive caller-owned strings before normalization clones them.
        // Complete escaped/default-expanded admission belongs to the compiler's one encoder.
        let mut remaining = crate::MAX_PLAN_BYTES;
        let mut add = |value: &str| {
            remaining = remaining.checked_sub(value.len()).ok_or_else(|| {
                ExtractionError::resource("plan", "query_bytes", crate::MAX_PLAN_BYTES as u64)
            })?;
            Ok::<_, ExtractionError>(())
        };
        add(&self.select)?;
        if let Some(Reading::Attribute(name) | Reading::Url(name)) = &self.read {
            add(name)?;
        }
        for exclusion in self.exclude.iter().flatten() {
            add(exclusion)?;
        }
        for (name, field) in self.fields.iter().flat_map(|fields| fields.iter()) {
            add(name)?;
            add(&field.select)?;
            if let Reading::Attribute(name) | Reading::Url(name) = &field.read {
                add(name)?;
            }
            for exclusion in &field.exclude {
                add(exclusion)?;
            }
        }
        for guard in &self.expect {
            add(&guard.select)?;
            if let Some(Reading::Attribute(name)) = &guard.read {
                add(name)?;
            }
            if let Some(value) = &guard.equals {
                add(value)?;
            }
            if let Some(pattern) = &guard.pattern {
                add(pattern)?;
            }
        }

        pattern(&self.select)?;
        cardinality(
            self.match_mode,
            self.min,
            self.max,
            self.index,
            self.limits.max_candidates,
        )?;
        if self.expect.len() > MAX_CHECKS {
            return Err(invalid());
        }
        let mut exclusions = 0;
        if let Some(fields) = &self.fields {
            if fields.is_empty()
                || fields.len() > crate::MAX_RECORD_FIELDS
                || self.read.is_some()
                || self.exclude.is_some()
                || self.following_siblings.unwrap_or(0) > crate::limits::MAX_FOLLOWING_SIBLINGS
            {
                return Err(invalid());
            }
            for (name, field) in fields {
                RecordField::validate_name(name)?;
                let validate = || {
                    pattern(&field.select)?;
                    let mode = match field.match_mode {
                        FieldMatch::One | FieldMatch::Optional => Match::One,
                        FieldMatch::All => Match::All,
                        FieldMatch::Nth => Match::Nth,
                    };
                    cardinality(
                        mode,
                        field.min,
                        field.max,
                        field.index,
                        self.limits.max_candidates,
                    )?;
                    reading(&field.read, &field.exclude)
                };
                validate().map_err(|mut error| {
                    error.field_name = Some(name.clone());
                    error
                })?;
                if field.exclude.len() > MAX_CHECKS - exclusions {
                    return Err(invalid());
                }
                exclusions += field.exclude.len();
            }
        } else {
            if self.following_siblings.is_some() {
                return Err(invalid());
            }
            let exclude = self.exclude.as_deref().unwrap_or_default();
            if exclude.len() > MAX_CHECKS {
                return Err(invalid());
            }
            reading(self.read.as_ref().unwrap_or(&Reading::Text), exclude)?;
        }
        for guard in &self.expect {
            pattern(&guard.select)?;
            if guard.min > guard.max
                || guard.max > self.limits.max_candidates
                || (guard.equals.is_some() && guard.pattern.is_some())
            {
                return Err(invalid());
            }
            if guard.equals.is_none() && guard.pattern.is_none() {
                if guard.read.is_some() {
                    return Err(invalid());
                }
            } else {
                let read = guard.read.as_ref().unwrap_or(&Reading::Text);
                if !matches!(
                    read,
                    Reading::Text | Reading::Literal | Reading::Attribute(_)
                ) {
                    return Err(invalid());
                }
                reading(read, &[])?;
            }
            if let Some(regex) = &guard.pattern {
                pattern(regex)?;
            }
        }
        Ok(())
    }
}
