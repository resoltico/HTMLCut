//! Atomic browser-target resolution against a prepared snapshot.

use scraper::ElementRef;
use selectors::work_budget::SelectorWorkBudget;
use sha2::{Digest, Sha256};

use crate::interop::v2::PreparedDocument;

use super::{
    DOM_PATH_PROFILE, ElementTargetHint, ExplorationAttribute, ExplorationError,
    ExplorationErrorCode, MAX_TARGET_TEXT_BYTES, PathBuildFailure,
    TARGET_RESOLUTION_RESULT_SCHEMA_NAME, TARGET_RESOLUTION_RESULT_SCHEMA_VERSION,
    TargetResolutionOptions, TargetResolutionResult, discovery_identity, element_name,
    exploration_error, inspect_element, namespaced_path, selector_proposals,
};

/// Returns the browser-target text fingerprint defined by the v2 Published Language.
pub fn normalized_dom_text_digest(text: &str) -> String {
    let normalized = text.replace("\r\n", "\n").replace('\r', "\n");
    let mut hasher = Sha256::new();
    hasher.update(b"htmlcut.dom_text.v1\0");
    hasher.update(normalized.as_bytes());
    hex_digest(hasher.finalize())
}

/// Atomically resolves a complete target hint and proves proposals against one prepared snapshot.
pub fn resolve_target_and_propose(
    document: &PreparedDocument,
    hint: &ElementTargetHint,
    options: &TargetResolutionOptions,
) -> Result<TargetResolutionResult, Box<ExplorationError>> {
    let discovery_identity = discovery_identity(document);
    if options.validate().is_err() {
        return Err(Box::new(exploration_error(
            ExplorationErrorCode::InvalidLimits,
            discovery_identity,
            "Target-resolution limits are outside HTMLCut's supported range.",
        )));
    }
    if hint.path_profile != DOM_PATH_PROFILE
        || !is_lower_sha256(&hint.normalized_text_digest_sha256)
    {
        return Err(Box::new(exploration_error(
            ExplorationErrorCode::TargetNotFound,
            discovery_identity,
            "Target hint does not use HTMLCut's current path and text-fingerprint contract.",
        )));
    }

    let budget = SelectorWorkBudget::new(options.max_work_units.get());
    for ordinal in 1..=document.element_count() {
        let element = document.element_at_ordinal(
            std::num::NonZeroU32::new(ordinal).expect("prepared element ordinals are one-based"),
        );
        if inspect_element(&budget).is_err() {
            return Err(Box::new(exploration_error(
                ExplorationErrorCode::TargetResolutionLimitExceeded,
                discovery_identity,
                "Target resolution exceeded its configured work budget.",
            )));
        }
        let Ok(name) = element_name(&element) else {
            continue;
        };
        if name != hint.element_name {
            continue;
        }
        let path = match namespaced_path(&element, &budget) {
            Ok(path) => path,
            Err(PathBuildFailure::Omission(_)) => continue,
            Err(PathBuildFailure::WorkLimit) => {
                return Err(Box::new(exploration_error(
                    ExplorationErrorCode::TargetResolutionLimitExceeded,
                    discovery_identity,
                    "Target resolution exceeded its configured work budget.",
                )));
            }
        };
        if path != hint.path {
            continue;
        }
        if !hint_attributes_match(&element, &hint.semantic_attributes) {
            continue;
        }
        let text = match bounded_normalized_dom_text(document.document(), &element, &budget) {
            Ok(text) => text,
            Err(TargetTextFailure::LengthLimit) => {
                return Err(Box::new(exploration_error(
                    ExplorationErrorCode::TargetFingerprintLimitExceeded,
                    discovery_identity.clone(),
                    "Target descendant text exceeds the fingerprint byte limit.",
                )));
            }
            Err(TargetTextFailure::WorkLimit) => {
                return Err(Box::new(exploration_error(
                    ExplorationErrorCode::TargetResolutionLimitExceeded,
                    discovery_identity,
                    "Target resolution exceeded its configured work budget.",
                )));
            }
        };
        if normalized_dom_text_digest(&text) != hint.normalized_text_digest_sha256 {
            continue;
        }
        // A complete `namespaced_path` is injective: every segment includes its namespace,
        // local name, and same-name sibling ordinal. Once all target facts agree, this is the
        // only possible target in the prepared snapshot, so continuing the walk cannot discover
        // an alternative and would only spend caller work budget.
        let (selector_proposals, work_truncated) = selector_proposals(
            document.document(),
            &element,
            options.max_proposals,
            &budget,
        );
        if work_truncated {
            return Err(Box::new(exploration_error(
                ExplorationErrorCode::TargetResolutionLimitExceeded,
                discovery_identity,
                "Target proposal proof exceeded its configured work budget.",
            )));
        }
        return Ok(TargetResolutionResult {
            schema_name: TARGET_RESOLUTION_RESULT_SCHEMA_NAME.to_owned(),
            schema_version: TARGET_RESOLUTION_RESULT_SCHEMA_VERSION,
            discovery_identity_sha256: discovery_identity,
            path: hint.path.clone(),
            selector_proposals,
        });
    }
    Err(Box::new(exploration_error(
        ExplorationErrorCode::TargetNotFound,
        discovery_identity,
        "Target hint does not identify an element in this prepared document.",
    )))
}

#[derive(Debug, PartialEq, Eq)]
enum TargetTextFailure {
    LengthLimit,
    WorkLimit,
}

