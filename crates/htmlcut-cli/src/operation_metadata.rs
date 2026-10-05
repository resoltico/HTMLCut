// SPDX-License-Identifier: MPL-2.0
//! One narrow discovery/help vocabulary, separate from executable extraction semantics.

use serde_json::{Value, json};

use crate::input::options;

pub(crate) const EXTRACT_ABOUT: &str =
    "Extract requested values; defaults to single/dom_text and bare data JSON.";
pub(crate) const EXTRACT_DETAILS: &str = "Accept exactly one source and one inline selection or plan. Default single requires exactly one candidate. --read chooses literal or normalized text, Markdown, HTML, attributes or source bytes. dom_text concatenates parsed text literally, including hidden content. Repeated --field NAME CSS READ declares record fields; NAME? permits zero or one match and returns null when absent. --field-exclude NAME CSS removes matching descendants from one declared field. --following-siblings includes an explicit bounded sibling group. Advanced cardinality and guards use a plan. --raw requires one value and adds no LF. Semantic failures publish no values. HTML base elements are ignored; relative URL resolution requires an explicit --base-url.";
pub(crate) const RUN_ABOUT: &str =
    "Recompute and verify a self-contained snapshot, plan and receipt bundle.";
pub(crate) const INSPECT_ABOUT: &str = "Count an explicit CSS selector; --identifiers shows bounded id/class and structural text samples.";
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
                    json!({"selection":"single","read":"dom_text","output":"data_json"});
            }
            Ok(value)
        }
    }
}
