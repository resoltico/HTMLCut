use std::fmt;

use htmlcut_core::DiagnosticCode;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

macro_rules! cli_error_codes {
    (
        $(
            $(#[$meta:meta])*
            $variant:ident => $code:literal,
        )+
    ) => {
        /// Stable CLI-specific error codes emitted by `htmlcut-cli`.
        #[derive(
            Clone,
            Copy,
            Debug,
            Serialize,
            Deserialize,
            JsonSchema,
            PartialEq,
            Eq,
            PartialOrd,
            Ord,
            Hash,
        )]
        pub enum CliErrorCode {
            $(
                $(#[$meta])*
                #[serde(rename = $code)]
                $variant,
            )+
        }

        impl CliErrorCode {
            /// Returns the complete stable CLI error-code inventory.
            pub const ALL: &'static [Self] = &[
                $(
                    Self::$variant,
                )+
            ];

            /// Returns the stable string form of this CLI error code.
            pub const fn as_str(self) -> &'static str {
                match self {
                    $(
                        Self::$variant => $code,
                    )+
                }
            }
        }

        impl fmt::Display for CliErrorCode {
            fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str(self.as_str())
            }
        }

        impl PartialEq<&str> for CliErrorCode {
            fn eq(&self, other: &&str) -> bool {
                self.as_str() == *other
            }
        }

        impl PartialEq<CliErrorCode> for &str {
            fn eq(&self, other: &CliErrorCode) -> bool {
                *self == other.as_str()
            }
        }
    };
}

