// SPDX-License-Identifier: MPL-2.0
//! Run admitted selectors at the consumer thread/process boundary where stack aborts occurred.
use htmlcut_core::{CompiledPlan, ErrorCode, ExtractionPlan, PreparedDocument, SourceSnapshot};
use serde_json::json;
use std::process::Command;

const CHILD_INPUT: &str = "HTMLCUT_SELECTOR_PROCESS_CONTROL";

fn nested_source(levels: usize) -> String {
    format!(
        "{}<span>X</span>{}",
        "<div>".repeat(levels),
        "</div>".repeat(levels)
    )
}

fn check_process(label: &str, source: String, selector: String, refused: bool) {
    check_process_values(label, source, selector, refused, &["X"]);
}

fn check_process_values(
    label: &str,
    source: String,
    selector: String,
    refused: bool,
    expected: &[&str],
) {
    check_process_input(
        label,
        json!({"source":source,"selector":selector,"refused":refused,"expected":expected}),
    );
}

fn check_process_input(label: &str, input: serde_json::Value) {
    let output = Command::new(std::env::current_exe().unwrap())
        .args(["--exact", "consumer_thread_control", "--nocapture"])
        .env(CHILD_INPUT, input.to_string())
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{label}: {}\nstdout: {}\nstderr: {}",
        output.status,
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("consumer control completed"),
        "{label}: child did not execute its assertions"
    );
}

#[test]
fn consumer_thread_control() {
    let Ok(input) = std::env::var(CHILD_INPUT) else {
        return;
    };
    let input: serde_json::Value = serde_json::from_str(&input).unwrap();
    std::thread::Builder::new()
        .name("selector-consumer".into())
        .stack_size(2 * 1024 * 1024)
        .spawn(move || {
            let source = input["source"].as_str().unwrap();
            let selector = input["selector"].as_str().unwrap();
            let snapshot = SourceSnapshot::new(source, Default::default()).unwrap();
            let document = PreparedDocument::new(snapshot, Default::default()).unwrap();
            if input["inspect"].as_bool().unwrap_or(false) {
                assert_refusal(document.inspect(selector, 1).unwrap_err(), source, selector);
                return;
            }
            let request = if let Some(query) = input.get("query") {
                ExtractionPlan::from_json(&serde_json::to_vec(query).unwrap()).unwrap()
            } else {
                let mut request = ExtractionPlan::css(selector).unwrap();
                request.match_mode = htmlcut_core::Match::All;
                request.min = Some(0);
                request
            };
            let plan = CompiledPlan::compile(&request);
            if input["refused"].as_bool().unwrap() {
                let error = plan.err().expect("unsafe selector was admitted");
                assert_refusal(error, source, selector);
            } else {
                let result = document.execute(&plan.unwrap()).unwrap();
                assert_eq!(
                    serde_json::to_value(result.data().as_values().unwrap()).unwrap(),
                    input["expected"]
                );
            }
        })
        .unwrap()
        .join()
        .unwrap();
    eprintln!("consumer control completed");
}

fn assert_refusal(error: htmlcut_core::ExtractionError, source: &str, selector: &str) {
    assert_eq!(error.code, ErrorCode::ResourceLimit);
    assert_eq!(error.stage, "compilation");
    assert_eq!(
        error.resource_counter.as_deref(),
        Some("selector_matching_depth")
    );
    assert_eq!(error.configured_bound, Some(64));
    let serialized = serde_json::to_string(&error).unwrap();
    assert!(!serialized.contains(selector));
    assert!(!serialized.contains(source));
    assert!(!serialized.contains("private-query-marker"));
    assert!(!serialized.contains("private-source-marker"));
}

#[test]
fn chain_admission_is_safe_on_a_normal_consumer_stack() {
    for (compounds, refused) in [
        (16, false),
        (63, false),
        (64, false),
        (65, true),
        (2001, true),
    ] {
        let ancestors = compounds - 1;
        check_process(
            &format!("descendant {compounds}"),
            nested_source(ancestors),
            format!("{}span", "* ".repeat(ancestors)),
            refused,
        );
    }
    for combinator in [">", "+", "~"] {
        for (compounds, refused) in [(63, false), (64, false), (65, true)] {
            let predecessors = compounds - 1;
            let source = if combinator == ">" {
                nested_source(predecessors)
            } else {
                format!("{}<span>X</span>", "<div></div>".repeat(predecessors))
            };
            check_process(
                &format!("{combinator} {compounds}"),
                source,
                format!("{}span", format!("div {combinator} ").repeat(predecessors)),
                refused,
            );
        }
    }
}

#[test]
fn longest_selector_list_branch_is_admitted_independently() {
    for (compounds, refused) in [(64, false), (65, true)] {
        let chain = format!("{}span", "* ".repeat(compounds - 1));
        for selector in [format!(".absent, {chain}"), format!("{chain}, .absent")] {
            check_process(
                "selector list longest branch",
                nested_source(compounds - 1),
                selector,
                refused,
            );
        }
    }
}

