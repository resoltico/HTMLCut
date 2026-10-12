# Architecture

HTMLCut is an extraction-contract engine for caller-owned HTML snapshots.
The Rust core compiles closed plans, lazily prepares an immutable upstream scraper
DOM, checks complete cardinalities and original-DOM guards, and projects owned
strings/records. It has no acquisition, browser, reader or converter subsystem.

The CLI supervisor admits plans and regular-file/UTF-8 source bytes, communicates
with one private child running the same executable, verifies its bounded response
and successful termination, then uses the existing atomic publisher. Worker stdout
and stderr remain private until a complete result is admitted. A ten-second active-worker deadline (excluding caller input wait), bounded
concurrent pipe pumping and kill/reap cleanup protect against blocked or runaway
workers. A worker failure cannot become a partial successful extraction.

## Native resource policy

Resource policy is installed before untrusted parsing/matching. Linux uses native
address-space limits. macOS uses a virtual-size baseline measured in the fresh
worker plus a bounded virtual-growth allowance: a small absolute cap cannot fit
Darwin's large shared virtual reservations. A native over-cap mapping preflight
must fail before input is accepted. This is a virtual-growth guarantee, **not an
exact RSS cap**; pre-existing mappings can become resident. Worker startup and
baseline must be bounded and independently tested. Windows uses a Job Object
assigned before input release, with a process-memory cap and kill-on-close.
The worker allowance is 1 GiB, declared once in `worker.rs`; the macOS pre-input
resident baseline must be at most 64 MiB. Linux limits total virtual size; Windows
limits process committed memory. These differ from exact total RSS. A valid
50 MiB inert-script snapshot is a required native acceptance case.

Only the narrowly scoped OS wrapper permits unsafe calls. Product extraction
remains safe Rust. This isolates a fixed binary's untrusted data processing; it is
not a sandbox for arbitrary code or plugins. The library does not install these
physical controls. Source limits precede upstream parse; prepared node/depth limits
are post-parse admission, and owned traversal units do not count all upstream work.

## Design and challenge

The handoff is a proposal, not an independent correctness oracle. Our selected
architecture keeps the reduced version-8 contract and existing publication code,
uses the public upstream selector parser/matcher with upstream element callbacks, and removes
the five parser/selector carriers only when the worker boundary is verified.
A hand-written parser, a second extraction engine and compatibility adapters add
ownership without solving this task and are excluded.

The [separate skeptical design review](../evaluation/design-qa/README.md) found
three important corrections: macOS needs a baseline-relative cap; virtual growth
must not be described as RSS; upstream opaque per-element matching loses shared
positional caches. The implementation instead compiles the public upstream selector
AST with scraper's public parser and uses upstream matching contexts/caches; this
also retains iterative 64-unit matching-depth admission without customized engines. Native probes and source inspection establish feasibility, not
finished product assurance. Anchor-inclusive matching, original context, negative
guards, exact complete values and native resource failure tests remain mandatory.

For paired small-query CLI trials, the pre-registered acceptance threshold is a
median launch-time ratio of at most six to published v21 on the same machine and
fixtures, while preserving complete values/refusals. The worker adds an intentional
process launch and IPC boundary. Positional selectors and warm DOM/plan reuse need
separate measurements; cold Python launches are not warm-parser comparators.
No byte ratio is reported as a token saving. Measurements and failures are retained
in [evaluation](../evaluation/README.md).

Atomic file create/replace is a single-file publication guarantee, without crash
durability. Once a complete result begins writing to arbitrary stdout, a failed
recipient can observe a prefix. Source content, business identity and domain
interpretation remain caller responsibilities.
