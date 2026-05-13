#[cfg(test)]
use std::collections::BTreeSet;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::{
    CORE_REQUEST_SCHEMA_VERSION, CORE_RESULT_SCHEMA_NAME, CORE_RESULT_SCHEMA_VERSION,
    CORE_SOURCE_INSPECTION_SCHEMA_NAME, CORE_SOURCE_INSPECTION_SCHEMA_VERSION,
    EXTRACTION_REQUEST_SCHEMA_NAME, INSPECTION_OPTIONS_SCHEMA_NAME, RUNTIME_OPTIONS_SCHEMA_NAME,
    SOURCE_REQUEST_SCHEMA_NAME, SchemaRef,
};

macro_rules! operation_ids {
    (
        $(
            $(#[$meta:meta])*
            $variant:ident => $id:literal,
        )+
    ) => {
        /// Stable identifiers for HTMLCut's canonical user-facing operations.
        ///
        /// These IDs are intentionally narrow: they exist only for operations that callers can
        /// invoke as product behavior across the CLI and embeddable core. Helper functions, flags,
        /// and internal implementation details do not get operation IDs.
        #[derive(
            Clone, Copy, Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq, PartialOrd, Ord, Hash,
        )]
        pub enum OperationId {
            $(
                $(#[$meta])*
                #[serde(rename = $id)]
                $variant,
            )+
        }

        impl OperationId {
            /// Every stable operation ID in declaration order.
            pub const ALL: &'static [Self] = &[
                $(
                    Self::$variant,
                )+
            ];

            /// Returns the stable string form of this operation ID.
            pub const fn as_str(self) -> &'static str {
                match self {
                    $(
                        Self::$variant => $id,
                    )+
                }
            }
        }

        impl std::str::FromStr for OperationId {
            type Err = OperationIdParseError;

            fn from_str(value: &str) -> Result<Self, Self::Err> {
                match value {
                    $(
                        $id => Ok(Self::$variant),
                    )+
                    _ => Err(OperationIdParseError),
                }
            }
        }
    };
}

operation_ids! {
    /// Load and parse HTML into a document tree for in-process callers.
    DocumentParse => "document.parse",
    /// Inspect the parsed document and summarize structure, samples, and base-URL behavior.
    SourceInspect => "source.inspect",
    /// Preview selector matches without committing to a final extraction payload.
    SelectPreview => "select.preview",
    /// Preview literal or regex slices without committing to a final extraction payload.
    SlicePreview => "slice.preview",
    /// Extract final values from CSS selector matches.
    SelectExtract => "select.extract",
    /// Extract final values between literal or regex boundaries in raw source text.
    SliceExtract => "slice.extract",
}

/// Error returned when parsing an unknown operation ID string.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct OperationIdParseError;

/// Structured contract surface for one operation input or output.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct OperationContract {
    /// Public contract family name exposed to operators and embedders.
    pub family: &'static str,
    /// JSON schema references when the contract has an exported JSON form.
    pub schema_refs: &'static [SchemaRef],
}

/// A catalog entry that binds one stable operation ID to the CLI and core surfaces that expose it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct OperationDescriptor {
    /// The stable canonical operation identifier.
    pub id: OperationId,
    /// The CLI command surface when the operation is directly user-invokable from the CLI.
    pub cli_surface: Option<&'static str>,
    /// Public name for the embeddable core API capability behind this operation.
    pub core_api: &'static str,
    /// The public request contract for the operation.
    pub request_contract: OperationContract,
    /// The public result contract for the operation.
    pub result_contract: OperationContract,
    /// A concise statement of what the operation does.
    pub description: &'static str,
}

const NO_SCHEMA_REFS: &[SchemaRef] = &[];
const SOURCE_RUNTIME_SCHEMA_REFS: &[SchemaRef] = &[
    SchemaRef::new(SOURCE_REQUEST_SCHEMA_NAME, CORE_REQUEST_SCHEMA_VERSION),
    SchemaRef::new(RUNTIME_OPTIONS_SCHEMA_NAME, CORE_REQUEST_SCHEMA_VERSION),
];
const SOURCE_RUNTIME_INSPECTION_SCHEMA_REFS: &[SchemaRef] = &[
    SchemaRef::new(SOURCE_REQUEST_SCHEMA_NAME, CORE_REQUEST_SCHEMA_VERSION),
    SchemaRef::new(RUNTIME_OPTIONS_SCHEMA_NAME, CORE_REQUEST_SCHEMA_VERSION),
    SchemaRef::new(INSPECTION_OPTIONS_SCHEMA_NAME, CORE_REQUEST_SCHEMA_VERSION),
];
const EXTRACTION_RUNTIME_SCHEMA_REFS: &[SchemaRef] = &[
    SchemaRef::new(EXTRACTION_REQUEST_SCHEMA_NAME, CORE_REQUEST_SCHEMA_VERSION),
    SchemaRef::new(RUNTIME_OPTIONS_SCHEMA_NAME, CORE_REQUEST_SCHEMA_VERSION),
];
const EXTRACTION_RESULT_SCHEMA_REFS: &[SchemaRef] = &[SchemaRef::new(
    CORE_RESULT_SCHEMA_NAME,
    CORE_RESULT_SCHEMA_VERSION,
)];
const SOURCE_INSPECTION_RESULT_SCHEMA_REFS: &[SchemaRef] = &[SchemaRef::new(
    CORE_SOURCE_INSPECTION_SCHEMA_NAME,
    CORE_SOURCE_INSPECTION_SCHEMA_VERSION,
)];