#[test]
fn logical_branches_add_to_the_active_combinator_chain() {
    // The pseudo-class belongs to the leftmost compound: the outer chain is still
    // active when the matcher enters its argument, unlike a rightmost pseudo-class.
    for pseudo in ["is", "where", "not"] {
        for (inner_compounds, refused) in [(31, false), (32, true)] {
            let target = if pseudo == "not" { ".absent" } else { "div" };
            let inner = format!("{}{}", "div > ".repeat(inner_compounds - 1), target);
            for branch in [
                inner.clone(),
                format!(".also-absent, {inner}"),
                format!("{inner}, .also-absent"),
            ] {
                let selector = format!(":{pseudo}({branch}){} > span", " > div".repeat(31));
                check_process(
                    &format!("additive {pseudo} {inner_compounds}"),
                    nested_source(64),
                    selector,
                    refused,
                );
            }
        }
    }
    // Multiple syntax levels, each individually modest, share the active path too.
    for pseudo in ["is", "where", "not"] {
        for (nesting, refused) in [(31, false), (32, true)] {
            let mut inner = if pseudo == "not" && nesting % 2 == 1 {
                ".absent".into()
            } else {
                "div".into()
            };
            for _ in 0..nesting {
                inner = format!(":{pseudo}({inner})");
            }
            let selector = format!("{inner}{} > span", " > div".repeat(31));
            check_process(
                &format!("combined chain and nesting {pseudo} {nesting}"),
                nested_source(64),
                selector,
                refused,
            );
        }
    }
}

#[test]
fn relational_arguments_share_matching_depth_and_preserve_exact_results() {
    for combinator in [" ", ">", "+", "~"] {
        for (relative_compounds, refused) in [(30, false), (31, true)] {
            let relative = format!(
                "{combinator} {}span",
                format!("div {combinator} ").repeat(relative_compounds - 1)
            );
            let outer_branch = format!(
                "{}<span>X</span>{}",
                "<div>".repeat(31),
                "</div>".repeat(31)
            );
            let source = if ["+", "~"].contains(&combinator) {
                format!(
                    "<div>{outer_branch}</div>{}<span></span>",
                    "<div></div>".repeat(relative_compounds - 1)
                )
            } else {
                let relative_branch = format!(
                    "{}<span></span>{}",
                    "<div>".repeat(relative_compounds - 1),
                    "</div>".repeat(relative_compounds - 1)
                );
                format!("<div>{relative_branch}{outer_branch}</div>")
            };
            for argument in [
                relative.clone(),
                format!(".absent, {relative}"),
                format!("{relative}, .absent"),
            ] {
                let selector = format!("div:has({argument}){} > span", " > div".repeat(31));
                check_process(
                    "relative branch plus active outer chain",
                    source.clone(),
                    selector,
                    refused,
                );
            }
        }
    }
}

#[test]
fn pseudo_depth_uses_the_active_path_and_accounts_for_functional_host() {
    // A rightmost logical branch returns before the long leftward chain executes.
    check_process(
        "rightmost pseudo does not add an inactive chain",
        nested_source(63),
        format!("{}:is(span)", "div > ".repeat(63)),
        false,
    );
    for (nesting, refused) in [(62, false), (63, false), (64, true)] {
        let mut selector = "span".to_owned();
        for _ in 0..nesting {
            selector = format!(":is({selector})");
        }
        check_process(
            "logical nesting boundary",
            "<span>X</span>".into(),
            selector,
            refused,
        );
    }
    let mut logical = "span".to_owned();
    let mut zero_specificity = "span".to_owned();
    for _ in 0..63 {
        logical = format!(":is({logical})");
        zero_specificity = format!(":where({zero_specificity})");
    }
    check_process(
        "sequential branches use maximum rather than sum",
        "<span>X</span>".into(),
        format!("{logical}{zero_specificity}"),
        false,
    );
    check_process(
        "fixed positional helper preserves a boundary chain",
        nested_source(63),
        format!("{}span:only-child", "div > ".repeat(63)),
        false,
    );
    check_process_values(
        "functional host remains a non-match in an HTML document",
        "<div>X</div>".into(),
        ":host(div)".into(),
        false,
        &[],
    );
    check_process_values(
        "functional host argument cannot bypass admission",
        nested_source(64),
        format!(":host({logical})"),
        true,
        &[],
    );
}

#[test]
fn every_public_selector_admission_route_refuses_the_same_depth() {
    let selector = format!("{}span.private-query-marker", "* ".repeat(64));
    let source = "<span class='private-query-marker'>private-source-marker</span>";
    for query in [
        json!({"version":7,"select":selector}),
        json!({"version":7,"select":"span","fields":{"value":{"select":selector}}}),
        json!({"version":7,"select":"span","fields":{"value":{"select":"span","exclude":[selector]}}}),
        json!({"version":7,"select":"span","exclude":[selector]}),
        json!({"version":7,"select":"span","expect":[{"select":selector}]}),
    ] {
        check_process_input(
            "JSON selector admission route",
            json!({"source":source,"selector":selector,"refused":true,"query":query}),
        );
    }
    check_process_input(
        "targeted inspection selector admission",
        json!({"source":source,"selector":selector,"inspect":true}),
    );
}

#[test]
fn parsed_components_ignore_punctuation_inside_attribute_values() {
    let value = "()[] > + ~ , :is(.absent)".repeat(70);
    let source = format!(
        "{}<span data-mark='{value}'>X</span>{}",
        "<div>".repeat(63),
        "</div>".repeat(63)
    );
    let selector = format!("{}span[data-mark='{value}']", "div > ".repeat(63));
    check_process(
        "attribute punctuation does not add parsed matching depth",
        source,
        selector,
        false,
    );
}
