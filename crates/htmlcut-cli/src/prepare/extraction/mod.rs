mod extract;
mod preview;
mod shared;

use htmlcut_core::DEFAULT_REGEX_FLAGS;

pub(crate) fn default_regex_flags() -> String {
    DEFAULT_REGEX_FLAGS.to_owned()
}