/// Canonical catalog of every stable HTMLCut operation ID.
pub const OPERATION_CATALOG: &[OperationDescriptor] = &[
    OperationDescriptor {
        id: OperationId::DocumentParse,
        cli_surface: None,
        core_api: "parse document",
        request_contract: OperationContract {
            family: "source request + runtime options",
            schema_refs: SOURCE_RUNTIME_SCHEMA_REFS,
        },
        result_contract: OperationContract {
            family: "parsed document result",
            schema_refs: NO_SCHEMA_REFS,
        },
        description: "Load and parse HTML into a document tree for in-process callers.",
    },
    OperationDescriptor {
        id: OperationId::SourceInspect,
        cli_surface: Some("inspect source"),
        core_api: "inspect source",
        request_contract: OperationContract {
            family: "source request + runtime options + inspection options",
            schema_refs: SOURCE_RUNTIME_INSPECTION_SCHEMA_REFS,
        },
        result_contract: OperationContract {
            family: "source inspection result",
            schema_refs: SOURCE_INSPECTION_RESULT_SCHEMA_REFS,
        },
        description: "Inspect the parsed document and summarize structure, samples, and base-URL behavior.",
    },
    OperationDescriptor {
        id: OperationId::SelectPreview,
        cli_surface: Some("inspect select"),
        core_api: "preview selector extraction",
        request_contract: OperationContract {
            family: "extraction request + runtime options",
            schema_refs: EXTRACTION_RUNTIME_SCHEMA_REFS,
        },
        result_contract: OperationContract {
            family: "extraction result",
            schema_refs: EXTRACTION_RESULT_SCHEMA_REFS,
        },
        description: "Preview selector matches without committing to a final extraction payload.",
    },
    OperationDescriptor {
        id: OperationId::SlicePreview,
        cli_surface: Some("inspect slice"),
        core_api: "preview slice extraction",
        request_contract: OperationContract {
            family: "extraction request + runtime options",
            schema_refs: EXTRACTION_RUNTIME_SCHEMA_REFS,
        },
        result_contract: OperationContract {
            family: "extraction result",
            schema_refs: EXTRACTION_RESULT_SCHEMA_REFS,
        },
        description: "Preview literal or regex slices without committing to a final extraction payload.",
    },
    OperationDescriptor {
        id: OperationId::SelectExtract,
        cli_surface: Some("select"),
        core_api: "extract selector values",
        request_contract: OperationContract {
            family: "extraction request + runtime options",
            schema_refs: EXTRACTION_RUNTIME_SCHEMA_REFS,
        },
        result_contract: OperationContract {
            family: "extraction result",
            schema_refs: EXTRACTION_RESULT_SCHEMA_REFS,
        },
        description: "Extract final values from CSS selector matches.",
    },
    OperationDescriptor {
        id: OperationId::SliceExtract,
        cli_surface: Some("slice"),
        core_api: "extract slice values",
        request_contract: OperationContract {
            family: "extraction request + runtime options",
            schema_refs: EXTRACTION_RUNTIME_SCHEMA_REFS,
        },
        result_contract: OperationContract {
            family: "extraction result",
            schema_refs: EXTRACTION_RESULT_SCHEMA_REFS,
        },
        description: "Extract final values between literal or regex boundaries in raw source text.",
    },
];

impl std::fmt::Display for OperationId {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(self.as_str())
    }
}

impl std::fmt::Display for OperationIdParseError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("unknown HTMLCut operation ID")
    }
}

impl std::error::Error for OperationIdParseError {}

/// Returns the canonical catalog of HTMLCut operations.
pub fn operation_catalog() -> &'static [OperationDescriptor] {
    OPERATION_CATALOG
}

/// Returns the descriptor for one canonical operation ID.
pub fn operation_descriptor(id: OperationId) -> Option<&'static OperationDescriptor> {
    operation_catalog()
        .iter()
        .find(|descriptor| descriptor.id == id)
}

#[cfg(test)]
pub(crate) fn operation_catalog_contract_string_errors_for_tests() -> Vec<String> {
    operation_catalog_contract_string_errors(operation_catalog())
}

#[cfg(test)]
pub(crate) fn operation_catalog_contract_string_errors_for_tests_with(
    catalog: &[OperationDescriptor],
) -> Vec<String> {
    operation_catalog_contract_string_errors(catalog)
}

#[cfg(test)]
pub(crate) fn assert_operation_catalog_contract_strings_for_tests(catalog: &[OperationDescriptor]) {
    let errors = operation_catalog_contract_string_errors(catalog);
    assert!(
        errors.is_empty(),
        "operation catalog contract strings drifted:\n- {}",
        errors.join("\n- ")
    );
}

#[cfg(test)]
fn operation_catalog_contract_string_errors(catalog: &[OperationDescriptor]) -> Vec<String> {
    let mut errors = Vec::new();
    let mut seen_ids = BTreeSet::new();

    for descriptor in catalog {
        if !seen_ids.insert(descriptor.id) {
            errors.push(format!(
                "{} appears more than once in OPERATION_CATALOG",
                descriptor.id
            ));
        }

        if descriptor.core_api.trim().is_empty() {
            errors.push(format!("{} has an empty core_api", descriptor.id));
        }
        if descriptor.request_contract.family.trim().is_empty() {
            errors.push(format!("{} has an empty request family", descriptor.id));
        }
        if descriptor.result_contract.family.trim().is_empty() {
            errors.push(format!("{} has an empty result family", descriptor.id));
        }
    }

    for operation_id in OperationId::ALL {
        if !seen_ids.contains(operation_id) {
            errors.push(format!("{operation_id} is missing from OPERATION_CATALOG"));
        }
    }

    errors
}
