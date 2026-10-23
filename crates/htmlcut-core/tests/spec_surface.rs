// SPDX-License-Identifier: MPL-2.0
use htmlcut_core::{CONTRACT_SUMMARY, Contract, Match, Reading};
fn schema() -> serde_json::Value {
    Contract::schema()
}
#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;
    fn words(line: &str) -> BTreeSet<String> {
        line.split_once(": ")
            .unwrap()
            .1
            .split_whitespace()
            .map(Into::into)
            .collect()
    }
    fn consistent(schema: &serde_json::Value, summary: &str, docs: &str) -> bool {
        let mut lines = summary.lines();
        let node = words(lines.next().unwrap());
        let predicate = words(lines.next().unwrap());
        let readings = words(lines.next().unwrap());
        let properties = |name: &str| {
            schema["$defs"][name]["properties"]
                .as_object()
                .unwrap()
                .keys()
                .cloned()
                .collect::<BTreeSet<_>>()
        };
        let documented = docs
            .split("| Member |")
            .nth(1)
            .unwrap()
            .lines()
            .skip(2)
            .take_while(|line| line.starts_with('|'))
            .filter(|l| l.starts_with("| `"))
            .flat_map(|l| {
                l.split('|')
                    .nth(1)
                    .unwrap()
                    .split('`')
                    .enumerate()
                    .filter(|(i, _)| i % 2 == 1)
                    .map(|(_, s)| s.to_owned())
                    .collect::<Vec<_>>()
            })
            .collect::<BTreeSet<_>>();
        let forms = [
            Reading::Text,
            Reading::TextContent,
            Reading::OuterHtml,
            Reading::Attr("NAME".into()),
            Reading::Url("NAME".into()),
        ]
        .into_iter()
        .map(|r| r.to_string())
        .collect::<BTreeSet<_>>();
        node == properties("NodeObject")
            && predicate == properties("PredicateObject")
            && node == documented
            && readings == forms
    }
    #[test]
    fn summary_schema_and_member_table_have_one_surface() {
        let schema = schema();
        let docs = include_str!("../../../docs/contract.md");
        assert!(consistent(&schema, CONTRACT_SUMMARY, docs));
        let mut fake = schema.clone();
        fake["$defs"]["NodeObject"]["properties"]["fake"] = true.into();
        assert!(!consistent(&fake, CONTRACT_SUMMARY, docs));
        assert!(!consistent(
            &schema,
            CONTRACT_SUMMARY,
            &docs.replacen("| `select`", "| `invented`", 1)
        ));
        assert!(!consistent(
            &schema,
            &CONTRACT_SUMMARY.replace("expect", "invented"),
            docs
        ));
    }
    #[test]
    fn reading_and_match_strings_round_trip() {
        for text in [
            "text",
            "text-content",
            "outer-html",
            "attr:NAME",
            "url:NAME",
        ] {
            let read: Reading = text.parse().unwrap();
            assert_eq!(read.to_string(), text);
            assert_eq!(
                serde_json::from_str::<Reading>(&serde_json::to_string(&read).unwrap()).unwrap(),
                read
            );
        }
        for text in ["one", "optional", "all"] {
            assert_eq!(text.parse::<Match>().unwrap().to_string(), text);
        }
        for text in ["attr:", "url:a b", "attr:a/b", "bad"] {
            assert!(text.parse::<Reading>().is_err());
        }
    }
}
