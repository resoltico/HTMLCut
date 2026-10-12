# Independent skeptical design QA

This review challenged the selected design separately from the implementer's design pass, before core or CLI replacement. It is conditional permission to build, not acceptance of the final implementation. The handoff is evidence and a proposal; the user's requested scope controls the work.

## Verdict

The smallest coherent architecture is one maintained `scraper` DOM, the reduced version-8 plan contract, a reusable trusted in-process Rust API, and one native CLI supervisor/worker executable. Keep the existing atomic publisher. Remove the five fork carriers only after worker policy and remaining semantic regressions pass. No compatibility reader or parallel extraction implementation is warranted.

Native memory enforcement is feasible on this macOS ARM host, but its precise guarantee is **virtual-memory growth after worker startup**, not a fixed resident-memory cap. A fixed small `RLIMIT_AS` is invalid because the native executable already maps roughly 500 GB of virtual address space, mostly runtime/shared reservations. The successful native policy is current virtual-size baseline plus a bounded growth allowance. The worker must install both soft and hard limits before reading the HTML or plan payload. Tests must measure the real worker baseline and legitimate large-input behavior.

## Independent challenges

1. **Native cap must reject allocations, not just return success.** The C probe below demonstrates both `malloc` and `mmap` allocations failing with `ENOMEM` under baseline + 128 MiB. A fixed 128 MiB cap returns `EINVAL`; ignoring that return permits 384 MiB allocation. Do not use `RLIMIT_RSS` as a separate control: the SDK aliases it to `RLIMIT_AS`. Do not promise exact RSS.
2. **Existing mappings remain usable.** Reserving 256 MiB before installing baseline + 128 MiB, then touching it after installing the cap, reaches about 404 MiB RSS before further allocations fail. This falsifies a 128 MiB RSS guarantee. Startup must precede untrusted input, and actual worker baseline mappings need native stress evidence. This is a resource boundary around a fixed executable, not a sandbox for arbitrary code.
3. **Jetsam is unavailable to ordinary standalone binaries.** `memorystatus_control(7, self, fatal active/inactive 128 MiB)` returns `EPERM` for uid 501 here. Apple source gates it on root/private entitlement; ad hoc signing cannot supply an Apple private entitlement. Do not introduce privileged helpers or claim this path works.
4. **Upstream scope semantics need deliberate adaptation.** `scraper` 0.27 has public `Selector::matches_with_scope`, sufficient for original-DOM `:scope`. `ElementRef::select` excludes its anchor, while v21's selection scan includes the anchor. Scan an anchor subtree inclusively or explicitly prepend/check the anchor. Preserve original DOM context for selectors and guards; do not clone rows into fragment documents that alter ancestor, sibling, `nth`, or `:scope` semantics. Exclusions affect projection after original-DOM guards.
5. **Public matching loses shared nth caches.** `matches_with_scope` creates fresh caches each call; the reusable-cache API is crate-private. Repeated positional matching may become quadratic. The public `ElementRef::select` iterator retains caches but excludes the anchor. Prefer a cached public iterator plus an explicit anchor check if measured positional regressions warrant it; otherwise document the measured tradeoff and enforce the deadline. Avoid recreating a custom selector stack merely for caching.
6. **Logical traversal units are not selector-engine work.** Replacing callback accounting with public matching cannot preserve the old meaning of `max_work` or parser-construction counters. Define retained counters as what is actually counted, explicitly change the direct-library assurance contract, and rely on native CLI policy for physical containment. Depth checks after DOM construction are postconditions, not parser admission protections.
7. **Worker IPC needs simultaneous pumping.** The supervisor must bound worker stdout and stderr while sending input and waiting for termination. A blocked input write must not bypass the wall deadline. Kill/reap on timeout, overflow, malformed/truncated frame, panic, or unsuccessful exit; keep raw child diagnostics private. Accept exactly one complete bounded response and successful exit before publication. A trailing success marker alone is insufficient.
8. **Protocol payload limits differ from source limits.** JSON-encoding arbitrary HTML can expand its byte size several-fold. Bound complete request framing independently or use checked length-delimited raw source bytes. Validate response length before allocating, reject trailing frames/bytes, and separately bound serialized result bytes.
9. **Atomic publication remains essential.** Preserve a destination on semantic or worker failure, keep no-overwrite atomic against concurrent writers, and retain regular-file admission before reading. Canonical path equality alone misses hardlink aliases: verify input/output identity using native metadata where the contract requires alias refusal. Once complete output is written to arbitrary stdout, a downstream write failure may expose a prefix; do not promise stdout transactionality.
10. **Simple fixtures cannot prove complete value fidelity.** Run the supplied full expected-value matrix with current v21 version 7 and candidate version 8 plans. Preserve malformed adoption, templates, inert text, Unicode, preformatted text, URL attributes, exclusions, nested CSS, `:has`, `nth`, and source-free errors. A syntactically valid wrong business entity can still pass; document and demonstrate that limitation.

