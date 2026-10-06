// SPDX-License-Identifier: MPL-2.0
//! Bounded, source-independent query compilation and lazy query identity.

use crate::limits::{MAX_PATTERN_DEPTH, MAX_REGEX_BYTES};
use crate::{ErrorCode, ExtractionError, ExtractionPlan};
use regex::{Regex, RegexBuilder};
use scraper::Selector;
use std::{cell::OnceCell, sync::Arc};

pub(crate) struct CompiledGuard {
    pub(crate) selector: Selector,
    pub(crate) predicate: Option<Regex>,
}
pub(crate) struct CompiledField {
    pub(crate) name: String,
    pub(crate) selector: Selector,
    pub(crate) exclusions: Vec<Selector>,
    pub(crate) field: crate::RecordField,
}
/// Reusable validated query grammar, with fresh counters for every execution.
pub struct CompiledPlan {
    pub(crate) plan: ExtractionPlan,
    pub(crate) selector: Selector,
    pub(crate) guards: Vec<CompiledGuard>,
    pub(crate) exclusions: Vec<Selector>,
    pub(crate) fields: Vec<CompiledField>,
    pub(crate) normalized_bytes: Arc<str>,
    digest: OnceCell<String>,
}
impl CompiledPlan {
    /// Validates and normalizes all construction routes, retaining one bounded query encoding.
    pub fn compile(plan: &ExtractionPlan) -> Result<Self, ExtractionError> {
        let plan = plan.normalized()?;
        let normalized_bytes = crate::identity::query_bytes(&plan)?;
        let regex_count = plan
            .expect
            .iter()
            .filter(|guard| guard.pattern.is_some())
            .count();
        let regex_budget = MAX_REGEX_BYTES / (2 * regex_count.max(1));
        let selector = compile_selector(&plan.select)?;
        let guards = plan
            .expect
            .iter()
            .enumerate()
            .map(|(index, guard)| {
                let selector = compile_selector(&guard.select).map_err(|mut error| {
                    error.plan_path = Some(format!("expect[{index}].select"));
                    error
                })?;
                let predicate = guard
                    .pattern
                    .as_deref()
                    .map(|pattern| compile_regex(pattern, regex_budget))
                    .transpose()
                    .map_err(|mut error| {
                        error.plan_path = Some(format!("expect[{index}].pattern"));
                        error
                    })?;
                Ok(CompiledGuard {
                    selector,
                    predicate,
                })
            })
            .collect::<Result<_, ExtractionError>>()?;
        let exclusions = plan
            .exclude
            .iter()
            .flatten()
            .map(|value| compile_selector(value))
            .collect::<Result<_, _>>()?;
        let fields = plan
            .fields
            .iter()
            .flat_map(|fields| fields.iter())
            .map(|(name, field)| {
                let selector = compile_selector(&field.select).map_err(|mut error| {
                    error.field_name = Some(name.clone());
                    error.plan_path = Some(format!("fields.{name}.select"));
                    error
                })?;
                let exclusions = field
                    .exclude
                    .iter()
                    .enumerate()
                    .map(|(index, value)| {
                        compile_selector(value).map_err(|mut error| {
                            error.field_name = Some(name.clone());
                            error.plan_path = Some(format!("fields.{name}.exclude[{index}]"));
                            error
                        })
                    })
                    .collect::<Result<_, _>>()?;
                Ok(CompiledField {
                    name: name.clone(),
                    selector,
                    field: field.clone(),
                    exclusions,
                })
            })
            .collect::<Result<_, ExtractionError>>()?;
        Ok(Self {
            plan,
            selector,
            guards,
            exclusions,
            fields,
            normalized_bytes: Arc::from(normalized_bytes),
            digest: OnceCell::new(),
        })
    }
    /// Effective defaults and preserved explicit assumptions used by this compiled query.
    pub fn plan(&self) -> &ExtractionPlan {
        &self.plan
    }
    /// Canonical normalized JSON, independent of downstream serde_json feature choices.
    pub fn normalized_json(&self) -> &str {
        &self.normalized_bytes
    }
    /// Lazily computes the domain-separated identity of the stored normalized query.
    pub fn plan_sha256(&self) -> &str {
        self.digest.get_or_init(|| {
            crate::identity::framed("htmlcut.plan/6", &[self.normalized_bytes.as_bytes()])
        })
    }
}

pub(crate) fn compile_selector(value: &str) -> Result<Selector, ExtractionError> {
    // Bound grammar recursion before invoking the maintained CSS parser. Quoted and escaped
    // delimiters do not increase nesting; the parser remains authoritative for syntax.
    let mut depth = 0_u32;
    let mut quote = None;
    let mut escaped = false;
    let mut comment = false;
    let mut characters = value.chars().peekable();
    while let Some(c) = characters.next() {
        if comment {
            if c == '*' && characters.peek() == Some(&'/') {
                characters.next();
                comment = false;
            }
            continue;
        }
        if escaped {
            escaped = false;
            continue;
        }
        if c == '\\' {
            escaped = true;
            continue;
        }
        if let Some(q) = quote {
            if c == q {
                quote = None;
            }
            continue;
        }
        if c == '/' && characters.peek() == Some(&'*') {
            characters.next();
            comment = true;
            continue;
        }
        if c == '\'' || c == '"' {
            quote = Some(c);
            continue;
        }
        if c == '(' || c == '[' {
            depth += 1;
            if depth > MAX_PATTERN_DEPTH {
                return Err(ExtractionError::resource(
                    "compilation",
                    "syntax_depth",
                    MAX_PATTERN_DEPTH.into(),
                ));
            }
        } else if c == ')' || c == ']' {
            depth = depth.saturating_sub(1);
        }
    }
    Selector::parse(value).map_err(|_| {
        ExtractionError::new(
            ErrorCode::InvalidSelector,
            "compilation",
            "The CSS selector is invalid or unsupported.",
        )
    })
}

fn compile_regex(pattern: &str, regex_budget: usize) -> Result<Regex, ExtractionError> {
    RegexBuilder::new(pattern)
        .size_limit(regex_budget)
        .dfa_size_limit(regex_budget)
        .nest_limit(MAX_PATTERN_DEPTH)
        .build()
        .map_err(|error| {
            if matches!(error, regex::Error::CompiledTooBig(_)) {
                return ExtractionError::resource(
                    "compilation",
                    "regex_program_bytes",
                    regex_budget as u64,
                );
            }
            ExtractionError::new(
                ErrorCode::InvalidRegex,
                "compilation",
                "The regular expression is invalid or exceeds compilation limits.",
            )
        })
}
