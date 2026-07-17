use super::*;

#[test]
fn content_candidate_sampling_keeps_all_eligible_candidates_within_limit() {
    let document = parse_document_node(
        "<html><body>\
            <section class=\"content feature-one\"><h2>One</h2><p>Alpha beta gamma delta epsilon zeta eta theta iota kappa lambda mu nu xi omicron.</p></section>\
            <section class=\"content feature-two\"><h2>Two</h2><p>Alpha beta gamma delta epsilon zeta eta theta iota kappa lambda mu nu xi omicron.</p></section>\
            <section class=\"content feature-three\"><h2>Three</h2><p>Alpha beta gamma delta epsilon zeta eta theta iota kappa lambda mu nu xi omicron.</p></section>\
            <section class=\"content feature-four\"><h2>Four</h2><p>Alpha beta gamma delta epsilon zeta eta theta iota kappa lambda mu nu xi omicron.</p></section>\
            <section class=\"content feature-five\"><h2>Five</h2><p>Alpha beta gamma delta epsilon zeta eta theta iota kappa lambda mu nu xi omicron.</p></section>\
            <section class=\"content feature-six\"><h2>Six</h2><p>Alpha beta gamma delta epsilon zeta eta theta iota kappa lambda mu nu xi omicron.</p></section>\
        </body></html>",
    );

    assert_eq!(build_content_candidates(&document, 8).len(), 6);
}
