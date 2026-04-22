mod preview;
mod shared;
mod source;

pub(crate) use self::preview::render_preview_text;
#[cfg(test)]
pub(crate) use self::preview::{render_preview_location, render_preview_match_lines};
pub(crate) use self::shared::build_human_diagnostic_stderr_lines;
#[cfg(test)]
pub(crate) use self::shared::{
    compact_inline_preview, render_attribute_summary, render_diagnostic_level,
    render_range_summary, render_source_kind,
};
#[cfg(test)]
pub(crate) use self::source::render_text_preview;
pub(crate) use self::source::{
    build_source_inspection_verbose_lines, build_source_load_error_lines, build_verbose_lines,
    fallback_document_title, render_source_inspection_text,
};
