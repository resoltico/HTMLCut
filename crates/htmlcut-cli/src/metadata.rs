pub(crate) const TOOL_NAME: &str = "htmlcut";
pub(crate) const ENGINE_NAME: &str = "htmlcut-core";
pub(crate) const HTMLCUT_VERSION: &str = env!("CARGO_PKG_VERSION");
pub(crate) const HTMLCUT_DESCRIPTION: &str = env!("CARGO_PKG_DESCRIPTION");

pub(crate) fn version_banner() -> String {
    format!("{TOOL_NAME} {HTMLCUT_VERSION}\n{HTMLCUT_DESCRIPTION}")
}