fn bounded_normalized_dom_text(
    document: &scraper::Html,
    element: &ElementRef<'_>,
    budget: &SelectorWorkBudget,
) -> Result<String, TargetTextFailure> {
    let node = document
        .tree
        .get(element.id())
        .expect("prepared target element belongs to its prepared document");
    let mut normalized = String::new();
    let mut pending_carriage_return = false;
    for descendant in node.descendants() {
        if ElementRef::wrap(descendant).is_some() {
            inspect_element(budget).map_err(|_| TargetTextFailure::WorkLimit)?;
        }
        let scraper::Node::Text(text) = descendant.value() else {
            continue;
        };
        append_normalized_target_text(&mut normalized, &mut pending_carriage_return, text)?;
    }
    finish_normalized_target_text(&mut normalized, pending_carriage_return)?;
    Ok(normalized)
}

fn append_normalized_target_text(
    normalized: &mut String,
    pending_carriage_return: &mut bool,
    text: &str,
) -> Result<(), TargetTextFailure> {
    for character in text.chars() {
        if *pending_carriage_return {
            if character == '\n' {
                push_target_text_character(normalized, '\n')
                    .then_some(())
                    .ok_or(TargetTextFailure::LengthLimit)?;
                *pending_carriage_return = false;
                continue;
            }
            push_target_text_character(normalized, '\n')
                .then_some(())
                .ok_or(TargetTextFailure::LengthLimit)?;
            *pending_carriage_return = false;
        }
        if character == '\r' {
            *pending_carriage_return = true;
        } else {
            push_target_text_character(normalized, character)
                .then_some(())
                .ok_or(TargetTextFailure::LengthLimit)?;
        }
    }
    Ok(())
}

fn finish_normalized_target_text(
    normalized: &mut String,
    pending_carriage_return: bool,
) -> Result<(), TargetTextFailure> {
    if pending_carriage_return {
        push_target_text_character(normalized, '\n')
            .then_some(())
            .ok_or(TargetTextFailure::LengthLimit)?;
    }
    Ok(())
}

#[cfg(test)]
fn normalized_dom_text_nodes<I, T>(texts: I) -> Option<String>
where
    I: IntoIterator<Item = T>,
    T: AsRef<str>,
{
    let mut normalized = String::new();
    let mut pending_carriage_return = false;
    for text in texts {
        for character in text.as_ref().chars() {
            if pending_carriage_return {
                if character == '\n' {
                    if !push_target_text_character(&mut normalized, '\n') {
                        return None;
                    }
                    pending_carriage_return = false;
                    continue;
                }
                if !push_target_text_character(&mut normalized, '\n') {
                    return None;
                }
                pending_carriage_return = false;
            }
            if character == '\r' {
                pending_carriage_return = true;
            } else if !push_target_text_character(&mut normalized, character) {
                return None;
            }
        }
    }
    if pending_carriage_return && !push_target_text_character(&mut normalized, '\n') {
        return None;
    }
    Some(normalized)
}

fn push_target_text_character(normalized: &mut String, character: char) -> bool {
    if normalized
        .len()
        .checked_add(character.len_utf8())
        .is_none_or(|length| length > MAX_TARGET_TEXT_BYTES)
    {
        return false;
    }
    normalized.push(character);
    true
}

#[cfg(test)]
pub(crate) fn normalized_dom_text_nodes_for_tests<I, T>(texts: I) -> Option<String>
where
    I: IntoIterator<Item = T>,
    T: AsRef<str>,
{
    normalized_dom_text_nodes(texts)
}

fn hint_attributes_match(element: &ElementRef<'_>, attributes: &[ExplorationAttribute]) -> bool {
    attributes.iter().all(|attribute| {
        !attribute.value_truncated
            && element
                .value()
                .attr(&attribute.name)
                .is_some_and(|value| value == attribute.value)
    })
}

pub(super) fn is_lower_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || matches!(byte, b'a'..=b'f'))
}

#[cfg(test)]
pub(crate) fn is_lower_sha256_for_tests(value: &str) -> bool {
    is_lower_sha256(value)
}

fn hex_digest(digest: impl AsRef<[u8]>) -> String {
    digest
        .as_ref()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use scraper::Selector;

    #[test]
    fn bounded_target_text_uses_the_same_crlf_normalization_for_prepared_dom_text() {
        let document = scraper::Html::parse_document("<main>one\r\ntwo\rthree</main>");
        let element = document
            .select(&Selector::parse("main").expect("selector"))
            .next()
            .expect("main");
        assert_eq!(
            bounded_normalized_dom_text(&document, &element, &SelectorWorkBudget::new(64)),
            Ok("one\ntwo\nthree".to_owned())
        );

        let trailing = scraper::Html::parse_document("<main>tail\r</main>");
        let element = trailing
            .select(&Selector::parse("main").expect("selector"))
            .next()
            .expect("main");
        assert_eq!(
            bounded_normalized_dom_text(&trailing, &element, &SelectorWorkBudget::new(64)),
            Ok("tail\n".to_owned())
        );
    }

    #[test]
    fn target_text_state_machine_handles_split_crlf_lone_cr_and_trailing_cr() {
        let mut normalized = String::new();
        let mut pending = false;
        append_normalized_target_text(&mut normalized, &mut pending, "one\r")
            .expect("first fragment");
        append_normalized_target_text(&mut normalized, &mut pending, "\ntwo\rthree\r")
            .expect("second fragment");
        finish_normalized_target_text(&mut normalized, pending).expect("finish fragment");
        assert_eq!(normalized, "one\ntwo\nthree\n");
    }

    #[test]
    fn bounded_target_text_reports_work_exhaustion_before_scanning_a_descendant() {
        let document = scraper::Html::parse_document("<main><span>one</span></main>");
        let element = document
            .select(&Selector::parse("main").expect("selector"))
            .next()
            .expect("main");
        assert!(matches!(
            bounded_normalized_dom_text(&document, &element, &SelectorWorkBudget::new(1)),
            Err(TargetTextFailure::WorkLimit)
        ));
    }
}
