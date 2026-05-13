use arbitrary::Arbitrary;
use htmlcut_core::interop::v1::{
    CssSelectorText, DelimiterBoundaryRetention, DelimiterBoundaryText, DelimiterMode, HtmlInput,
    Plan, PlanStrategy, RegexFlag, execute_plan,
};

use crate::interop_common::{FuzzRendering, FuzzSelection, FuzzValueKind, sample_base_url};

#[derive(Arbitrary, Debug)]
pub struct InteropInput {
    html: String,
    strategy: FuzzInteropStrategy,
    value_kind: FuzzValueKind,
    selection: FuzzSelection,
    rendering: FuzzRendering,
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
    let strategy_kind = strategy.kind();

    let plan = Plan::new(
        strategy,
        input.selection.to_interop_selection(),
        input.value_kind.to_output(strategy_kind),
        input.rendering.to_interop_rendering(),
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
            Self::Selector { selector } => CssSelectorText::new(selector.clone())
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
                let start = DelimiterBoundaryText::new(start.clone()).ok()?;
                let end = DelimiterBoundaryText::new(end.clone()).ok()?;
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
                    DelimiterBoundaryRetention::from_flags(*include_start, *include_end),
                    flags,
                ))
            }
        }
    }
}
