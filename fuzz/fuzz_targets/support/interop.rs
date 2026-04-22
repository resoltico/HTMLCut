use arbitrary::Arbitrary;
use htmlcut_core::interop::v1::{
    DelimiterMode, HtmlInput, Output, Plan, PlanStrategy, RegexFlag, execute_plan,
};
use htmlcut_core::{SelectorQuery, SliceBoundary};

use crate::interop_common::{FuzzNormalization, FuzzSelection, FuzzValueKind, sample_base_url};

#[derive(Arbitrary, Debug)]
pub struct InteropInput {
    html: String,
    strategy: FuzzInteropStrategy,
    value_kind: FuzzValueKind,
    selection: FuzzSelection,
    normalization: FuzzNormalization,
}

pub fn drive(input: InteropInput) {
    let Ok(source) = HtmlInput::new("fuzz", &input.html)
        .map(|source| source.with_input_base_url(sample_base_url()))
    else {
        return;
    };

    let Some(strategy) = input.strategy.to_plan_strategy() else {
        return;
    };

    let plan = Plan::new(
        strategy,
        input.selection.to_interop_selection(),
        Output::new(input.value_kind.to_output_kind()),
        input.normalization.to_interop_normalization(),
    );

    let _ = plan.stable_json();
    let _ = execute_plan(&source, &plan);
}

#[derive(Arbitrary, Debug)]
enum FuzzInteropStrategy {
    Selector {
        selector: String,
    },
    DelimiterPair {
        start: String,
        end: String,
        regex_mode: bool,
        include_start: bool,
        include_end: bool,
        flags: FuzzRegexFlags,
    },
}

#[derive(Arbitrary, Clone, Copy, Debug)]
struct FuzzRegexFlags {
    case_insensitive: bool,
    multi_line: bool,
    dot_matches_new_line: bool,
    swap_greed: bool,
    ignore_whitespace: bool,
}

impl FuzzRegexFlags {
    fn to_interop_flags(self) -> Vec<RegexFlag> {
        let mut flags = Vec::new();
        if self.case_insensitive {
            flags.push(RegexFlag::CaseInsensitive);
        }
        if self.multi_line {
            flags.push(RegexFlag::MultiLine);
        }
        if self.dot_matches_new_line {
            flags.push(RegexFlag::DotMatchesNewLine);
        }
        if self.swap_greed {
            flags.push(RegexFlag::SwapGreed);
        }
        if self.ignore_whitespace {
            flags.push(RegexFlag::IgnoreWhitespace);
        }
        flags
    }
}

impl FuzzInteropStrategy {
    fn to_plan_strategy(&self) -> Option<PlanStrategy> {
        match self {
            Self::Selector { selector } => SelectorQuery::new(selector.clone())
                .ok()
                .map(PlanStrategy::css_selector),
            Self::DelimiterPair {
                start,
                end,
                regex_mode,
                include_start,
                include_end,
                flags,
            } => {
                let start = SliceBoundary::new(start.clone()).ok()?;
                let end = SliceBoundary::new(end.clone()).ok()?;
                let mode = if *regex_mode {
                    DelimiterMode::Regex
                } else {
                    DelimiterMode::Literal
                };
                let flags = if matches!(mode, DelimiterMode::Regex) {
                    flags.to_interop_flags()
                } else {
                    Vec::new()
                };

                Some(PlanStrategy::delimiter_pair(
                    start,
                    end,
                    mode,
                    *include_start,
                    *include_end,
                    flags,
                ))
            }
        }
    }
}
