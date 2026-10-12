# Extraction-contract implementation and evidence

The implementation is based on published v21 source `4cd5752740b6d26aa63594fa3ea001d3290db71d`.
Rust was bumped to 1.99.0 before the redesign. The next major source version is 22;
plan/semantics version is 8. No public release or tag is authorized by this work.
The handoff's documents were treated as a proposal and evidence, not governing
instructions. The selected [design](../docs/architecture.md) and genuinely separate
[design QA](design-qa/README.md) preceded replacement; [implementation QA](design-qa/implementation-review.md)
challenged the actual tree and delivered binary.

## Numbered findings and resolutions relative to v21

1. Product scope is reusable fail-closed extraction of caller-owned snapshots. Browser acquisition, article inference and document conversion belong to other tools.
2. All five customized parser/selector packages are removed. The locked registry graph uses scraper's upstream DOM, public parser/types and unmodified selector matcher/cache; no product-owned HTML parser or selector implementation remains.
3. Native CLI execution moves to one private worker. Policy precedes parsing; simultaneous bounded pipes, active deadlines, complete framing, successful termination and kill/reap protect publication. macOS is explicitly a baseline-relative virtual-growth policy, not an RSS guarantee.
4. Markdown, resolved Markdown and inner HTML are removed, leaving text/literal/outer HTML/attributes/explicit-base URLs.
5. Heuristic unscoped survey and its schema/API are removed. Explicit-selector counts/samples and representative outer HTML remain.
6. Sibling-forest records are removed; fields return nodes from one anchor subtree while original context remains available to predicates.
7. Short tested inline and saved-plan examples are supplied. Independent scripted agent exercises succeeded without retries; these were not blind trials.
8. Business identity remains caller-owned. The reviewer demonstrated a valid wrong-business-identity success and documented the limitation.
9. Query/worker/encoding failures produce no public data. Existing destinations survive; a real eight-writer no-overwrite race has exactly one complete winner. Named input and redirected regular-file stdin aliases now refuse by physical identity on Unix/Windows, including plan stdin. Unknown pipe origins remain caller-owned.
10. Grammar compilation precedes source-path/destination validation, preserving source-independent selector/regex refusals; all file protections still run before reading source or publishing. Values remain lexical strings/null/arrays. No domain typing, transformation DSL or speculative API is added.
11. Actual published v21 bytes and final candidate pass all 11 supplied scenarios with their respective plans and full independent values/refusals. Strict references gained duplicate-field/missing-attribute controls and complete article-text validation. Historical pilot/benchmarks remain explicitly historical.
12. README publication claims now distinguish published v21 from this unpublished candidate. Legal notices derive from the locked native graph; published history/tags/assets remain intact.

## Contract changes

| Surface | Current behavior / required action |
|---|---|
| Plan | Version 8 only; rewrite version-7 plans and remove retired members/readings. Unknown/duplicate/null members refuse. |
| Rust | Retained immutable snapshot, compiled plan, owned data/counts; remove `max_parse_work`, survey, forest and conversion enum/API uses. |
| Resource assurance | DOM bounds are post-parse admission; traversal work is not all upstream instructions. Isolate adversarial direct-library calls externally. |
| CLI | `extract`, selector-required `inspect`, canonical named `schema`; existing regular-file admission and atomic publication, now supervised worker execution. |
| Physical policy | 1 GiB Linux virtual size, macOS virtual growth after bounded startup, Windows process committed memory; ten-second active-worker deadline excludes caller input wait. |
| Build/release | Stable Rust 1.99.0; nightly only for libFuzzer. Two project crates, no carrier publishing/Miri lanes. All four native package gates retained. |

## Executed local verification

