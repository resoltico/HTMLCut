//! Accepted immutable source and lazy, failure-caching preparation.

use std::cell::OnceCell;
use std::sync::Arc;

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
    source_digest: String,
}

impl SourceSnapshot {
    /// Accepts exact UTF-8 bytes; does not strip BOMs or normalize line endings.
    pub fn new(html: impl AsRef<str>, metadata: SnapshotMetadata) -> Result<Self, ExtractionError> {
        let html = html.as_ref();
        if html.len() > PreparationLimits::default().max_source_bytes as usize {
            return Err(ExtractionError::limit("source"));
        }
        let metadata = match metadata.base_url {
            None => SnapshotMetadata::default(),
            Some(value) => {
                if value.len() > crate::limits::MAX_URL_INPUT_BYTES {
                    return Err(ExtractionError::limit("metadata"));
                }
                let url = Url::parse(&value).map_err(|_| invalid_base())?;
                if !matches!(url.scheme(), "http" | "https")
                    || !url.username().is_empty()
                    || url.password().is_some()
                {
                    return Err(invalid_base());
                }
                if url.as_str().len() > crate::limits::MAX_URL_INPUT_BYTES {
                    return Err(ExtractionError::limit("metadata"));
                }
                SnapshotMetadata {
                    base_url: Some(url.into()),
                }
            }
        };
        let source_digest = crate::identity::sha256(html.as_bytes());
        Ok(Self {
            html: Arc::from(html),
            metadata,
            source_digest,
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
        &self.source_digest
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
    dom: OnceCell<Result<PreparedDom, ExtractionError>>,
    digest: String,
    #[cfg(test)]
    parses: std::cell::Cell<u32>,
}

struct PreparedDom {
    html: Html,
    elements: Vec<ego_tree::NodeId>,
}

impl PreparedDocument {
    /// Accepts and binds immutable preparation policy without parsing HTML.
    pub fn new(
        snapshot: SourceSnapshot,
        limits: PreparationLimits,
    ) -> Result<Self, ExtractionError> {
        limits.validate()?;
        if snapshot.html.len() > limits.max_source_bytes as usize {
            return Err(ExtractionError::limit("source"));
        }
        let metadata = crate::canonical_json(&snapshot.metadata)?;
        let policy = crate::canonical_json(&limits)?;
        let digest = crate::identity::framed(
            "htmlcut.prepared/1",
            &[
                snapshot.source_digest.as_bytes(),
                metadata.as_bytes(),
                policy.as_bytes(),
            ],
        );
        Ok(Self {
            snapshot,
            limits,
            dom: OnceCell::new(),
            digest,
            #[cfg(test)]
            parses: std::cell::Cell::new(0),
        })
    }
    /// Immutable accepted input.
    pub fn snapshot(&self) -> &SourceSnapshot {
        &self.snapshot
    }
    /// Snapshot/metadata/preparation-policy identity for bound discovery evidence.
    pub fn prepared_sha256(&self) -> &str {
        &self.digest
    }
    fn prepared_dom(&self) -> Result<&PreparedDom, ExtractionError> {
        self.dom
            .get_or_init(|| {
                #[cfg(test)]
                self.parses.set(self.parses.get() + 1);
                Html::parse_document_indexed(
                    self.snapshot.html(),
                    ParseLimits {
                        elements: self.limits.max_elements,
                        nodes: self.limits.max_nodes,
                        depth: self.limits.max_depth,
                        work: self.limits.max_parse_work,
                    },
                )
                .map(|(html, elements)| PreparedDom { html, elements })
                .map_err(|_| ExtractionError::limit("preparation"))
            })
            .as_ref()
            .map_err(Clone::clone)
    }
    pub(crate) fn document(&self) -> Result<&Html, ExtractionError> {
        self.prepared_dom().map(|dom| &dom.html)
    }
    pub(crate) fn element_ids(&self) -> Result<&[ego_tree::NodeId], ExtractionError> {
        self.prepared_dom().map(|dom| dom.elements.as_slice())
    }
    #[cfg(test)]
    pub(crate) fn parse_count(&self) -> u32 {
        self.parses.get()
    }
}
