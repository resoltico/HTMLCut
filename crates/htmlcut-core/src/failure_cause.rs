//! Closed recovery information without untrusted transport or source values.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Safe owning configuration role, independent of supplied values.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ConfigurationRole {
    /// JSON syntax or object interpretation.
    Document,
    /// Extraction plan.
    Plan,
    /// Source specification.
    Source,
    /// Explicit character encoding.
    Encoding,
    /// Command-line arguments.
    Arguments,
    /// Explicit source metadata.
    Metadata,
}

/// Portable configuration failure category.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ConfigurationProblem {
    /// Malformed syntax.
    Syntax,
    /// An object contains fields outside its declared shape.
    UnknownFields,
    /// Contract version is not current.
    UnsupportedVersion,
    /// A value violates a declared constraint.
    InvalidValue,
    /// Required input is absent.
    MissingRequired,
    /// Two options cannot be used together.
    ConflictingOptions,
}

/// Real I/O boundary at which a failure happened.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum IoOperation {
    /// Reading a regular input/configuration file.
    Input,
    /// Delivering stdout bytes.
    Stdout,
    /// Staging or committing a file.
    Publication,
}

/// Safe portable I/O failure categories; no path or OS message is retained.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum IoProblem {
    /// The input does not exist.
    NotFound,
    /// Access is denied.
    PermissionDenied,
    /// A descriptor cannot perform the requested operation.
    InvalidDescriptor,
    /// A pipe has no reader.
    BrokenPipe,
    /// Input is not an ordinary file.
    UnsupportedKind,
    /// I/O failed for a reason not otherwise classified.
    Other,
}

/// Safe transport classification from typed errors, never their text.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum TransportProblem {
    /// Hostname resolution failed.
    Dns,
    /// A connection could not be established.
    Connection,
    /// TLS authentication or negotiation failed.
    Tls,
    /// A transfer deadline expired.
    Timeout,
    /// Response framing is invalid or incomplete.
    Framing,
    /// A body stream is incomplete.
    IncompleteBody,
    /// An opaque transport failure could not be classified more precisely.
    Other,
}

/// Bounded, closed cause facts supplied only when known by the owning boundary.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum FailureCause {
    /// Configuration rejection without supplied values.
    Configuration {
        /// Owning role.
        role: ConfigurationRole,
        /// Failed constraint category.
        problem: ConfigurationProblem,
    },
    /// Real I/O failure.
    Io {
        /// Operation requiring I/O.
        operation: IoOperation,
        /// Portable reason.
        problem: IoProblem,
    },
    /// Final HTTP status was received and rejected.
    HttpStatus {
        /// Numeric received status, not a fabricated status for transport errors.
        status: u16,
    },
    /// Typed transport failure.
    Transport {
        /// Safe typed category.
        problem: TransportProblem,
    },
    /// Media-type parameter grammar or conflicting metadata.
    MediaType {},
    /// Origin encoding/BOM/bytes were inconsistent.
    Charset {},
    /// Content encoding is unsupported or invalid.
    Compression {},
    /// The full-source GET returned a partial representation.
    PartialResponse {
        /// Actual received status.
        status: u16,
    },
    /// A quantitative bound was exhausted.
    Resource {},
    /// A known option name and safe constraint, derived from command definitions.
    Option {
        /// Definition-owned option, never an arbitrary user token.
        #[schemars(length(max = 64))]
        option: String,
        /// Constraint category.
        problem: ConfigurationProblem,
    },
}
