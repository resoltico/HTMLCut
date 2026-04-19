use std::collections::BTreeMap;
use std::num::NonZeroUsize;

use htmlcut_core::interop::v1::{
    DelimiterMode, ErrorCode, InteropError, InteropResult, Normalization, Output, OutputKind, Plan,
    PlanStrategy, RegexFlag, ResultExecution, ResultSource, SelectedMatch, SelectedMatchMetadata,
    Selection, SelectionMode, StrategyKind, TextWhitespace, stable_json_v1,
};
use htmlcut_core::{Diagnostic, DiagnosticLevel, SelectorQuery, SliceBoundary, result::Range};
use serde_json::{Map, Value};
use url::Url;

const CASES: usize = 256;

struct CaseGenerator {
    state: u64,
}

impl CaseGenerator {
    const fn new(seed: u64) -> Self {
        Self { state: seed }
    }

    fn next_u64(&mut self) -> u64 {
        self.state ^= self.state << 7;
        self.state ^= self.state >> 9;
        self.state ^= self.state << 8;
        self.state
    }

    fn next_bool(&mut self) -> bool {
        self.next_u64() & 1 == 0
    }

    fn bounded(&mut self, upper: usize) -> usize {
        if upper == 0 {
            0
        } else {
            (self.next_u64() as usize) % upper
        }
    }

    fn non_empty_text(&mut self, max_len: usize) -> String {
        let mut text = self.text(max_len, true);
        if text.is_empty() {
            text.push('x');
        }
        text
    }

    fn text(&mut self, max_len: usize, allow_empty: bool) -> String {
        const ALPHABET: &[u8] =
            b"abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789 _./:-";
        let min_len = usize::from(!allow_empty);
        let len = min_len + self.bounded(max_len.saturating_sub(min_len) + 1);
        (0..len)
            .map(|_| {
                let index = self.bounded(ALPHABET.len());
                ALPHABET[index] as char
            })
            .collect()
    }

    fn json_value(&mut self, depth: usize) -> Value {
        if depth == 0 {
            return self.json_leaf();
        }

        match self.bounded(6) {
            0 | 1 => self.json_leaf(),
            2 => Value::Array(
                (0..self.bounded(4))
                    .map(|_| self.json_value(depth - 1))
                    .collect(),
            ),
            _ => {
                let mut object = Map::new();
                for _ in 0..self.bounded(4) {
                    object.insert(self.non_empty_text(12), self.json_value(depth - 1));
                }
                Value::Object(object)
            }
        }
    }

    fn json_leaf(&mut self) -> Value {
        match self.bounded(4) {
            0 => Value::Null,
            1 => Value::Bool(self.next_bool()),
            2 => Value::Number(((self.next_u64() % 10_000) as i64 - 5_000).into()),
            _ => Value::String(self.text(24, true)),
        }
    }

    fn json_value_non_null(&mut self, depth: usize) -> Value {
        loop {
            let value = self.json_value(depth);
            if !value.is_null() {
                return value;
            }
        }
    }

    fn regex_flags(&mut self) -> Vec<RegexFlag> {
        let mut flags = Vec::new();
        if self.next_bool() {
            flags.push(RegexFlag::CaseInsensitive);
        }
        if self.next_bool() {
            flags.push(RegexFlag::MultiLine);
        }
        if self.next_bool() {
            flags.push(RegexFlag::DotMatchesNewLine);
        }
        if self.next_bool() {
            flags.push(RegexFlag::SwapGreed);
        }
        if self.next_bool() {
            flags.push(RegexFlag::IgnoreWhitespace);
        }
        flags
    }

    fn output_kind(&mut self) -> OutputKind {
        match self.bounded(3) {
            0 => OutputKind::Text,
            1 => OutputKind::InnerHtml,
            _ => OutputKind::OuterHtml,
        }
    }

    fn selection(&mut self) -> Selection {
        match self.bounded(3) {
            0 => Selection::single(),
            1 => Selection::first(),
            _ => Selection::nth(NonZeroUsize::new(self.bounded(4) + 1).expect("non-zero index")),
        }
    }

    fn selection_mode(&mut self) -> SelectionMode {
        match self.bounded(3) {
            0 => SelectionMode::Single,
            1 => SelectionMode::First,
            _ => SelectionMode::Nth,
        }
    }

    fn text_whitespace(&mut self) -> TextWhitespace {
        if self.next_bool() {
            TextWhitespace::Normalize
        } else {
            TextWhitespace::Preserve
        }
    }

    fn range(&mut self) -> Range {
        let start = self.bounded(64);
        Range {
            start,
            end: start + self.bounded(16),
        }
    }

