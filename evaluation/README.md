# HTMLCut comparative evaluation — 24 September 2026

This branch contains evaluation infrastructure and observations, not a product implementation change. Main and release artifacts were not modified by the evaluation. The tested release was **14.0.0**, commit `a606a959ebab0b6397b47e704967ec5db549d0e3`.

## Decision

HTMLCut has a useful future as a focused, reusable extraction CLI/core. Basic CSS selection and token reduction are not unique advantages over existing tools or small generated scripts. **The tested default text renderer is not sufficiently faithful to recommend as a general-purpose reader.** Correctness should take priority over adding another browser/crawling layer.

## Reproducible fidelity failures

Run `python evaluation/check_fidelity.py --htmlcut /path/to/htmlcut`. The check deliberately exits nonzero when selected meaningful content is lost. Version 14.0.0 passed three controls and failed three fidelity cases:

- Adding `class="reference internal"` to an ordinary link removed its label from selected paragraph text.
- An ordinary nested section with `id="policy"` caused its content and following article content to disappear; changing just the ID to a neutral value preserved them.
- Adding an unrelated table caption in the same article suppressed an image's meaningful alternative text. An ordinary image-only control passed; this is not a claim that all image labels are dropped.

All six extraction processes themselves exited successfully. Supplying a base URL did not cure the omissions. These are content-fidelity checks, not claims that every extraction mode fails. Attribute and outer-HTML extraction worked on the tested payloads. Final QA narrowed the image case to its caption-dependent condition rather than treating the initially simplified image case as a failure.

On the captured official Python 3.13 free-threading section, eight checked technical terms were absent from HTMLCut text and present in htmlq, pup, Beautiful Soup, lxml, and Selectolax output. HTMLCut outer-HTML followed by Pandoc preserved all eight. The relevant policies are in `crates/htmlcut-core/src/document/text/policy.rs`, `vocabulary.rs`, and `render/media.rs` at the tested tag.

## Measured comparisons

Four real public HTML pages, two scraping demonstration pages, a GitHub release JSON response, and controlled fixtures were used. Captured bytes were shared across methods. This was an exploratory single-operator evaluation, not a blinded agent A/B trial or a representative web benchmark.

- Thirty Hacker News story URLs and twenty complete book-demo title attributes were equivalent across six extractors after accounting for blank-line framing and pup escaping.
- Twenty domain records were identical from direct Beautiful Soup mapping and HTMLCut outer-HTML followed by that mapper. HTMLCut did not remove the domain-mapping step.
- Raw CSS did not find the ten JavaScript-created quote elements. Real offline Chromium rendering followed by HTMLCut did. The same ten records could also be obtained from inline JSON without rendering.
- Exactly-one match enforcement, missing-target/attribute failures, source-size limits, request replay, and stale target-evidence rejection worked in the tested controls. Default `first` selection is not a strict uniqueness check.
- Changed surrounding meaning with unchanged selected text was not detected; application-level validation remains necessary.
- Local HTTP charset decoding, HEAD-first, GET-only, and HEAD-405 fallback worked. Saved request definitions rejected a URL query string before acquisition.

### Token proxies

Using tiktoken 0.14.0 / o200k_base, not actual model billing:

| Representation | Tokens |
|---|---:|
| Raw Hacker News HTML / selected URLs | 11,739 / 573 |
| Raw book HTML / selected full titles | 9,939 / 189 |
| Generic structured report for twenty product subtrees | 39,151 |
| JSON attribute report / compact projected title array | 8,690 / 192 |
| Compact twenty-record domain JSON | 1,132 |
| Root plus select help | 1,867 |
| Full catalog plus schema output | 71,010 |
| Guarded three-backend Python comparison script | 460 |

Full schema discovery is avoidable overhead; equally compact alternative extractors obtain the same principal payload savings. Smaller incomplete text is not a legitimate efficiency win.

### Local subprocess timing

Thirty measured launches per method after three warmups; randomized order; identical fixed local title-extraction task. Median milliseconds: HTMLCut 2.98, htmlq 2.95, pup 2.61, Beautiful Soup 667.98, lxml 614.86, Selectolax 606.93. **Bare Python startup itself measured 625 ms in this environment**, so these do not establish 200-fold parser superiority. In an already-running Python process, 100 parse-and-select operations averaged 10.18, 0.69, and 0.58 ms/document respectively. Rust in-process performance was not measured.

## Provenance and scope

Release tarball SHA-256: `82f97c200671a7f6d5c24468bf0529547f3e9a8059ddb73d5840a881c1a754ba`.

Public input capture: Actions run `35974248552`, artifact `10797323033`, seven-day retention. The capture workflow is read-only and has no schedule. Full measurement JSON, trial data, scripts, hashes, and the detailed report were supplied in the accompanying evaluation download. Full third-party article bodies are not redistributed in that download. Changing live inputs are not a substitute for the recorded historical snapshots.
