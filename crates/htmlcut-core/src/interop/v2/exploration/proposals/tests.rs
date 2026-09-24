use super::*;

fn main_element(document: &scraper::Html) -> ElementRef<'_> {
    let selector = Selector::parse("main").expect("HTMLCut-owned selector");
    document.select(&selector).next().expect("main element")
}

fn non_zero_proposal_count(value: u32) -> NonZeroU32 {
    NonZeroU32::new(value).expect("test proposal count is non-zero")
}

#[test]
fn proposal_count_stops_at_the_exact_requested_limit() {
    let document = scraper::Html::parse_document(
        "<main id=\"anchor\" aria-label=\"Primary content\">One</main>",
    );
    let element = main_element(&document);
    let candidates = proposal_candidates(&element);
    assert!(
        candidates.len() >= 2,
        "fixture must supply multiple unique candidates"
    );

    let (proposals, work_truncated) = selector_proposals(
        &document,
        &element,
        non_zero_proposal_count(1),
        &SelectorWorkBudget::new(100_000),
    );

    assert!(!work_truncated);
    assert_eq!(proposals.len(), 1);
}

#[test]
fn selector_proposals_admit_an_identifier_at_the_exact_public_byte_limit() {
    let identifier = "a".repeat(MAX_SELECTOR_BYTES - 1);
    let expected_selector = format!("#{identifier}");
    let document = scraper::Html::parse_document(&format!("<main id=\"{identifier}\">One</main>"));
    let element = main_element(&document);
    let (proposals, work_truncated) = selector_proposals(
        &document,
        &element,
        non_zero_proposal_count(1),
        &SelectorWorkBudget::new(100_000),
    );

    assert!(!work_truncated);
    assert_eq!(expected_selector.len(), MAX_SELECTOR_BYTES);
    assert_eq!(proposals.len(), 1);
    assert_eq!(proposals[0].selector.as_str(), expected_selector);
}

#[test]
fn selector_proposals_skip_an_identifier_that_exceeds_the_public_byte_limit() {
    let identifier = "a".repeat(MAX_SELECTOR_BYTES);
    let document = scraper::Html::parse_document(&format!("<main id=\"{identifier}\">One</main>"));
    let element = main_element(&document);
    let (proposals, work_truncated) = selector_proposals(
        &document,
        &element,
        non_zero_proposal_count(1),
        &SelectorWorkBudget::new(100_000),
    );

    assert!(!work_truncated);
    assert!(
        proposals
            .iter()
            .all(|proposal| proposal.selector.as_str().len() <= MAX_SELECTOR_BYTES)
    );
    assert!(
        proposals
            .iter()
            .all(|proposal| proposal.selector.as_str() != format!("#{identifier}"))
    );
}

#[test]
fn structural_fallback_handles_a_non_unique_svg_parser_projection() {
    let document = scraper::Html::parse_document(
        "<svg><linearGradient></linearGradient><linearGradient></linearGradient></svg>",
    );
    let element = document
        .select(&Selector::parse("linearGradient").expect("selector"))
        .next()
        .expect("gradient");
    let (proposals, work_truncated) = selector_proposals(
        &document,
        &element,
        non_zero_proposal_count(1),
        &SelectorWorkBudget::new(100_000),
    );
    assert!(!work_truncated);
    assert!(
        proposals
            .iter()
            .all(|proposal| proposal.match_count.get() == 1)
    );
}

#[test]
fn structural_fallback_reports_shared_work_exhaustion_without_a_partial_selector() {
    let document = scraper::Html::parse_document("<main>One</main>");
    let element = main_element(&document);
    let (proposals, work_truncated) = selector_proposals(
        &document,
        &element,
        non_zero_proposal_count(1),
        &SelectorWorkBudget::new(1),
    );
    assert!(proposals.is_empty());
    assert!(work_truncated);
}

#[test]
fn structural_fallback_discards_a_non_unique_proof_without_claiming_a_selector() {
    let document = scraper::Html::parse_document("<main>One</main>");
    let element = main_element(&document);
    let (proposals, work_truncated) = selector_proposals_with(
        &document,
        &element,
        non_zero_proposal_count(1),
        &SelectorWorkBudget::new(100_000),
        |_, _, _, _| UniqueProof::NotUnique,
        |_, _, _, _| UniqueProof::NotUnique,
    );
    assert!(proposals.is_empty());
    assert!(!work_truncated);
}

#[test]
fn structural_fallback_omits_an_oversized_selector_without_truncating_it() {
    let document = scraper::Html::parse_document(&format!(
        "{}leaf{}",
        "<div>".repeat(300),
        "</div>".repeat(300)
    ));
    let element = document
        .tree
        .root()
        .descendants()
        .filter_map(ElementRef::wrap)
        .last()
        .expect("deep element");
    let (proposals, work_truncated) = selector_proposals(
        &document,
        &element,
        non_zero_proposal_count(1),
        &SelectorWorkBudget::new(5_000_000),
    );
    assert!(proposals.is_empty());
    assert!(!work_truncated);
}

#[test]
fn data_attribute_candidates_exclude_non_data_attributes_and_generated_values() {
    let document = scraper::Html::parse_document(
        "<main title=\"ordinary-title\" data-generated=\"css-991827364551\" data-topic=\"story\">One</main>",
    );
    let element = main_element(&document);
    let candidates = proposal_candidates(&element);

    assert!(
        candidates
            .iter()
            .any(|candidate| candidate.selector.contains("data-topic"))
    );
    assert!(
        candidates
            .iter()
            .all(|candidate| !candidate.selector.contains("title="))
    );
    assert!(
        candidates
            .iter()
            .all(|candidate| !candidate.selector.contains("data-generated"))
    );
}

