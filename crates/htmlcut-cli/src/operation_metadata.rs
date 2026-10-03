//! One narrow discovery/help vocabulary, separate from executable extraction semantics.

use serde_json::{Value, json};

use crate::input::options;

pub(crate) const EXTRACT_ABOUT: &str =
    "Extract requested values; defaults to single/dom_text and bare data JSON.";
pub(crate) const EXTRACT_DETAILS: &str = "Accept exactly one source and one inline selection or plan. Default single requires exactly one candidate. dom_text concatenates parsed text literally, including hidden content. --raw requires one value and adds no LF. markdown, HTML and attributes are explicit projections; slices return exact source bytes. Semantic failures publish no values. HTML base elements are ignored; URL resolution requires an explicit plan transform.";
pub(crate) const RUN_ABOUT: &str =
    "Recompute and verify a self-contained snapshot, plan and receipt bundle.";
pub(crate) const INSPECT_ABOUT: &str =
    "Count an explicit CSS selector and show up to three bounded samples.";
pub(crate) const DESCRIBE_ABOUT: &str =
    "Retrieve the compact operation index or one operation description.";
pub(crate) const SCHEMA_ABOUT: &str =
    "Retrieve one named schema; loading all schemas is unnecessary.";

const OPERATIONS: &[(&str, &str)] = &[
    ("extract", EXTRACT_ABOUT),
    ("run", RUN_ABOUT),
    ("inspect", INSPECT_ABOUT),
    ("describe", DESCRIBE_ABOUT),
    ("schema", SCHEMA_ABOUT),
];

pub(crate) fn describe(name: Option<&str>) -> Result<Value, htmlcut_core::ExtractionError> {
    match name {
        None => Ok(
            json!({ "schema": "htmlcut.operations", "version": htmlcut_core::SCHEMA_VERSION,
            "operations": OPERATIONS.iter().map(|(name, summary)| json!({"name":name,"summary":summary})).collect::<Vec<_>>() }),
        ),
        Some(name) => {
            let (_, summary) = OPERATIONS
                .iter()
                .find(|(operation, _)| *operation == name)
                .ok_or_else(|| options("The requested operation name is unsupported."))?;
            let mut value = json!({"schema":"htmlcut.operation","version":htmlcut_core::SCHEMA_VERSION,"name":name,"summary":summary});
            if name == "extract" {
                value["details"] = json!(EXTRACT_DETAILS);
                value["plan_schema"] = json!("htmlcut.extraction.plan");
                value["defaults"] =
                    json!({"selection":"single","projection":"dom_text","output":"data_json"});
            }
            Ok(value)
        }
    }
}
