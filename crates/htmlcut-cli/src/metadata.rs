pub(crate) const TOOL_NAME: &str = "htmlcut";
pub(crate) const DISPLAY_NAME: &str = "HTMLCut";
pub(crate) const ENGINE_NAME: &str = "htmlcut-core";
pub(crate) const HTMLCUT_VERSION: &str = env!("CARGO_PKG_VERSION");
pub(crate) const HTMLCUT_DESCRIPTION: &str = env!("CARGO_PKG_DESCRIPTION");
pub(crate) const HTMLCUT_REPOSITORY: &str = env!("CARGO_PKG_REPOSITORY");

pub(crate) fn identity_banner() -> String {
    format!("{DISPLAY_NAME} {HTMLCUT_VERSION}\n{HTMLCUT_DESCRIPTION}")
}

pub(crate) fn version_banner() -> String {
    format!(
        "{}\nengine: {ENGINE_NAME}\nschema-profile: {}\nrepository: {HTMLCUT_REPOSITORY}",
        identity_banner(),
        htmlcut_core::HTMLCUT_JSON_SCHEMA_PROFILE
    )
}
