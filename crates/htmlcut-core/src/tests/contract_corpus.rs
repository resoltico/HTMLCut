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
            .values,
        [literal]
    );
    let mut plan = plan;
    plan.projection = Projection::DocumentText;
    let structural = "## Free-threading\nThe [global interpreter lock](gil.html) affects Windows and macOS.\n```\n--disable-gil\nPYTHON_GIL\nsys.version\nPy_mod_gil\nPyUnstable_Module_SetGIL\n```\nHidden source note remains included.\nBroken mirror\n[table]\nCharges\n[header] Amount | [rowspan=2] EUR 180\n[/table]";
    assert_eq!(
        source
            .execute(&CompiledPlan::compile(&plan).unwrap())
            .unwrap()
            .values,
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
            .values,
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
            .values,
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
            .values,
        [literal]
    );
    plan.projection = Projection::DocumentText;
    assert_eq!(
        source
            .execute(&CompiledPlan::compile(&plan).unwrap())
            .unwrap()
            .values,
        ["First\nSecondoutside\n[table]\nA | B\n[/table]\nUnicode: é ✓ NBSP\u{a0} ZWSP\u{200b}"]
    );
    assert_eq!(source.parse_count(), 1);
}
