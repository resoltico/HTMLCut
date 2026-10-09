// SPDX-License-Identifier: MPL-2.0
//! Effective HTML inline roles and explicitly declared fenced-code language metadata.

use super::*;

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum Style {
    Emphasis,
    Strong,
}

impl Style {
    pub(super) fn open(self) -> &'static str {
        match self {
            Self::Emphasis => "<em>",
            Self::Strong => "<strong>",
        }
    }
    pub(super) fn close(self) -> &'static str {
        match self {
            Self::Emphasis => "</em>",
            Self::Strong => "</strong>",
        }
    }
}

#[derive(Clone, Copy, Default, PartialEq, Eq)]
pub(super) struct Styles(pub(super) [Option<Style>; 2]);

impl Styles {
    pub(super) fn with(mut self, style: Style) -> Self {
        if !self.0.contains(&Some(style)) {
            let position = usize::from(self.0[0].is_some());
            self.0[position] = Some(style);
        }
        self
    }
}

pub(super) struct Ancestry<'a> {
    pub(super) styles: Styles,
    pub(super) pre: Option<ElementRef<'a>>,
    pub(super) code: bool,
}

pub(super) fn ancestry<'a>(
    root: ElementRef<'a>,
    budget: &WorkBudget,
) -> Result<Ancestry<'a>, ExtractionError> {
    let mut pre = None;
    let mut code = false;
    let mut emphasis = false;
    let mut strong = false;
    let mut parent = root.parent();
    while let Some(node) = parent {
        crate::execution::charge(budget, 1)?;
        if let Some(element) = ElementRef::wrap(node)
            && context::html(element.value())
        {
            match element.value().name() {
                "pre" => {
                    emphasis = false;
                    strong = false;
                    if pre.is_none() {
                        pre = Some(element);
                    }
                }
                "code" => {
                    code = true;
                    emphasis = false;
                    strong = false;
                }
                "em" | "i" => emphasis = true,
                "strong" | "b" | "th" => strong = true,
                _ => (),
            }
        }
        parent = node.parent();
    }
    let mut styles = Styles::default();
    if emphasis {
        styles = styles.with(Style::Emphasis);
    }
    if strong {
        styles = styles.with(Style::Strong);
    }
    Ok(Ancestry { styles, pre, code })
}

pub(super) fn language(
    pre: ElementRef<'_>,
    budget: &WorkBudget,
) -> Result<Option<String>, ExtractionError> {
    let mut result = None;
    classes(pre.attr("class"), &mut result, budget)?;
    let mut direct_code = None;
    let mut count = 0;
    for node in pre.children() {
        crate::execution::charge(budget, 1)?;
        if let Some(element) = ElementRef::wrap(node)
            && context::html(element.value())
            && element.value().name() == "code"
        {
            count += 1;
            direct_code = Some(element);
        }
    }
    if count == 1 {
        classes(
            direct_code.expect("one direct code child").attr("class"),
            &mut result,
            budget,
        )?;
    }
    Ok(result)
}

fn classes(
    value: Option<&str>,
    result: &mut Option<String>,
    budget: &WorkBudget,
) -> Result<(), ExtractionError> {
    let Some(value) = value else {
        return Ok(());
    };
    crate::execution::charge(budget, value.len().div_ceil(64) + 1)?;
    for token in value.split_ascii_whitespace() {
        let Some(language) = token.strip_prefix("language-") else {
            continue;
        };
        if language.is_empty()
            || language.len() > 64
            || !language.as_bytes()[0].is_ascii_alphanumeric()
            || !language
                .bytes()
                .all(|c| c.is_ascii_alphanumeric() || b"_+.-".contains(&c))
            || result.as_ref().is_some_and(|prior| prior != language)
        {
            return Err(ExtractionError::new(
                ErrorCode::InvalidRepresentation,
                "projection",
                "Fenced-code language metadata is invalid or ambiguous.",
            ));
        }
        *result = Some(language.into());
    }
    Ok(())
}

#[cfg(test)]
mod language_work_tests {
    use super::*;

    #[test]
    fn explicit_empty_class_still_costs_a_processing_unit() {
        let budget = WorkBudget::new(1);
        let mut result = None;
        classes(Some(""), &mut result, &budget).unwrap();
        assert_eq!(result, None);
        assert_eq!(budget.remaining(), 0);
    }

    #[test]
    fn language_scan_cannot_spend_less_than_its_processing_allowance() {
        for (units, accepted) in [(1, false), (2, true)] {
            let budget = WorkBudget::new(units);
            let mut result = None;
            let outcome = classes(Some("language-rust"), &mut result, &budget);
            assert_eq!(outcome.is_ok(), accepted);
            if accepted {
                assert_eq!(result.as_deref(), Some("rust"));
            } else {
                assert_eq!(outcome.unwrap_err().code, ErrorCode::ResourceLimit);
            }
        }
    }
}
