// SPDX-License-Identifier: MPL-2.0
//! Accepted immutable source and lazy, failure-caching preparation.

use std::cell::OnceCell;
use std::sync::{Arc, OnceLock};

use schemars::JsonSchema;
use scraper::{Html, html::ParseLimits};
use serde::{Deserialize, Serialize};
use url::Url;

use crate::{ErrorCode, ExtractionError, PreparationLimits};

/// Explicit caller/acquisition metadata; HTML base elements are never inferred.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(default, deny_unknown_fields)]
pub struct SnapshotMetadata {
    /// Effective absolute HTTP(S) base URL; operational evidence must redact its query.
    pub base_url: Option<String>,
}

/// Exact accepted UTF-8 snapshot with no filesystem/network identity.
#[derive(Clone, Debug)]
pub struct SourceSnapshot {
    html: Arc<str>,
    metadata: SnapshotMetadata,
    source_digest: Arc<OnceLock<String>>,
}

impl SourceSnapshot {
    /// Accepts exact UTF-8 bytes; does not strip BOMs or normalize line endings.
    pub fn new(html: impl AsRef<str>, metadata: SnapshotMetadata) -> Result<Self, ExtractionError> {
        let html = html.as_ref();
        if html.len() > PreparationLimits::default().max_source_bytes as usize {
            return Err(ExtractionError::resource(
                "source",
                "max_source_bytes",
                PreparationLimits::default().max_source_bytes.into(),
            ));
        }
        let metadata = match metadata.base_url {
            None => SnapshotMetadata::default(),
            Some(value) => {
                if value.len() > crate::limits::MAX_URL_INPUT_BYTES {
                    return Err(ExtractionError::resource(
                        "metadata",
                        "base_url_bytes",
                        crate::limits::MAX_URL_INPUT_BYTES as u64,
                    ));
                }
                let url = Url::parse(&value).map_err(|_| invalid_base())?;
                if !matches!(url.scheme(), "http" | "https")
                    || !url.username().is_empty()
                    || url.password().is_some()
                {
                    return Err(invalid_base());
                }
                if url.as_str().len() > crate::limits::MAX_URL_INPUT_BYTES {
                    return Err(ExtractionError::resource(
                        "metadata",
                        "base_url_bytes",
                        crate::limits::MAX_URL_INPUT_BYTES as u64,
                    ));
                }
                SnapshotMetadata {
                    base_url: Some(url.into()),
                }
            }
        };
        Ok(Self {
            html: Arc::from(html),
            metadata,
            source_digest: Arc::new(OnceLock::new()),
        })
    }
    /// Exact accepted source, including CRLF and original source spelling.
    pub fn html(&self) -> &str {
        &self.html
    }
    /// Explicit metadata, never derived from DOM contents.
    pub fn metadata(&self) -> &SnapshotMetadata {
        &self.metadata
    }
    /// SHA-256 of exact accepted UTF-8 bytes.
    pub fn source_sha256(&self) -> &str {
        self.source_digest
            .get_or_init(|| crate::identity::sha256(self.html.as_bytes()))
    }
}

fn invalid_base() -> ExtractionError {
    ExtractionError::new(
        ErrorCode::InvalidBaseUrl,
        "source",
        "Base metadata requires an absolute HTTP(S) URL without userinfo.",
    )
}

/// Opaque snapshot with a lazily prepared DOM cached at most once, including failures.
pub struct PreparedDocument {
    pub(crate) snapshot: SourceSnapshot,
    pub(crate) limits: PreparationLimits,
    dom: OnceCell<Result<Html, ExtractionError>>,
    digest: OnceCell<String>,
    #[cfg(test)]
    parses: std::cell::Cell<u32>,
}

impl PreparedDocument {
    /// Accepts and binds immutable preparation policy without parsing HTML.
    pub fn new(
        snapshot: SourceSnapshot,
        limits: PreparationLimits,
    ) -> Result<Self, ExtractionError> {
        limits.validate()?;
        if snapshot.html.len() > limits.max_source_bytes as usize {
            return Err(ExtractionError::resource(
                "source",
                "max_source_bytes",
                limits.max_source_bytes.into(),
            ));
        }
        Ok(Self {
            snapshot,
            limits,
            dom: OnceCell::new(),
            digest: OnceCell::new(),
            #[cfg(test)]
            parses: std::cell::Cell::new(0),
        })
    }
    /// Immutable accepted input.
    pub fn snapshot(&self) -> &SourceSnapshot {
        &self.snapshot
    }
    /// Actual immutable preparation policy, needed for self-contained replay.
    pub fn preparation_limits(&self) -> &PreparationLimits {
        &self.limits
    }
    /// Snapshot/metadata/preparation-policy identity, distinct from extraction data.
    pub fn prepared_sha256(&self) -> &str {
        self.digest.get_or_init(|| {
            let metadata = crate::canonical_json(&self.snapshot.metadata)
                .expect("validated metadata serializes");
            let policy =
                crate::canonical_json(&self.limits).expect("validated preparation serializes");
            crate::identity::framed(
                "htmlcut.prepared/3",
                &[
                    self.snapshot.source_sha256().as_bytes(),
                    metadata.as_bytes(),
                    policy.as_bytes(),
                ],
            )
        })
    }
    fn prepared_dom(&self) -> Result<&Html, ExtractionError> {
        self.dom
            .get_or_init(|| {
                #[cfg(test)]
                self.parses.set(self.parses.get() + 1);
                Html::parse_document_bounded(
                    self.snapshot.html(),
                    ParseLimits {
                        elements: self.limits.max_elements,
                        nodes: self.limits.max_nodes,
                        depth: self.limits.max_depth,
                        work: self.limits.max_parse_work,
                    },
                )
                .map_err(|failure| {
                    use scraper::html::ParseLimitExceeded;
                    let (counter, bound) = match failure {
                        ParseLimitExceeded::Elements => ("max_elements", self.limits.max_elements),
                        ParseLimitExceeded::Nodes => ("max_nodes", self.limits.max_nodes),
                        ParseLimitExceeded::Depth => ("max_depth", self.limits.max_depth),
                        ParseLimitExceeded::Work => ("max_parse_work", self.limits.max_parse_work),
                    };
                    ExtractionError::resource("preparation", counter, bound.into())
                })
            })
            .as_ref()
            .map_err(Clone::clone)
    }
    pub(crate) fn document(&self) -> Result<&Html, ExtractionError> {
        self.prepared_dom()
    }
    #[cfg(test)]
    pub(crate) fn parse_count(&self) -> u32 {
        self.parses.get()
    }
}
