// SPDX-License-Identifier: MPL-2.0
//! Public consumer for ordinary parser refusals, also executed with panic=abort.
use htmlcut_core::{Contract, Document, Limits, Source, Value};
fn main() {
    let cases = [
        "<table><tr><td>a<td>b<tr><td>c</table>",
        "<p><b><i>x</b>y</i></p>",
        "<template><table><tr><td>x</table><template><b>y</b></template></template>",
        "<svg><foreignObject><p>x</p></foreignObject></svg><math><annotation-xml encoding='text/html'><p>y</p></annotation-xml></math>",
        "<!--c--><p>a&amp;b</p><frameset><frame></frameset>",
        "<meta http-equiv=content-type content='text/html; charset'><p>ok</p>",
        "",
    ];
    let mut refusals = 0;
    let mut admitted = 0;
    for html in cases {
        for maximum in 0..80 {
            let mut limits = Limits::default();
            limits.max_nodes = maximum;
            limits.time_limit = None;
            match Document::parse(
                Source::new(html.as_bytes()),
                &limits.preparation(),
                &htmlcut_core::ExecutionContext::new(limits.time_limit).unwrap(),
            ) {
                Err(error) => {
                    assert_eq!(
                        serde_json::to_value(&error).unwrap()["limit"].as_str(),
                        Some("nodes")
                    );
                    refusals += 1;
                }
                Ok(document) => {
                    document
                        .extract(
                            &Contract::from_json(
                                r#"{"select":":scope","read":"outer-html"}"#,
                                &htmlcut_core::ExecutionContext::new(None).unwrap(),
                            )
                            .unwrap(),
                            &limits.execution(),
                            &htmlcut_core::ExecutionContext::new((limits).time_limit).unwrap(),
                        )
                        .unwrap();
                    admitted += 1;
                }
            }
        }
    }
    let document = Document::parse(
        Source::new(b"<meta http-equiv=content-type content='text/html; charset'><p>ok</p>"),
        &Limits::default().preparation(),
        &htmlcut_core::ExecutionContext::new(Limits::default().time_limit).unwrap(),
    )
    .unwrap();
    assert_eq!(
        document
            .extract(
                &Contract::from_json(
                    r#""p""#,
                    &htmlcut_core::ExecutionContext::new(None).unwrap()
                )
                .unwrap(),
                &Limits::default().execution(),
                &htmlcut_core::ExecutionContext::new(None).unwrap()
            )
            .unwrap(),
        Value::String("ok".into())
    );
    println!(
        "admitted={admitted} refusals={refusals} cases={}",
        cases.len() * 80
    );
}