On native macOS ARM, the isolated final `./check.sh` completed fmt, warnings-denied
workspace Clippy, Rust tests/doctests, the explicit million-element refusal, rustdoc,
37 Python maintenance tests and advisory/license/source checks. Its Rust commands
executed 228 passing tests in total; the one ordinarily ignored large test was
explicitly executed successfully. A prior concurrent check hit the active-worker
deadline while fuzzing/compilation competed with the large encoded-byte fixture;
the unchanged fixture reached its intended byte refusal in 2.62 seconds when
isolated, then the complete isolated gate passed. No test or resource guard was weakened.
Both required libFuzzer targets completed 1,000 runs with retained logs and no crash.
Windows target compilation also passed; that alone is not native enforcement proof.

All four Rust examples were executed. `cargo package --locked -p htmlcut-core`
verified the normalized archive against registry dependencies; an external consumer
of that archive compiled and checked complete record JSON with no workspace paths
or fork dependencies. Candidate official registry installation is not claimed:
publication requires separate authorization and core must be published before CLI.
The semver command against actual v21 succeeded under the major-version policy;
it skipped 254 compatibility checks, so it does not establish API compatibility.

The CLI suite defines thirteen native controls on each platform, including redirected source/plan stdin identity, grammar precedence and supervisor-death cleanup. Both supervisor-death controls consume the existing private worker workload acknowledgement before terminating the supervisor; Windows then verifies disappearance before the ordinary deadline; its native execution is a required package gate. Independent native tests demonstrated touched allocation refusal near 1 GiB,
blocked input and timeout termination near ten seconds, flood/crash/malformed-frame
refusals, supervisor lifetime protection, hardlink refusal and atomic publication.
The legitimate 50 MiB inert-script snapshot succeeds under the policy.
Exact code review, binary hashes, raw results and platform limitations are retained
under [design QA](design-qa/implementation-review.md).

## Comparisons and limits

[Current results](contract-matrix/results/comparison-summary.json) bind the final
release-profile executable SHA-256
`f952b2c67a991715ed7847c068704fe8e263b8f8adfc9cd25dbe4d5037a889c2`.
Every positive complete output matches published v21 byte-for-byte; every intended
refusal emits no data. Each run has fifteen internally randomized same-machine
trials. The two versions were run sequentially, not interleaved, and raw timings
are retained. Catalogue median launch was 5.25 ms for v21 versus 8.90 ms for the
worker candidate; the worst per-task median ratio was 2.74, within the pre-registered
sixfold threshold. This is a measured worker startup cost, not a speed improvement.

[Warm comparison](contract-matrix/results/warm-comparison.json) checks all twenty
titles independently and separates prepared DOM/compiled-query mapping from
parse+mapping. [Named tokenizer/task costs](contract-matrix/results/task-economics.json)
cover six exact tasks and separate source, plan/help and output sizes; they are not
billed reasoning or full blind-agent transcripts. Equal JSON output has equal
output token cost for the same tokenizer. Bytes are not tokens.

Offline Chromium executed the JavaScript fixture, and jq returned direct JSON.
[Trafilatura's automatic-reader trial](contract-matrix/results/reader.json) retained
navigation on this artificial repetitive page; its unfavorable full output is
recorded without a general reader-quality claim. Pandoc was not installed, and
htmlq/pup fresh trials, blind agents, market demand and billed-token accounting were
not executed. They are not represented as passing or as product dependencies.

## Work-package status and external rollout

W01–W05 and W07–W11 are implemented and locally verified as scoped above. W06 native
policy and W12 manifests/locks/licensing/source/native packaging/CI are implemented;
all four source-bound native package jobs must pass on the final PR head. Earlier
native gates passed on the production implementation, then artifact review identified
and added the Windows supervisor-death negative control. W13 independent design
and implementation QA is complete; review readiness requires that additional
control and all other required gates to pass on the delivered tree.
The PR head and source-bound native artifacts supply the authoritative commit,
changed-file inventory and platform results; local success cannot replace them.

The macOS 12 deployment floor was source-reviewed, not executed on macOS 12.
The candidate is not publicly released: official registry installs and anonymous
candidate published-download verification can occur only after separate publication
authorization. Existing release automation retains those checks. No tag, public
asset, registry package, main merge or remote protection setting is changed here.
