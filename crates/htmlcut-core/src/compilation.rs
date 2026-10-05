// SPDX-License-Identifier: MPL-2.0
//! Bounded source-independent grammar compilation.

use regex::{Regex, RegexBuilder};
use scraper::Selector;

use crate::limits::{MAX_PATTERN_DEPTH, MAX_REGEX_BYTES};
use crate::{
    Boundary, ErrorCode, ExtractionError, ExtractionPlan, Predicate, Projection, Strategy,
};

pub(crate) enum Matcher {
    Literal(String),
    Regex(Regex),
}

impl Matcher {
    pub(crate) fn find(&self, source: &str, offset: usize) -> Option<(usize, usize)> {
        match self {
            Self::Literal(value) => source[offset..]
                .find(value)
                .map(|start| (offset + start, offset + start + value.len())),
            Self::Regex(value) => value
                .find_at(source, offset)
                .map(|found| (found.start(), found.end())),
        }
    }
}

pub(crate) enum CompiledStrategy {
    Css {
        selector: Selector,
        projection: CompiledDomProjection,
    },
    Slice {
        start: Matcher,
        end: Matcher,
        include_start: bool,
        include_end: bool,
    },
}

pub(crate) enum CompiledDomProjection {
    Value(crate::ValueProjection),
    Records(u32),
}

pub(crate) struct CompiledGuard {
    pub(crate) selector: Selector,
    pub(crate) predicate: Option<Regex>,
}

pub(crate) struct CompiledField {
    pub(crate) selector: Selector,
    pub(crate) exclusions: Vec<Selector>,
    pub(crate) field: crate::RecordField,
}

/// Opaque reusable validated grammar; execution counters are never retained here.
pub struct CompiledPlan {
    pub(crate) plan: ExtractionPlan,
    pub(crate) strategy: CompiledStrategy,
    pub(crate) guards: Vec<CompiledGuard>,
    pub(crate) exclusions: Vec<Selector>,
    pub(crate) fields: Vec<CompiledField>,
    pub(crate) digest: String,
}

impl CompiledPlan {
    /// Compiles selector and regex grammar once, under finite plan/grammar limits.
    pub fn compile(plan: &ExtractionPlan) -> Result<Self, ExtractionError> {
        let plan = plan.normalized()?;
        let boundary_regexes = match &plan.strategy {
            Strategy::Slice { start, end, .. } => [start, end]
                .iter()
                .filter(|boundary| matches!(boundary, Boundary::Regex { .. }))
                .count(),
            _ => 0,
        };
        let regex_count = boundary_regexes
            + plan
                .guards
                .iter()
                .filter(|guard| matches!(guard.predicate, Some(Predicate::Regex { .. })))
                .count();
        // Program and DFA allowances together share one fixed plan allowance.
        let regex_budget = MAX_REGEX_BYTES / (2 * regex_count.max(1));
        let strategy = match &plan.strategy {
            Strategy::Css { selector } => CompiledStrategy::Css {
                selector: compile_selector(selector)?,
                projection: match plan.projection.dom()? {
                    crate::plan::DomProjection::Value(value) => {
                        CompiledDomProjection::Value(value.clone())
                    }
                    crate::plan::DomProjection::Records(following) => {
                        CompiledDomProjection::Records(following)
                    }
                },
            },
            Strategy::Slice {
                start,
                end,
                include_start,
                include_end,
            } => CompiledStrategy::Slice {
                start: compile_boundary(start, regex_budget)?,
                end: compile_boundary(end, regex_budget)?,
                include_start: *include_start,
                include_end: *include_end,
            },
        };
        let guards = plan
            .guards
            .iter()
            .map(|guard| {
                Ok(CompiledGuard {
                    selector: compile_selector(&guard.selector)?,
                    predicate: match &guard.predicate {
                        Some(Predicate::Regex { pattern, flags }) => {
                            Some(compile_regex(pattern, flags, regex_budget)?)
                        }
                        _ => None,
                    },
                })
            })
            .collect::<Result<_, ExtractionError>>()?;
        let exclusions = plan
            .exclude
            .iter()
            .map(|value| compile_selector(value))
            .collect::<Result<_, _>>()?;
        let digest = crate::identity::framed(
            "htmlcut.plan/5",
            &[crate::canonical_json(&plan)?.as_bytes()],
        );
        let fields = match &plan.projection {
            Projection::Records { fields, .. } => fields
                .iter()
                .map(|field| {
                    Ok(CompiledField {
                        selector: compile_selector(&field.selector)?,
                        field: field.clone(),
                        exclusions: field
                            .exclude
                            .iter()
                            .map(|s| compile_selector(s))
                            .collect::<Result<_, _>>()?,
                    })
                })
                .collect::<Result<Vec<_>, ExtractionError>>()?,
            _ => Vec::new(),
        };
        Ok(Self {
            plan,
            strategy,
            guards,
            exclusions,
            fields,
            digest,
        })
    }

    /// Fully materialized defaults used for execution and plan hashing.
    pub fn plan(&self) -> &ExtractionPlan {
        &self.plan
    }
    /// Domain-separated SHA-256 identity of the normalized plan.
    pub fn plan_sha256(&self) -> &str {
        &self.digest
    }
}

fn compile_boundary(boundary: &Boundary, regex_budget: usize) -> Result<Matcher, ExtractionError> {
    match boundary {
        Boundary::Literal { value } => Ok(Matcher::Literal(value.clone())),
        Boundary::Regex { pattern, flags } => {
            Ok(Matcher::Regex(compile_regex(pattern, flags, regex_budget)?))
        }
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
                return Err(ExtractionError::limit("compilation"));
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

fn compile_regex(
    pattern: &str,
    flags: &str,
    regex_budget: usize,
) -> Result<Regex, ExtractionError> {
    let mut builder = RegexBuilder::new(pattern);
    builder
        .size_limit(regex_budget)
        .dfa_size_limit(regex_budget)
        .nest_limit(MAX_PATTERN_DEPTH);
    builder
        .case_insensitive(flags.contains('i'))
        .multi_line(flags.contains('m'))
        .dot_matches_new_line(flags.contains('s'))
        .swap_greed(flags.contains('U'))
        .ignore_whitespace(flags.contains('x'));
    builder.build().map_err(|error| {
        if matches!(error, regex::Error::CompiledTooBig(_)) {
            return ExtractionError::limit("compilation");
        }
        ExtractionError::new(
            ErrorCode::InvalidRegex,
            "compilation",
            "The regular expression is invalid or exceeds compilation limits.",
        )
    })
}
