//! Bounded discovery bound to one prepared snapshot and option set.

use schemars::JsonSchema;
use scraper::{ElementRef, Node};
use selectors::work_budget::SelectorWorkBudget;
use serde::{Deserialize, Serialize};

use crate::{CompiledPlan, ErrorCode, ExtractionError, PreparedDocument, SCHEMA_VERSION};

/// Bounded descriptor of one original element; preview is explicitly incomplete when shortened.
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ElementDescriptor {
    /// Snapshot/options-bound element handle, not a durable semantic identity.
    pub handle: String,
    /// Parsed tag name.
    pub tag: String,
    /// At most eight source attribute previews, separate from operational metadata.
    pub attributes: Vec<AttributePreview>,
    /// Whether all attribute descriptors are included.
    pub attributes_complete: bool,
    /// Bounded literal descendant-text preview.
    pub preview: String,
    /// Whether the text preview covers all descendant text.
    pub complete: bool,
}

/// A bounded literal source attribute preview, never a complete extraction value when shortened.
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct AttributePreview {
    /// Parsed attribute name.
    pub name: String,
    /// First sixty-four Unicode characters of its parsed value.
    pub value: String,
    /// True when the entire parsed value is included.
    pub complete: bool,
}

/// One advancing descriptor page, distinct from an extraction result.
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct InspectionResult {
    /// Inspection role name.
    pub schema: String,
    /// Wire version.
    pub version: u32,
    /// Bound prepared-snapshot identity.
    pub prepared_sha256: String,
    /// Bounded original-DOM descriptors.
    pub elements: Vec<ElementDescriptor>,
    /// Next page cursor; absent at the terminal page.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_cursor: Option<String>,
}

