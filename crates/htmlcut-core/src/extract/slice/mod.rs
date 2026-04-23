mod markup;
mod matches;
mod patterns;
mod selection;

#[cfg(test)]
pub(crate) use markup::position_inside_markup_for_tests;
#[cfg(test)]
pub(crate) use matches::build_slice_match;
pub(crate) use matches::run_slice_extraction;
#[cfg(test)]
pub(crate) use patterns::{build_finder, build_regex, extract_slice_candidates};
pub(crate) use selection::select_candidates;
