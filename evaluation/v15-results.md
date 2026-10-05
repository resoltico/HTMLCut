<!--
AFAD:
  afad: "4.0"
  version: "19.2.0"
  domain: EVALUATION
  updated: "2026-10-05"
RETRIEVAL_HINTS:
  keywords: [released binary, live audit, fidelity, bounded work, regression evidence]
  questions: ["What did the independent v15 release audit find?"]
-->

# Released HTMLCut v15.0.0 live audit — 1 October 2026

This evaluation was conducted on an isolated branch without modifying product source, main, tags or release assets. Its workflows and this report were subsequently integrated into main. Tested release commit: `a075c33fbc47b0edfaafad45fca70d6b70486d46`. Linux release tarball SHA-256: `361dbc0887a874f1bd1c1c669c063f3e64fa39acbe494be721b3d285a29145ff`.

## Viability judgment

v15 is a credible, useful snapshot-extraction component with clearer contracts and much smaller output than v14. It is not a replacement for browsers, domain mappers or already adequate parsers. The primary viability weakness is incomplete delivery of its distinguishing bounded-work and faithful-rendering guarantees. Focused correction is warranted, not another redesign or an extinction verdict. No commercial-demand or adoption study was performed.

## Actual execution scope

- 281 independent CLI contract cases: all met their expectations, including the six earlier fidelity controls/failures and 100 generated class/ID invariants.
- 47 loopback HTTP checks: 46 met expectations; unsolicited partial-content acceptance remained.
- 69 file/publication/replay/discovery checks: 63 met expectations; three FIFO liveness probes and three structural-rendering probes exposed the grouped findings below.
- 432 seeded malformed-input/selector robustness probes: 431 completed with structured success or expected-class failure within a two-second watchdog. One large positional selector exceeded it and was separately investigated.
- Total across those four local matrices: 829 checks/probes, not 829 independent websites or a pooled statistical reliability score.
- Actual released macOS ARM and Windows x64 binaries: 15 native assertions each, all passed. The first runner attempt hit GitHub metadata rate limits before testing those binaries; a corrected direct-download workflow passed.
- 12 public URL targets and 10 separately captured-file replays, plus 22 useful-work/discovery observations. Public failures were classified separately from product defects.
- Normal timing: three warmups and 30 randomized launches per native CLI. Regression timing: three randomized paired repetitions per version and sibling count.

## Confirmed findings

1. **Positional CSS performance regression and coarse work accounting.** Same valid source and CSS query, verified selected value `A`: at 10,000 siblings v14 median 0.01594 s versus v15 0.31351 s; at 20,000, 0.02843 versus 1.21282; at 40,000, 0.05361 versus 4.64304 (86.6x for that workload). v15's equivalent `--css p --match nth --index 40000` route took 0.0676 s in a separate run. At 100,000 siblings, CSS nth-child exceeded an eight-second watchdog; explicit positional selection took 0.134 s. A max_work=200000 plan still completed the 40,000-sibling CSS query in 4.65 s. `patches/rust/scraper/src/selector/budget.rs` creates a fresh selector cache per element; sibling loops in `patches/rust/selectors/matching.rs::nth_child_index` are not individually budgeted. Repair cache lifetime and traversal charging together. These are not universal speed ratios or a claimed OS hard-CPU-limit violation.
2. **Structural rendering regression.** `<dl id="x"><dt>Name</dt><dd>Alice</dd><dt>Amount</dt><dd>180</dd></dl>` produces `NameAliceAmount180` under explicit v15 `document_text`; v14 preserved term/value boundaries. Minified details/summary and address fixtures also concatenate distinct units. This is not a dom_text bug. Restore deterministic semantic boundaries, not reader heuristics.
3. **Unsolicited HTTP 206 accepted as ordinary success.** A loopback partial body with Content-Range yields exit 0 and values=["ok"] for a normal non-Range GET. The adapter accepts all 2xx without inspecting range metadata. Reject unsolicited partial representations instead of silently treating them as complete fetches.
4. **Nonregular-file liveness gap.** Source, plan and saved-run FIFO paths with no writer block until an external watchdog kills the process. Byte limits do not bound a blocking File::open. This is an unattended-operation hardening gap, distinct from intentionally streaming stdin or ordinary regular-file handling.
5. **Diagnostic ergonomics.** Multiple HTTP statuses and transport failures become the same generic acquisition error; invalid CLI options often lack the specific failing option/constraint. Redaction is valuable, but safe status/cause categories would improve recovery.

## Positive evidence and limits

The original v14 link-label, policy-section and caption/alt omissions are fixed in the tested cases. Full literal text from the historical Python subsection matched the independent Beautiful Soup result; all eight old missing technical terms survived document_text. The live Python endpoint returned 503 independently, so no fresh Python-page success is claimed.

Twenty book title attributes and thirty current Hacker News hrefs agreed across HTMLCut, Beautiful Soup, lxml and Selectolax. Twenty complete catalogue records were equal with direct Python mapping and HTMLCut outer HTML followed by that mapper. Ten JavaScript-created quote values agreed between actual offline Chromium, HTMLCut on the rendered DOM, and inline JSON extraction. Browser execution remains external.

Default compact title-result JSON measured 357 o200k_base tokens, compared with the historical v14 title-report measurement of 8,690. The same compact value array was 192; raw newline values were 189. A scalar `ok` was one raw token or 154 tokens in its v15 result envelope. Discovery index: 108; extract description: 134; root plus extract help: 585; five-element inspection: 1,081. These are tiktoken 0.14.0 encoding counts, not actual billed agent usage or a blind agent study.

For normal twenty-title extraction, 30-launch medians were HTMLCut 2.77 ms, htmlq 3.43 ms, pup 3.16 ms. Warm Python parse/select means: Beautiful Soup 16.18 ms, lxml 1.10 ms, Selectolax 0.78 ms. Bare Python startup was 759 ms, so fresh Python timings would confound parser comparisons. The Rust library in-process interface was not benchmarked.

Regular-file publication/collision, explicit overwrite, unprivileged permission failures, saved-run replay, URL-env non-persistence, and an eight-writer no-overwrite race passed. Conflicting inspection options were correctly rejected. HTTP tests exercised redirects, charset/BOM errors, gzip corruption/bombs, transfer limits, chunking, certificate rejection and ~15.24 s slow-response failures. No crash was observed in completed robustness probes; this is not a formal security proof, full HTML conformance audit or full cross-platform stress certification.

## Evidence locations

Full Markdown report, standard-library minimal reproducer, complete local scripts, source/capture hashes, finalized JSON results and timing trials are provided in the conversation's downloadable evidence archive. Full third-party article bodies, token vocabularies and native binaries are not redistributed there. Workflows: public capture run `36839444517`; completed native controls run `36839922064`. Artifacts have seven-day retention. First-pass oracle corrections and interrupted comparator timing work are disclosed in the report and are not product defects or substituted successful trials.