## Native probe evidence

Compile and run on a native macOS host:

```sh
cc evaluation/design-qa/macos_memory_probe.c -o /tmp/htmlcut-memory-probe
/tmp/htmlcut-memory-probe fixed
/tmp/htmlcut-memory-probe delta
/tmp/htmlcut-memory-probe delta mmap
/tmp/htmlcut-memory-probe delta reserve
/tmp/htmlcut-memory-probe delta reset
/tmp/htmlcut-memory-probe delta prereserve
/tmp/htmlcut-memory-probe jetsam
```

The `delta` failures intentionally exit 3 when allocation is refused. The ordinary malloc and mmap paths touch every allocated page. `reserve` attempts an untouched 256 MiB mapping; it is refused with `ENOMEM` under the 128 MiB growth allowance. `reset` attempts to raise the installed hard cap back to infinity; that fails with `EPERM`. `prereserve` is the RSS-counterexample described above. Complete original logs and host/compiler metadata are retained beside this report. `data.txt` records a `RLIMIT_DATA` probe whose old print label incorrectly says AS; the source's current `kind` label fixes this ambiguity. Its fixed 128 MiB limit also failed with `EINVAL`.

A runtime native preflight can install the cap, attempt a single native `mmap` larger than the configured growth allowance with no page touching, and require `MAP_FAILED`/`ENOMEM` before accepting input. If the mapping unexpectedly succeeds, unmap it and refuse. This detects an ignored control without allocating resident pages. Such a preflight does not prove RSS containment or replace native parser stress tests.

## Upstream/source evidence

The installed upstream `scraper` 0.27 source was inspected at `src/selector.rs` and `src/element_ref/mod.rs`; [upstream source](https://github.com/rust-scraper/scraper) exposes scope matching and the cached iterator with the distinctions above.

Apple's [Monterey-era XNU 8020.101.4 resource implementation](https://github.com/apple-oss-distributions/xnu/blob/xnu-8020.101.4/bsd/kern/kern_resource.c), [Ventura-era XNU 8792.81.2](https://github.com/apple-oss-distributions/xnu/blob/xnu-8792.81.2/bsd/kern/kern_resource.c), and [Sequoia-era XNU 11215.81.4](https://github.com/apple-oss-distributions/xnu/blob/xnu-11215.81.4/bsd/kern/kern_resource.c) all route `RLIMIT_AS` to `vm_map_set_size_limit`; source evidence supports investigating the existing macOS 12 floor, not an automatic floor bump. This does not substitute for native floor/Intel tests. Apple's [current memorystatus implementation](https://github.com/apple-oss-distributions/xnu/blob/main/bsd/kern/kern_memorystatus.c) rejects memory-limit control without root/private entitlement, consistent with the native result.

## Conditional gates and limitations

Only macOS ARM was executed in this review. Native Intel, Linux and Windows remain unrun. All four release targets must test actual worker allocation/timeout refusal, pre-input policy, complete output framing, termination/reaping, resource-failure publication, and packaged executables. Windows must assign the Job Object before releasing input; Linux must enforce its address-space cap before parsing. Do not infer native success from compilation or mocks. The final independent review must examine the implemented tree and native results before marking the PR ready.

This review gives a conditional **GO to implement** the coherent design, with the precise macOS virtual-growth contract and these falsification gates. It does not authorize release publication or weakening failed tests.