#[test]
fn generated_token_heuristic_rejects_a_long_ordinary_value() {
    // Length alone must not downgrade a stable, human-readable token. This makes the
    // conjunction in `generated_looking` load-bearing rather than incidental.
    assert!(!generated_looking("long-stable-label"));
}

#[test]
fn generated_token_heuristic_recognizes_an_all_hexadecimal_value() {
    // This has no ASCII digits. It is generated-looking solely because every character is
    // hexadecimal, exercising that alternative independently of the digit-count branch.
    assert!(generated_looking("abcdefabcdefabcd"));
}

#[test]
fn generated_token_heuristic_recognizes_a_digit_heavy_non_hexadecimal_value() {
    // A generated-looking token need not be hexadecimal. This proves the digit-count arm
    // independently, including the `||` short-circuit before the hexadecimal comparison.
    assert!(generated_looking("prefix-123456-suffix"));
}

#[test]
fn generated_token_heuristic_accepts_overlapping_positive_signals() {
    // A generated token may be both digit-heavy and all hexadecimal; these are alternative
    // sufficient signals, not mutually exclusive classifications.
    assert!(generated_looking("abcdef123456abcd"));
}

#[test]
fn generated_token_heuristic_keeps_short_values_outside_the_heuristic() {
    // The length floor prevents small human identifiers from being classified solely by
    // their character composition.
    assert!(!generated_looking("abc123"));
}

#[test]
fn short_identifier_values_remain_available_as_selector_candidates() {
    // Exercise the length floor through the production candidate builder: a direct unit
    // call can be optimized independently of the parser-driven path that users exercise.
    let document = scraper::Html::parse_document("<main data-short-key=\"abc123\">One</main>");
    let element = main_element(&document);

    let candidates = proposal_candidates(&element);

    assert!(
        candidates
            .iter()
            .any(|candidate| candidate.selector.contains("data-short-key")),
        "a short data value must not be misclassified as generated"
    );
}

#[test]
fn structural_proof_failure_reports_work_truncation_without_a_partial_proposal() {
    let document = scraper::Html::parse_document("<main>One</main>");
    let main = main_element(&document);
    let (proposals, truncated) = selector_proposals_with(
        &document,
        &main,
        non_zero_proposal_count(1),
        &SelectorWorkBudget::new(100_000),
        |_, _, _, _| UniqueProof::NotUnique,
        |_, _, _, _| UniqueProof::WorkTruncated,
    );
    assert!(proposals.is_empty());
    assert!(truncated);
}

#[test]
fn structural_fast_proof_charges_ancestry_and_selector_matching() {
    let document = scraper::Html::parse_document("<main>One</main>");
    let main = main_element(&document);
    let selector = Selector::parse("html > body > main").expect("selector");
    assert!(matches!(
        structural_uniquely_selects(&document, &selector, main.id(), &SelectorWorkBudget::new(1)),
        UniqueProof::WorkTruncated
    ));
    let ancestor_count = document
        .tree
        .get(main.id())
        .expect("main node")
        .ancestors()
        .filter_map(ElementRef::wrap)
        .count() as u32;
    assert!(matches!(
        structural_uniquely_selects(
            &document,
            &selector,
            main.id(),
            &SelectorWorkBudget::new(ancestor_count)
        ),
        UniqueProof::WorkTruncated
    ));
    assert!(matches!(
        structural_uniquely_selects(
            &document,
            &Selector::parse("aside").expect("selector"),
            main.id(),
            &SelectorWorkBudget::new(100_000)
        ),
        UniqueProof::NotUnique
    ));
}

#[test]
fn html_structural_proof_succeeds_with_less_work_than_a_document_scan() {
    let source = format!("<main>One</main>{}", "<aside>Other</aside>".repeat(300));
    let document = scraper::Html::parse_document(&source);
    let main = main_element(&document);
    let selector = Selector::parse("html > body > main:nth-of-type(1)").expect("selector");

    assert!(matches!(
        structural_uniquely_selects(
            &document,
            &selector,
            main.id(),
            &SelectorWorkBudget::new(100)
        ),
        UniqueProof::Proved
    ));
    assert!(matches!(
        uniquely_selects(
            &document,
            &selector,
            main.id(),
            &SelectorWorkBudget::new(100)
        ),
        UniqueProof::WorkTruncated
    ));
}

#[test]
fn html_descendant_of_foreign_ancestry_keeps_full_selector_proof() {
    let document =
        scraper::Html::parse_document("<svg><foreignObject><main>One</main></foreignObject></svg>");
    let main = main_element(&document);
    let main_namespace: &str = main.value().name.ns.as_ref();
    assert_eq!(main_namespace, "http://www.w3.org/1999/xhtml");
    assert!(
        main.ancestors()
            .filter_map(ElementRef::wrap)
            .any(|ancestor| {
                let namespace: &str = ancestor.value().name.ns.as_ref();
                namespace == "http://www.w3.org/2000/svg"
            })
    );
    let selector = Selector::parse("main").expect("selector");
    assert!(matches!(
        structural_uniquely_selects(
            &document,
            &selector,
            main.id(),
            &SelectorWorkBudget::new(100_000)
        ),
        UniqueProof::Proved
    ));
}
