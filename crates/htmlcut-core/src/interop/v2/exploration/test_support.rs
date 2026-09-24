use std::num::NonZeroU32;

use crate::interop::v2::PreparedDocument;

use super::{ExplorationCursor, ExplorationOptions, discovery_identity, options_digest};

pub(crate) fn exploration_cursor_for_tests(
    document: &PreparedDocument,
    options: &ExplorationOptions,
    next_document_ordinal: NonZeroU32,
) -> ExplorationCursor {
    ExplorationCursor::new(
        discovery_identity(document),
        options_digest(options),
        next_document_ordinal,
    )
}