    fn url(&mut self) -> Url {
        let segment = self.non_empty_text(8).replace(' ', "_");
        let leaf = self.non_empty_text(8).replace(' ', "_");
        Url::parse(&format!("https://example.com/{segment}/{leaf}.html"))
            .expect("generated URL should parse")
    }

    fn extensions(&mut self) -> Option<BTreeMap<String, Value>> {
        if !self.next_bool() {
            return None;
        }

        let mut extensions = BTreeMap::new();
        for _ in 0..self.bounded(4) {
            extensions.insert(self.non_empty_text(12), self.json_value(2));
        }
        Some(extensions)
    }

    fn plan(&mut self) -> Plan {
        let strategy = if self.next_bool() {
            PlanStrategy::css_selector(
                SelectorQuery::new(self.non_empty_text(16)).expect("non-empty selector"),
            )
        } else {
            let mode = if self.next_bool() {
                DelimiterMode::Regex
            } else {
                DelimiterMode::Literal
            };
            let flags = if matches!(mode, DelimiterMode::Regex) {
                self.regex_flags()
            } else {
                Vec::new()
            };
            PlanStrategy::delimiter_pair(
                SliceBoundary::new(self.non_empty_text(12)).expect("non-empty boundary"),
                SliceBoundary::new(self.non_empty_text(12)).expect("non-empty boundary"),
                mode,
                self.next_bool(),
                self.next_bool(),
                flags,
            )
        };

        let mut plan = Plan::new(
            strategy,
            self.selection(),
            Output::new(self.output_kind()),
            Normalization::new(self.text_whitespace(), self.next_bool()),
        );
        plan.extensions = self.extensions();
        plan
    }

    fn result_source(&mut self) -> ResultSource {
        ResultSource {
            input_base_url: self.next_bool().then(|| self.url()),
            effective_base_url: self.next_bool().then(|| self.url()),
            document_title: self.next_bool().then(|| self.text(20, true)),
        }
    }

    fn diagnostic(&mut self, allow_error: bool) -> Diagnostic {
        let level = if allow_error {
            match self.bounded(3) {
                0 => DiagnosticLevel::Info,
                1 => DiagnosticLevel::Warning,
                _ => DiagnosticLevel::Error,
            }
        } else if self.next_bool() {
            DiagnosticLevel::Info
        } else {
            DiagnosticLevel::Warning
        };

        Diagnostic {
            level,
            code: self.non_empty_text(16),
            message: self.text(24, true),
            details: self.next_bool().then(|| self.json_value_non_null(2)),
        }
    }

    fn diagnostics(&mut self, allow_error: bool) -> Vec<Diagnostic> {
        (0..self.bounded(4))
            .map(|_| self.diagnostic(allow_error))
            .collect()
    }

    fn selected_match(
        &mut self,
        strategy_kind: StrategyKind,
        candidate_count: usize,
    ) -> SelectedMatch {
        let candidate_index =
            NonZeroUsize::new(self.bounded(candidate_count) + 1).expect("non-zero index");
        let metadata = match strategy_kind {
            StrategyKind::CssSelector => SelectedMatchMetadata::CssSelector {
                candidate_count,
                candidate_index,
                path: self.text(24, true),
                tag_name: self.non_empty_text(10),
            },
            StrategyKind::DelimiterPair => SelectedMatchMetadata::DelimiterPair {
                candidate_count,
                candidate_index,
                selected_range: self.range(),
                inner_range: self.range(),
                outer_range: self.range(),
                include_start: self.next_bool(),
                include_end: self.next_bool(),
            },
        };

        SelectedMatch {
            candidate_index,
            value_kind: self.output_kind(),
            value: self.text(24, true),
            comparison_input_text: self.text(24, true),
            inner_html: self.next_bool().then(|| self.text(24, true)),
            outer_html: self.next_bool().then(|| self.text(24, true)),
            metadata,
        }
    }

    fn interop_result(&mut self) -> InteropResult {
        let strategy_kind = if self.next_bool() {
            StrategyKind::CssSelector
        } else {
            StrategyKind::DelimiterPair
        };
        let candidate_count = self.bounded(4) + 1;
        let execution = ResultExecution::new(
            "plan-digest",
            strategy_kind,
            self.selection_mode(),
            candidate_count,
        );
        let mut result = InteropResult::new(
            execution,
            self.result_source(),
            self.selected_match(strategy_kind, candidate_count),
            self.diagnostics(false),
        );
        result.extensions = self.extensions();
        result
    }

