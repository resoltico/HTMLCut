// SPDX-License-Identifier: MPL-2.0
use super::*;

#[test]
fn t33_offline_technical_corpus_has_full_literal_and_structural_oracles() {
    let html = include_str!("../../../../evaluation/corpus/technical.html");
    let source = prepared(html);
    let plan = ExtractionPlan::css("#policy").unwrap();
    let literal = "Free-threadingThe global interpreter lock affects Windows and macOS.--disable-gil\nPYTHON_GIL\nsys.version\nPy_mod_gil\nPyUnstable_Module_SetGILHidden source note remains included.ChargesAmountEUR 180non-document payload";
    assert_eq!(
        source
            .execute(&CompiledPlan::compile(&plan).unwrap())
            .unwrap()
            .data
            .as_values()
            .unwrap(),
        [literal]
    );
    let mut plan = plan;
    plan.projection = Projection::Value(ValueProjection::Markdown {});
    let structural = "## Free-threading\n\nThe [global interpreter lock](<gil.html>) affects Windows and macOS.\n\n```\n--disable-gil\nPYTHON_GIL\nsys.version\nPy_mod_gil\nPyUnstable_Module_SetGIL\n```\n\nHidden source note remains included.\n\nBroken mirror\n\nCharges\n\n-\n  - <strong>Amount</strong>\n  - EUR 180";
    assert_eq!(
        source
            .execute(&CompiledPlan::compile(&plan).unwrap())
            .unwrap()
            .data
            .as_values()
            .unwrap(),
        [structural]
    );
    let renamed = html
        .replace("id=\"policy\"", "id=\"renamed\"")
        .replace("class=\"reference internal\"", "class=\"unrelated\"");
    let mut other = plan;
    other.strategy = Strategy::Css {
        selector: "#renamed".into(),
    };
    assert_eq!(
        prepared(&renamed)
            .execute(&CompiledPlan::compile(&other).unwrap())
            .unwrap()
            .data
            .as_values()
            .unwrap(),
        [structural]
    );
}

#[test]
fn t34_supplied_rendered_dom_is_data_and_scripts_are_never_executed() {
    let html = include_str!("../../../../evaluation/corpus/rendered.html");
    let mut plan = ExtractionPlan::css(".quote").unwrap();
    plan.selection = Selection::All {
        min: 1,
        max: Some(2),
    };
    assert_eq!(
        prepared(html)
            .execute(&CompiledPlan::compile(&plan).unwrap())
            .unwrap()
            .data
            .as_values()
            .unwrap(),
        ["Externally created one", "Externally created two"]
    );
}

#[test]
fn t33_malformed_html_uses_html5_repairs_without_losing_literal_payload() {
    let html = include_str!("../../../../evaluation/corpus/malformed.html");
    let source = prepared(html);
    let mut plan = ExtractionPlan::css("main").unwrap();
    let literal = "FirstSecondoutsideABTMUnicode: é ✓ NBSP\u{a0} ZWSP\u{200b}";
    assert_eq!(
        source
            .execute(&CompiledPlan::compile(&plan).unwrap())
            .unwrap()
            .data
            .as_values()
            .unwrap(),
        [literal]
    );
    plan.projection = Projection::Value(ValueProjection::Markdown {});
    assert_eq!(
        source
            .execute(&CompiledPlan::compile(&plan).unwrap())
            .unwrap()
            .data
            .as_values()
            .unwrap(),
        ["First\n\nSecondoutside\n\n-\n  - A\n  - B\n\nUnicode: é ✓ NBSP\u{a0} ZWSP\u{200b}"]
    );
    assert_eq!(source.parse_count(), 1);
}

#[test]
fn t33_caption_classes_ids_and_unrelated_siblings_cannot_suppress_selected_content() {
    let base = "<article><p>BEGIN</p><table><caption>Charges</caption><tr><td>EUR 180</td></tr></table><p>END</p></article>";
    let variants = [
        base.to_string(),
        base.replace(
            "<caption>",
            "<caption id='policy' class='reference internal' hidden aria-hidden='true'>",
        ),
        format!("<aside id='policy'>unrelated</aside>{base}<footer>unrelated</footer>"),
    ];
    for html in variants {
        let source = prepared(&html);
        let mut plan = ExtractionPlan::css("article").unwrap();
        assert_eq!(
            source
                .execute(&CompiledPlan::compile(&plan).unwrap())
                .unwrap()
                .data
                .as_values()
                .unwrap(),
            ["BEGINChargesEUR 180END"]
        );
        plan.projection = Projection::Value(ValueProjection::Markdown {});
        assert_eq!(
            source
                .execute(&CompiledPlan::compile(&plan).unwrap())
                .unwrap()
                .data
                .as_values()
                .unwrap(),
            ["BEGIN\n\nCharges\n\n-\n  - EUR 180\n\nEND"]
        );
    }
}