cli_error_codes! {
    /// The user asked for clap to parse invalid argv.
    ParseError => "CLI_PARSE_ERROR",
    /// Core extraction or inspection diagnostics were missing a primary error.
    PrimaryDiagnosticMissing => "CLI_PRIMARY_DIAGNOSTIC_MISSING",
    /// Writing a request definition file failed.
    RequestFileWriteFailed => "CLI_REQUEST_FILE_WRITE_FAILED",
    /// The CLI operation catalog is missing a required surface.
    ContractMissing => "CLI_CONTRACT_MISSING",
    /// The user asked for an unknown canonical operation ID.
    OperationIdUnknown => "CLI_OPERATION_ID_UNKNOWN",
    /// The user asked for an unknown schema name or version.
    SchemaUnknown => "CLI_SCHEMA_UNKNOWN",
    /// Rendering one JSON payload for CLI output failed.
    JsonRenderFailed => "CLI_JSON_RENDER_FAILED",
    /// Materializing one JSON schema document failed.
    SchemaExportFailed => "CLI_SCHEMA_EXPORT_FAILED",
    /// A required CLI parameter is missing.
    RequiredParameterMissing => "CLI_REQUIRED_PARAMETER_MISSING",
    /// The selector text is invalid at CLI preparation time.
    SelectorInvalid => "CLI_SELECTOR_INVALID",
    /// One literal or regex slice boundary is invalid at CLI preparation time.
    SliceBoundaryInvalid => "CLI_SLICE_BOUNDARY_INVALID",
    /// The user supplied mutually incompatible attribute-related flags.
    AttributeConflict => "CLI_ATTRIBUTE_CONFLICT",
    /// The supplied attribute name is invalid.
    AttributeInvalid => "CLI_ATTRIBUTE_INVALID",
    /// Attribute extraction requires an explicit attribute name.
    AttributeRequired => "CLI_ATTRIBUTE_REQUIRED",
    /// Output-file mode requires a stdout-capable payload mode.
    OutputFileRequiresStdoutPayload => "CLI_OUTPUT_FILE_REQUIRES_STDOUT_PAYLOAD",
    /// The requested HTML output mode is invalid for the selected value mode.
    OutputHtmlInvalid => "CLI_OUTPUT_HTML_INVALID",
    /// `--output none` is only valid when a bundle is requested.
    OutputNoneWithoutBundle => "CLI_OUTPUT_NONE_WITHOUT_BUNDLE",
    /// Regex flags were supplied in a context that does not accept them.
    RegexFlagsConflict => "CLI_REGEX_FLAGS_CONFLICT",
    /// Structured output requires JSON or none.
    StructuredOutputInvalid => "CLI_STRUCTURED_OUTPUT_INVALID",
    /// Match-selection flags conflict with each other.
    MatchIndexConflict => "CLI_MATCH_INDEX_CONFLICT",
    /// The supplied match index is invalid.
    MatchIndexInvalid => "CLI_MATCH_INDEX_INVALID",
    /// Nth-match selection requires an explicit match index.
    MatchIndexRequired => "CLI_MATCH_INDEX_REQUIRED",
    /// The supplied base URL is syntactically invalid.
    BaseUrlInvalid => "CLI_BASE_URL_INVALID",
    /// The supplied base URL uses an unsupported scheme.
    BaseUrlSchemeInvalid => "CLI_BASE_URL_SCHEME_INVALID",
    /// The supplied byte-size string is invalid.
    ByteSizeInvalid => "CLI_BYTE_SIZE_INVALID",
    /// The supplied preview character count is invalid.
    PreviewCharsInvalid => "CLI_PREVIEW_CHARS_INVALID",
    /// The supplied source URL is syntactically invalid.
    SourceUrlInvalid => "CLI_SOURCE_URL_INVALID",
    /// The supplied source URL uses an unsupported scheme.
    SourceUrlSchemeInvalid => "CLI_SOURCE_URL_SCHEME_INVALID",
    /// Inline CLI flags conflict with `--request-file`.
    RequestFileConflict => "CLI_REQUEST_FILE_CONFLICT",
    /// The request file could not be parsed into a supported definition.
    RequestFileInvalid => "CLI_REQUEST_FILE_INVALID",
    /// The request file could not be read from disk.
    RequestFileReadFailed => "CLI_REQUEST_FILE_READ_FAILED",
    /// The request file uses an unsupported schema identity.
    RequestFileSchemaUnsupported => "CLI_REQUEST_FILE_SCHEMA_UNSUPPORTED",
    /// The request file strategy does not match the invoked command.
    RequestFileStrategyMismatch => "CLI_REQUEST_FILE_STRATEGY_MISMATCH",
    /// `--schema-version` requires an explicit schema name filter.
    SchemaVersionRequiresName => "CLI_SCHEMA_VERSION_REQUIRES_NAME",
    /// Creating the bundle directory failed.
    BundleDirectoryCreateFailed => "CLI_BUNDLE_DIRECTORY_CREATE_FAILED",
    /// Writing the bundle HTML artifact failed.
    BundleHtmlWriteFailed => "CLI_BUNDLE_HTML_WRITE_FAILED",
    /// Writing the bundle report artifact failed.
    BundleReportWriteFailed => "CLI_BUNDLE_REPORT_WRITE_FAILED",
    /// Writing the bundle text artifact failed.
    BundleTextWriteFailed => "CLI_BUNDLE_TEXT_WRITE_FAILED",
}

/// Stable typed error-code surface for CLI reports.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(untagged)]
pub enum ErrorReportCode {
    /// One core diagnostic code projected directly into a CLI report.
    Core(DiagnosticCode),
    /// One CLI-specific error code projected into a CLI report.
    Cli(CliErrorCode),
}

impl ErrorReportCode {
    /// Returns the stable string form of this code.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Core(code) => code.as_str(),
            Self::Cli(code) => code.as_str(),
        }
    }
}

impl fmt::Display for ErrorReportCode {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

impl PartialEq<&str> for ErrorReportCode {
    fn eq(&self, other: &&str) -> bool {
        self.as_str() == *other
    }
}

impl PartialEq<ErrorReportCode> for &str {
    fn eq(&self, other: &ErrorReportCode) -> bool {
        *self == other.as_str()
    }
}

impl From<CliErrorCode> for ErrorReportCode {
    fn from(value: CliErrorCode) -> Self {
        Self::Cli(value)
    }
}

impl From<DiagnosticCode> for ErrorReportCode {
    fn from(value: DiagnosticCode) -> Self {
        Self::Core(value)
    }
}
