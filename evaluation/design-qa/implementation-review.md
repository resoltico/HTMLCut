# Independent adversarial implementation review

## Scope and verdict

A reviewer separate from the implementation pass inspected the maintained DOM/selector adapter, parsed matching-depth admission, all-arena preparation limits, CLI IPC, native resource policy and atomic publisher. Concrete findings were sent directly to the implementers and corrected before this report. No outstanding production-code defect was found in the reviewed local tree. Native macOS ARM evidence passes; native macOS Intel, Linux and Windows remain required acceptance gates, not inferred successes. This is conditional acceptance of the implementation, not authorization to tag or publish a release.

The feature-disabled candidate built with the release profile independently copied and checked in the recorded native pass has SHA-256:

`8d809a8e604964a0f4fa77fdbd34854a755fe4878847385dfc9c7550a43c2dd4`

The earlier candidate built with the release profile and used for the complete worker-control/race pass has SHA-256:

`f2d016ddeed1744a4633803d666bd9dd3e5ee152b940ba4e0250e79abc88bcdb`

The difference is the deliberate removal of unused `scraper` default features, including parser-error collection and its CLI dependencies. Source inspection confirms error collection does not govern HTML repair. Final-byte native pressure, identity refusal and complete extraction/inspection values were independently rechecked after that manifest change. [Recorded source digests](reviewed-source-digests.json) identify the inspected files; hashes establish identity, not review quality. CI or package/signing may produce different bytes that need their own native verification.

## Findings corrected

