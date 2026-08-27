mod markup;
mod matches;
mod patterns;

#[cfg(test)]
pub(crate) use markup::{
    markup_cursor_step_is_valid_for_tests, markup_position_is_in_bounds_for_tests,
    position_inside_markup_for_tests, position_inside_markup_rejects_invalid_progress_for_tests,
    position_inside_markup_rejects_out_of_bounds_progress_for_tests,
    position_inside_markup_stalled_step_count_for_tests,
};
#[cfg(test)]
pub(crate) use matches::build_slice_match;
#[cfg(test)]
pub(crate) use matches::run_slice_extraction;
pub(crate) use matches::run_validated_slice_extraction;
pub(crate) use patterns::CompiledSlicePatterns;
#[cfg(test)]
pub(crate) use patterns::{
    build_finder, build_regex, extract_slice_candidates, slice_cursor_progress_for_tests,
};