    fn interop_error(&mut self) -> InteropError {
        let error_code = match self.bounded(4) {
            0 => ErrorCode::PlanInvalid,
            1 => ErrorCode::NoMatch,
            2 => ErrorCode::AmbiguousMatch,
            _ => ErrorCode::InternalError,
        };
        let mut details = BTreeMap::new();
        for _ in 0..self.bounded(4) {
            details.insert(self.non_empty_text(12), self.json_value(2));
        }

        let mut error = InteropError::new(
            "plan-digest",
            error_code,
            self.text(24, true),
            self.next_bool().then(|| {
                if self.next_bool() {
                    StrategyKind::CssSelector
                } else {
                    StrategyKind::DelimiterPair
                }
            }),
            details,
            self.diagnostics(true),
        );
        error.extensions = self.extensions();
        error
    }
}

fn canonical_json_string(value: &Value) -> String {
    match value {
        Value::Null | Value::Bool(_) | Value::Number(_) | Value::String(_) => {
            serde_json::to_string(value).expect("primitive JSON should serialize")
        }
        Value::Array(items) => {
            let mut output = String::from("[");
            for (index, item) in items.iter().enumerate() {
                if index > 0 {
                    output.push(',');
                }
                output.push_str(&canonical_json_string(item));
            }
            output.push(']');
            output
        }
        Value::Object(entries) => {
            let mut sorted_entries = entries.iter().collect::<Vec<_>>();
            sorted_entries.sort_unstable_by_key(|(key, _)| *key);

            let mut output = String::from("{");
            for (index, (key, entry_value)) in sorted_entries.into_iter().enumerate() {
                if index > 0 {
                    output.push(',');
                }
                output.push_str(
                    &serde_json::to_string(key).expect("JSON object keys should serialize"),
                );
                output.push(':');
                output.push_str(&canonical_json_string(entry_value));
            }
            output.push('}');
            output
        }
    }
}

#[test]
fn stable_json_v1_matches_recursive_canonical_serializer() {
    let mut generator = CaseGenerator::new(0x5172_0a9d_b3cc_41e3);

    for _ in 0..CASES {
        let value = generator.json_value(3);
        let stable = stable_json_v1(&value).expect("stable JSON should serialize");

        assert_eq!(stable, canonical_json_string(&value));
    }
}

#[test]
fn plans_round_trip_through_stable_json_and_have_deterministic_digests() {
    let mut generator = CaseGenerator::new(0x7f9a_49d2_91e1_0023);

    for _ in 0..CASES {
        let plan = generator.plan();
        let digest_once = plan.digest_sha256().expect("plan digest");
        let digest_twice = plan.digest_sha256().expect("plan digest");
        let stable = plan.stable_json().expect("plan stable JSON");
        let round_trip: Plan = serde_json::from_str(&stable).expect("plan round trip");

        assert_eq!(digest_once, digest_twice);
        assert_eq!(round_trip, plan);
        assert_eq!(
            round_trip.digest_sha256().expect("round-trip plan digest"),
            digest_once
        );
    }
}

#[test]
fn results_round_trip_through_stable_json_and_ignore_existing_self_digest() {
    let mut generator = CaseGenerator::new(0xc508_0ccd_2202_845b);

    for _ in 0..CASES {
        let mut result = generator.interop_result();
        let digest_once = result.digest_sha256().expect("result digest");
        let digest_twice = result.digest_sha256().expect("result digest");
        result.result_digest_sha256 = "already-present".to_owned();
        let digest_with_existing = result
            .digest_sha256()
            .expect("result digest with existing field");
        let stable = result.stable_json().expect("result stable JSON");
        let round_trip: InteropResult = serde_json::from_str(&stable).expect("result round trip");

        assert_eq!(digest_once, digest_twice);
        assert_eq!(digest_with_existing, digest_once);
        assert_eq!(round_trip, result);
        assert_eq!(
            round_trip
                .digest_sha256()
                .expect("round-trip result digest"),
            digest_once
        );
    }
}

#[test]
fn errors_round_trip_through_stable_json_and_ignore_existing_self_digest() {
    let mut generator = CaseGenerator::new(0x41e3_f601_2aef_41aa);

    for _ in 0..CASES {
        let mut error = generator.interop_error();
        let digest_once = error.digest_sha256().expect("error digest");
        let digest_twice = error.digest_sha256().expect("error digest");
        error.error_digest_sha256 = "already-present".to_owned();
        let digest_with_existing = error
            .digest_sha256()
            .expect("error digest with existing field");
        let stable = error.stable_json().expect("error stable JSON");
        let round_trip: InteropError = serde_json::from_str(&stable).expect("error round trip");

        assert_eq!(digest_once, digest_twice);
        assert_eq!(digest_with_existing, digest_once);
        assert_eq!(round_trip, error);
        assert_eq!(
            round_trip.digest_sha256().expect("round-trip error digest"),
            digest_once
        );
    }
}