/// Intentional abbreviated projection; safety-budget exhaustion is still an error.
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct PreviewResult {
    /// Preview role name.
    pub schema: String,
    /// Wire version.
    pub version: u32,
    /// Bound prepared-snapshot identity.
    pub prepared_sha256: String,
    /// Normalized plan identity.
    pub plan_sha256: String,
    /// Complete extraction candidate count, established under normal core budgets.
    pub candidate_count: u32,
    /// Complete selection count; may exceed the preview values shown.
    pub selected_count: u32,
    /// At most twenty values and the requested aggregate preview-character allowance.
    pub values: Vec<String>,
    /// True only when neither values nor content were abbreviated.
    pub complete: bool,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Token {
    prepared: String,
    options: String,
    position: u32,
    role: String,
    seal: String,
}

/// A snapshot-bound selector suggestion; caller adoption is required before extraction.
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct SelectorProposal {
    /// Proposal role name.
    pub schema: String,
    /// Wire version.
    pub version: u32,
    /// Always suggestion; no confidence or semantic-correctness claim.
    pub kind: String,
    /// Bound prepared identity.
    pub prepared_sha256: String,
    /// Original snapshot/options-bound handle.
    pub handle: String,
    /// Bounded CSS path for explicit caller selection.
    pub selector: String,
}

fn binding(page_size: u32) -> String {
    crate::identity::framed(
        "htmlcut.discovery-options/1",
        &[
            &page_size.to_be_bytes(),
            &64_u32.to_be_bytes(),
            &128_u32.to_be_bytes(),
            &8_u32.to_be_bytes(),
            &crate::ExecutionLimits::default().max_work.to_be_bytes(),
            &crate::SEMANTICS_VERSION.to_be_bytes(),
        ],
    )
}

fn token(prepared: &str, options: &str, position: u32, role: &str) -> String {
    let seal = crate::identity::framed(
        "htmlcut.discovery-token/1",
        &[
            prepared.as_bytes(),
            options.as_bytes(),
            &position.to_be_bytes(),
            role.as_bytes(),
        ],
    );
    crate::canonical_json(&Token {
        prepared: prepared.into(),
        options: options.into(),
        position,
        role: role.into(),
        seal,
    })
    .expect("discovery tokens contain only strings and an integer")
}

fn position(
    value: &str,
    prepared: &str,
    options: &str,
    role: &str,
) -> Result<u32, ExtractionError> {
    if value.len() > 1024 {
        return Err(stale());
    }
    let document = crate::parse_closed_json(value.as_bytes())?;
    let parsed: Token = serde_json::from_value(document).map_err(|_| stale())?;
    // Equality of the complete canonical token binds prepared/options/role and seal;
    // separate field comparisons would be redundant with this stronger comparison.
    if token(prepared, options, parsed.position, role) != crate::canonical_json(&parsed)? {
        return Err(stale());
    }
    Ok(parsed.position)
}

fn stale() -> ExtractionError {
    ExtractionError::new(
        ErrorCode::InvalidOptions,
        "discovery",
        "Discovery evidence is stale, altered or bound to different options.",
    )
}

impl PreparedDocument {
    /// Proposes a bounded positional path for an existing handle without executing a plan.
    pub fn propose(
        &self,
        handle: &str,
        page_size: u32,
    ) -> Result<SelectorProposal, ExtractionError> {
        if page_size == 0 || page_size > 100 {
            return Err(stale());
        }
        let ordinal = position(
            handle,
            self.prepared_sha256(),
            &binding(page_size),
            "handle",
        )?;
        let budget = SelectorWorkBudget::new(crate::ExecutionLimits::default().max_work);
        let document = self.document()?;
        let id = *self
            .element_ids()?
            .get(ordinal as usize)
            .ok_or_else(stale)?;
        crate::execution::charge(&budget, 1)?;
        let target = ElementRef::wrap(document.tree.get(id).unwrap()).unwrap();
        let mut components = Vec::new();
        let mut total = 0;
        for node in std::iter::once(*target).chain(target.ancestors()) {
            crate::execution::charge(&budget, 1)?;
            let Some(element) = ElementRef::wrap(node) else {
                continue;
            };
            if element.value().name().len() > 128 {
                return Err(ExtractionError::limit("proposal"));
            }
            let mut index = 1;
            for sibling in element.prev_siblings() {
                crate::execution::charge(&budget, 1)?;
                if sibling.value().is_element() {
                    index += 1;
                }
            }
            let mut component = String::new();
            cssparser::serialize_identifier(element.value().name(), &mut component)
                .expect("writing an identifier to String is infallible");
            component.push_str(&format!(":nth-child({index})"));
            total += component.len() + 3;
            if total > crate::limits::MAX_PATTERN_BYTES {
                return Err(ExtractionError::limit("proposal"));
            }
            components.push(component);
        }
        components.reverse();
        Ok(SelectorProposal {
            schema: "htmlcut.selector.proposal".into(),
            version: SCHEMA_VERSION,
            kind: "suggestion".into(),
            prepared_sha256: self.prepared_sha256().into(),
            handle: handle.into(),
            selector: components.join(" > "),
        })
    }
    /// Retrieves one bounded page; callers must reuse exactly the same snapshot/options.
    pub fn inspect(
        &self,
        page_size: u32,
        cursor: Option<&str>,
    ) -> Result<InspectionResult, ExtractionError> {
        if page_size == 0 || page_size > 100 {
            return Err(ExtractionError::new(
                ErrorCode::InvalidOptions,
                "discovery",
                "Inspection page size must be between one and one hundred.",
            ));
        }
        let options = binding(page_size);
        let start = cursor
            .map(|cursor| position(cursor, self.prepared_sha256(), &options, "cursor"))
            .transpose()?
            .unwrap_or(0);
        let budget = SelectorWorkBudget::new(crate::ExecutionLimits::default().max_work);
        let document = self.document()?;
        let mut elements = Vec::new();
        let ids = self.element_ids()?;
        if start as usize > ids.len() {
            return Err(stale());
        }
        let stop = (start as usize + page_size as usize).min(ids.len());
        let next_cursor = if stop < ids.len() {
            Some(token(
                self.prepared_sha256(),
                &options,
                stop as u32,
                "cursor",
            ))
        } else {
            None
        };
        for (ordinal, id) in ids.iter().enumerate().take(stop).skip(start as usize) {
            crate::execution::charge(&budget, 1)?;
            let element = ElementRef::wrap(document.tree.get(*id).unwrap()).unwrap();
            let (preview, complete) = text_preview(element, 64, &budget)?;
            if element.value().name().len() > 128 {
                return Err(ExtractionError::limit("discovery"));
            }
            let mut attributes = Vec::new();
            let mut attributes_complete = true;
            for (name, value) in element.value().attrs() {
                crate::execution::charge(&budget, 1)?;
                if attributes.len() == 8 {
                    attributes_complete = false;
                    break;
                }
                if name.len() > 128 {
                    return Err(ExtractionError::limit("discovery"));
                }
                let prefix: String = value.chars().take(64).collect();
                let complete = prefix.len() == value.len();
                attributes.push(AttributePreview {
                    name: name.into(),
                    value: prefix,
                    complete,
                });
            }
            elements.push(ElementDescriptor {
                handle: token(self.prepared_sha256(), &options, ordinal as u32, "handle"),
                tag: element.value().name().into(),
                attributes,
                attributes_complete,
                preview,
                complete,
            });
        }
        Ok(InspectionResult {
            schema: "htmlcut.inspection".into(),
            version: SCHEMA_VERSION,
            prepared_sha256: self.prepared_sha256().into(),
            elements,
            next_cursor,
        })
    }

    /// Executes the same projection path, then intentionally abbreviates this distinct preview.
    pub fn preview(
        &self,
        plan: &CompiledPlan,
        maximum_chars: u32,
    ) -> Result<PreviewResult, ExtractionError> {
        if maximum_chars == 0 || maximum_chars > 4096 {
            return Err(ExtractionError::new(
                ErrorCode::InvalidOptions,
                "preview",
                "Preview character allowance must be between one and 4096.",
            ));
        }
        #[cfg(test)]
        crate::projection::record_projection(4);
        let result = self.execute(plan)?;
        let mut remaining = maximum_chars as usize;
        let mut values = Vec::new();
        let mut complete = result.values.len() <= 20;
        for value in result.values.iter().take(20) {
            let prefix: String = value.chars().take(remaining).collect();
            remaining -= prefix.chars().count();
            if prefix.len() != value.len() {
                complete = false;
            }
            values.push(prefix);
        }
        Ok(PreviewResult {
            schema: "htmlcut.preview".into(),
            version: SCHEMA_VERSION,
            prepared_sha256: self.prepared_sha256().into(),
            plan_sha256: plan.plan_sha256().into(),
            candidate_count: result.candidate_count,
            selected_count: result.selected_count,
            values,
            complete,
        })
    }
}

fn text_preview(
    root: ElementRef<'_>,
    maximum: usize,
    budget: &SelectorWorkBudget,
) -> Result<(String, bool), ExtractionError> {
    let mut value = String::new();
    let mut chars = 0;
    for node in root.descendants() {
        crate::execution::charge(budget, 1)?;
        if let Node::Text(text) = node.value() {
            for c in text.text.chars() {
                if chars == maximum {
                    return Ok((value, false));
                }
                value.push(c);
                chars += 1;
            }
        }
    }
    Ok((value, true))
}

#[cfg(test)]
#[path = "tests/discovery_binding.rs"]
mod binding_tests;