1. **macOS small absolute caps were ineffective.** Native probes showed `EINVAL` under a fixed 128 MiB limit because the executable already maps large shared reservations. The worker now installs baseline-relative native virtual-growth limits, rejects excessive resident startup, and requires an over-cap native mapping refusal before reading input. Documentation explicitly excludes an exact RSS claim. Pre-reserved mappings can become resident after policy installation; the earlier [design QA](README.md) preserves the counterexample.
2. **Windows policy was initially not verified in the child.** The supervisor now assigns the Job Object before releasing a start byte. The worker then queries its immediate job, verifies nonzero bounded process committed-memory and kill-on-close policy, and requires `VirtualAlloc` refusal before ready/plan input. [Microsoft documents immediate-job querying](https://learn.microsoft.com/en-us/windows/win32/api/jobapi2/nf-jobapi2-queryinformationjobobject) for nested jobs. Native Windows compilation and execution remain pending at this review, so the inspected API sequence is not claimed as executed Windows proof.
3. **Supervisor death could leave a Unix worker running.** The worker now installs a CPU backstop and parent-lifetime guard; Linux also uses a parent-death signal with a race check. A real supervisor kill on the reviewed macOS debug candidate left no running worker; termination was observed after 0.0057 seconds.
4. **Negative probes initially accepted unrelated early errors.** Private evaluation workloads now emit a start marker before their injected failure. Verification checks the relevant deadline/channel/protocol outcome. Pressure requires a successful measured tuple; it cannot pass merely because the worker failed. Known wall/channel failures report their actual bounds rather than a misleading generic memory bound.
5. **Matching caches and recursion admission were lost through opaque per-element matching.** The final adapter parses once through public upstream `scraper::selector::{Parser, Simple}`, uses upstream `ElementRef`, `MatchingContext` and `SelectorCaches`, and admits AST matching paths iteratively. It introduces no custom selector parser, element implementation or fork. Fresh matchers own caches for one immutable document and borrowed selector. Root-relative scope and inclusive row anchors are preserved.
6. **Preparation counted only attached nodes.** The final parser admission counts every retained arena node/element, including detached parser-repair nodes, and separately checks attached DOM depth. This prevents orphaned constructed nodes from evading declared counts. Checks remain post-parse; the library does not claim physical containment.
7. **Hardlink aliases passed publication admission.** An independent real hardlink source/output test initially succeeded under `--overwrite`. Atomic replacement preserved source contents, but the alias was not distinct as required. Native file identity now rejects it. Rechecked final candidate bytes return exit 2, no stdout, and preserve both original byte streams. Equal contents in distinct files remain valid through the implementation's separate positive regression.
8. **Caller source wait was conflated with active execution deadline.** The deadline is now defined explicitly around active worker processing, excluding intentional caller source-input wait. Plan compilation completes before source acquisition. A stalled worker input pipe is still bounded and independently tested; this is different from waiting for a caller to finish stdin. No HTML parsing occurs during that wait.

## Executed evidence

[Worker boundary results](worker-boundary-results.json) preserve all outputs, exit statuses, timings and binary identity for the immutable release-profile control pass. All eight controls passed: native allocation refusal, touched memory pressure, stdout flood, stderr flood, abnormal exit, malformed frame, blocked worker input, and timeout. Workload-start markers and exact outcome checks establish that relevant failure paths were exercised. Blocked worker input ended after 10.0118 seconds and timeout after 10.0174 seconds. No public stdout/stderr escaped those private verification runs.

The final feature-disabled binary's [extraction results](extraction-results.json) record native pressure allocation of **1,040,187,392 bytes**, measured resident memory **1,043,546,112 bytes**, and configured virtual-growth policy **1,073,741,824 bytes**. These observations prove touched allocation/refusal on this native host, not a universal RSS bound. Both hardlink aliases retained their original bytes after refusal.

The earlier complete release-profile pass also launched eight concurrent real writers against one no-overwrite destination: exactly one succeeded, and the destination contained the complete expected JSON. [Additional publication/lifetime results](publication-results.json) preserve semantic failure with an existing destination, no leftover staging file, supervisor termination, and an intentionally valid wrong-business-identity success. That success demonstrates the stated semantic limitation rather than an engine defect.

Independent targeted Rust execution:

```sh
cargo test --locked -p htmlcut-core --test selector_matching_depth --test row_containment
```

At that execution, all 11 discovered tests passed, including all eight parsed-depth/branch/process controls and three row/context/refusal controls; no tests were skipped. A subsequent additional compiled positional-plan cross-document reuse regression was added by the implementer and belongs to the final required gate; its execution is not claimed as independently rerun here.

## Scripted agent ergonomics

Two exercises used the handoff's synthetic corpus. The reviewer already knew the corpus/handoff context: these are **scripted exercises, not blind trials or model-token measurements**. No retries were required. All complete values were compared to `evidence/expected.json`, not inferred from a successful exit.

- **Catalogue:** `inspect --select article --samples 2` returned count 30 and the `book` class in 436 bytes. First-row `outer-html` returned the complete representative row in 209 bytes. A [saved guarded plan](catalog-guarded-plan.json), including a document-title equality guard, returned all 30 exact records in 2,792 bytes. The missing-field fixture refused with exit 3 and zero data.
- **Repair amount:** `inspect --select 'h2,p' --samples 3` returned the three relevant nodes and IDs in 523 bytes. Selected `outer-html` returned the complete amount node in 37 bytes. The [saved heading-guarded plan](repair-guarded-plan.json) returned exactly `EUR 3,140.00` in a 17-byte JSON array plus LF. Heading drift refused with exit 3 and zero data.

The byte counts describe actual tool outputs. They are not token counts, token-cost savings, preference/adoption evidence or a competitive benchmark. Arguments and complete result values are retained in the final extraction evidence.

## Reproduction

Use the final built executable and synthetic fixtures under `evaluation/contract-matrix/fixtures` with the saved plans above. Example commands:

```sh
HTMLCUT_PRIVATE_WORKER=verify-memory /absolute/path/htmlcut
HTMLCUT_PRIVATE_WORKER=verify-pressure /absolute/path/htmlcut
HTMLCUT_PRIVATE_WORKER=measure-pressure /absolute/path/htmlcut
HTMLCUT_PRIVATE_WORKER=verify-stdout-flood /absolute/path/htmlcut
HTMLCUT_PRIVATE_WORKER=verify-stderr-flood /absolute/path/htmlcut
HTMLCUT_PRIVATE_WORKER=verify-abnormal /absolute/path/htmlcut
HTMLCUT_PRIVATE_WORKER=verify-malformed /absolute/path/htmlcut
HTMLCUT_PRIVATE_WORKER=verify-blocked-input /absolute/path/htmlcut
HTMLCUT_PRIVATE_WORKER=verify-timeout /absolute/path/htmlcut
```

These environment modes are private delivered-binary evaluations, not extraction interfaces. The production worker installs native controls independently of the evaluation workload. The package/native evidence scripts and Rust integration tests automate equivalent checks. The fixture paths recorded in raw evidence refer to the extracted handoff used in this session; the repository contains the same synthetic source material.

## Remaining acceptance gates

Only native `aarch64-apple-darwin` was independently executed here. Native Intel macOS, Linux musl and Windows MSVC must prove allocation/deadline behavior, Job timing where applicable, source/output identity and file semantics, termination, actual packaged bytes and signing/packaging behavior. The oldest supported macOS floor was source-reviewed, not executed here. The implementer's full required gate, final paired performance comparisons, fuzz/property checks and all-platform CI need their own retained evidence. A locally passing review cannot turn an unrun or failing native gate green. Do not mark the PR ready until required authoritative checks complete successfully.

## Subsequent source-independent grammar correction

A later exact-app-diff review found the supervisor still canonicalized named input paths before worker grammar compilation. A missing source file therefore masked invalid CSS with an acquisition error. The implementer moved destination/input identity validation into the acquisition closure, after the capped worker's compile acknowledgement and before snapshot open/read or source-payload release. Independent skeptical inspection confirms alias, no-overwrite and atomic publication protection remain in place; closure failure kills/reaps the waiting worker and emits no data. Invalid CSS/regex plus missing source and an existing destination now have explicit negative controls, alongside valid CSS retaining the acquisition exit class.

This is a source-review follow-up; the candidate hashes and native measurements above refer to the earlier immutable copies actually executed. The changed application composition requires the rerun final gate and new final binary/package hashes. Native evidence is not silently reassigned to different bytes.
