# Foundations acceptance and implementation QA

This records responsibility migration from baseline `bc3e6f2319f3705e810b821ed7e22784e4942f21` (22.0.0) to the breaking foundations change. The checkout matched that baseline and had no unrelated changes; no reset or source-snapshot overlay was used. Release tags/history and original legal material remain unchanged. This record is not a claim that external CI, publication or human full-mutation review has occurred.

## Product acceptance

| Requirement | Implementation and control |
|---|---|
| Pure engine / shared application boundary | Core forbids unsafe code. Runtime owns local acquisition, preflight, exports, staging/authorization, commit/completion. CLI production dependencies are exactly Clap/runtime. Metadata guard has prohibited-dependency negative graphs. |
| Prepared reuse / thread safety | Admitted owned strings in the existing tree; no unsafe Send/Sync or second DOM. Exact Document/Contract/value/report/opaque-handle auto-traits, source release, independent worker policy/cancellation and reuse tests. |
| One job context | Checked absolute deadline shared through compilation, acquisition chunks, decoding/parsing, indexing, evaluation, bounded outline/JSON encoding and delivery. Job-local counters share context; no phase restarts time. |
| Stop/commit/completion | Barrier races, early/late cancellation, delayed supervisor, actual commit outcome, inert observers, panic/Internal behavior and cooperative stop followed by held cleanup. Completion encloses owned native/stage cleanup. |
| Shared exports | Typed/JSON quick-query parity and rejection bounds; one core raw policy; canonical LF admission including optional null; later smaller Save re-admission. Schema exact/below-bound controls. |
| Structured outline | Borrowed row facts enter one bounded canonical writer before owned copies. The same writer renders later Saves. Header line/omission accounting, clipped attributes/classes/previews, namespaces/template/grouping, UTF-8 and hidden oversized identifiers are controlled. |
| Corpus / delivered binary | Core, runtime and actual CLI consume independent literal fixtures. `HTMLCUT_TEST_BINARY` selects an unpacked production executable. Empty/incomplete/broken-expectation controls remain. Four embedded Python product harnesses are Rust/native probes. |
| Native guarantees | Existing authorization/staging algorithms are relocated, not replaced by generic atomic-file helpers. Direct-stream BrokenPipe is neutral Disconnected; named publication retains private-before-bytes and identity/policy revalidation. |
| Packaging / release | Three versioned product crates, coordinated Cargo verification, source archives, exact native layout/modes/legal data, strict signatures, immutable publication admission and source-bound public-byte execution. External gates remain distinct. |

## Separate implementation QA findings

The skeptical implementation pass found and repaired: shell-marker handling in explicit runtime file sources/destinations; contract-file FD0 aliases escaping conflict admission; observed cancellation being remapped to a typed grammar-size error; output encoding lacking an initial zero-time check; shell repair text inside runtime; unbounded intermediate attribute-name copies during smaller-policy outline rendering; unnecessary deep copies on Save; missing cooperative checks between short reads/writes; duplicate supervisor claims, missing fail-safe exit around supervisor reporting, and archive version/path and repeated-directory admission. Regressions exercise those boundaries. Native code and release protections were reviewed independently of the architectural design verdict.

The baseline Python run discovered 117 tests with one frozen-helper failure and one native Windows skip; the full log was retained. The handoff's two process timing findings were separately repaired with explicit readiness and a post-cleanup release gate. A deliberately defective cleanup fixture proves detection; isolated passing reruns are not used to erase earlier failures.

## Native acceptance migration

| Former family | Retained/new real boundary |
|---|---|
| representation/stdin/schema/help | CLI conformance and input-contract tests; runtime facade preflight/export/schema tests. |
| blocked input/stderr | `process_lifetime.rs`: filled native pipe control, held input, self-termination and exactly one readable diagnostic. |
| publication without parent listing | `native_access.rs` on Unix; `windows_authorization.rs` plus independent restricted-token PowerShell/native control on Windows. Native open/create/rename, denied listing, parent aliases, write/search refusals, exact policy and no ordinary residue. |
| FD0 aliases / unconnected FIFOs | CLI input-contract, destination and lifetime suites; both contract/HTML roles, symlinks, every maintained descriptor namespace and three unconnected FIFO roles. |
| connected Windows pipe lifetime | Real held named-pipe producer/consumer in `windows_authorization.rs`; test-parent termination is cleanup/failure, never evidence of executable timeout. |
| native authorization/inheritance | Real denied reads before/after replacement, independent ACL controls and restoration in Unix/Windows suites; runtime unit/native admission controls. |
| Windows parent ACL propagation | Protected/unprotected and inherited-only fixtures; actual read effects track parent changes/removal and binary ACE facts match. |
| Windows mandatory integrity | Initial low-token read permitted, then medium no-read-up label denies; normal reads and low denial plus owner/DACL/label facts survive publication. Helper impersonation is scoped/restored. |
| two-phase external acquisition | Maintenance HTTP-server test: 404/500/truncation never extract/change output; redirects, compression, charset and no-JS semantics use explicit production binary. |
| owned-stage cleanup | Runtime native identity/race tests and delivered refusal/success/expiry tests. Only confirmed owned objects are cleaned. Foreign replacements/unconfirmed creations survive; blocked commit/hard-kill residue caveats remain. |

## Maintenance disposition

Destinations below are responsibility records, not executable aliases. Paths abbreviated as `release/...` belong to `tools/htmlcut_maintenance/`.

| Baseline path | Outcome / protection |
|---|---|
| `scripts/run-resource-bounded.py` | `tools/htmlcut_maintenance/process.py`; direct ownership API and real process tests, including deliberately defective cleanup. |
| `scripts/focused-smoke.py` | `tools/htmlcut_maintenance/fuzz.py`; exact seeds, manifest target discovery, sequential bounded phases and retained evidence. |
| `scripts/mutation-audit.py` | `tools/htmlcut_maintenance/mutation.py`; source binding, nonempty exact outcomes, stop ownership and honest review status. |
| `scripts/generate-package-notice.py` | `tools/htmlcut_maintenance/release/notice.py`; locked graph coverage and original native/runtime terms. |
| `scripts/release-assets.py` | `tools/htmlcut_maintenance/release/assets.py`; bounded exact inventory/digests and immutable retry admission. |
| `scripts/release-notes.py` | `tools/htmlcut_maintenance/release/notes.py`; unique nonempty immutable-source section and bounded command capture. |
| `scripts/check-release-verification.py` | `release/admission.py` and `release/identity.py` in the maintenance package; latest same-source main push CI. |
| `scripts/verify-macos-signature.py` | `release/signature.py` and `release/accept.py`; real unsigned/tampered/wrong-identity controls before execution. |
| `scripts/native-acceptance.py` | Retired after migration to Rust delivered-binary suites and the external acquisition test; native matrix below. |
| `scripts/field-semantics.py` | Retired runner and historical expectation rewrites. All 68 final cases are literal `conformance/contract/semantic-*` fixtures. |
| `scripts/check-ci-results.py` | Retired. CI has one authoritative needs list and an always-run expression rejecting failed/cancelled/skipped dependencies. |
| `scripts/native-shell.py` | Retired. Python invokes native programs with argument arrays; no Python-to-Bash path/operation bridge. |
| `examples/acquire_snapshot.py` | Retained caller-owned acquisition recipe. Real local HTTP integration uses an explicitly supplied executable. |
| `tests/test_resource_budget.py` | `tools/htmlcut_maintenance/tests/test_process.py`; behavioral controls retained against the new owner/API. |
| `tests/test_fuzz_cleanup.py` | `tools/htmlcut_maintenance/tests/test_fuzz.py`; behavioral controls retained against the new owner/API. |
| `tests/test_mutation_audit.py` | `tools/htmlcut_maintenance/tests/test_mutation.py`; behavioral controls retained against the new owner/API. |
| `tests/test_package_notice.py` | `tools/htmlcut_maintenance/tests/test_notice.py`; behavioral controls retained against the new owner/API. |
| `tests/test_release_assets.py` | `tools/htmlcut_maintenance/tests/test_release_assets.py`; behavioral controls retained against the new owner/API. |
| `tests/test_release_notes.py` | `tools/htmlcut_maintenance/tests/test_release_notes.py`; behavioral controls retained against the new owner/API. |
| `tests/test_release_verification.py` | `tools/htmlcut_maintenance/tests/test_release_admission.py`; behavioral controls retained against the new owner/API. |
| `tests/test_release_source.py` | `tools/htmlcut_maintenance/tests/test_release_source.py`; behavioral controls retained against the new owner/API. |
| `tests/test_package_guides.py` | `tools/htmlcut_maintenance/tests/test_package_guides.py`; behavioral controls retained against the new owner/API. |
| `tests/test_macos_signature.py` | `tools/htmlcut_maintenance/tests/test_signature.py`; behavioral controls retained against the new owner/API. |
| `tests/test_workflow_wiring.py` | Retired YAML-shape/cache assertions. Concrete process/fuzz/install/release tests and Actionlint/real CI cover execution and wiring. |
| `tests/test_check_prerequisites.py` | Concrete command/ownership prerequisites in maintenance command/process tests; optional tools are not required by unrelated commands. |
| `tests/test_ci_results.py` | Retired with the duplicate result script; actual CI aggregate remains an external execution gate. |
| `tests/test_conformance_frozen.py` | Retired explicitly replaced byte-freezing policy; semantic oracle rejection tests remain. |
| `tests/test_contract_dialects.py` | Rust core conformance test checks documented accepted selector examples against literal dialect cases. |
| `check.sh` | Retired. `python3 -m tools.htmlcut_maintenance check` orchestrates standard checks and rejects empty discovery. |
| `scripts/focused-smoke.sh` | Retired. `qa fuzz` reads the nightly/tool inventory directly. |
| `scripts/check-conformance-frozen.sh` | Retired breaking policy. Literal semantic and exact-byte fixtures remain under normal review/testing. |
| `scripts/contributor-rust-tools.sh` | `tools/contributor-tools.toml`, `config.py` and `install.py`; fresh locked bounded installs, exact version/feature protocol, no custom binary caches. |
| `scripts/common.sh` | Concrete config, source and package functions replace shell path/native fallback helpers. |
| `scripts/workspace-version.sh` | TOML source manifest parsing in `config.py` / `release/source.py`; immutable refs never substitute advancing checkout versions. |
| `scripts/release-targets.sh` | `tools/release-targets.toml`; config derives matrix, names and floors. |
| `scripts/build-release-artifact.sh` | `release/package.py`; exact-source native Cargo artifact discovery, guide, attribution, signing and single native archiver. |
| `scripts/build-source-archives.sh` | `release/source.py`; Git archives of one resolved immutable SHA, with ZIP/tar modes and byte preservation. |
| `scripts/build-release-checksums.sh` | `release/checksums.py`; derived complete inventory and hashlib digests. |
| `scripts/smoke-release-artifact.sh` | `release/accept.py`; strict unpacking, modes/signatures, production binary outside checkout, common Rust delivered tests. |
| `scripts/release-tag.sh` | `release/source.py` and `release/identity.py`; canonical annotated tag, manifest agreement, exact SHA and main ancestry. |
| `scripts/publish-github-release.sh` | `release/publish.py`; exact immutable notes/bytes, draft convergence, no published retry edits or downgrade. |
| `scripts/verify-github-release.sh` | `release/verify.py`; bounded anonymous public download, checksum/source attestation and execution of downloaded bytes. |
| `.github/actions/cargo-tool/action.yml` | Retired composite and cache generations/trust/restore/save. Native installs use fresh owned roots and retained failure evidence. |
| `.github/workflows/ci.yml` | Retained/reworked: standard gate, MSRV, four native lanes, two-target fuzz, immutable checkout and authoritative aggregate. |
| `.github/workflows/mutation.yml` | Retained/reworked: main-ancestor SHA, fresh pinned tool, active runtime/CLI scopes, explicit exclusions and raw survivor/timeout failures. |
| `.github/workflows/release.yml` | Retained/reworked: same-source CI, annotated SHA, all native prepublication acceptance, provenance, serialized immutable publication and public-byte native lanes. |
| `.github/dependabot.yml` | Retained Cargo/GitHub Actions maintenance. Root workspace covers runtime; stdlib maintenance adds no Python dependency ecosystem. |

## Verification evidence boundaries

Run the standard gate with `python3 -m tools.htmlcut_maintenance check`. Native delivered acceptance includes explicitly ignored native/performance controls. Standalone Cargo tests need no Python; the acquisition recipe is a separately explicit Python/curl integration. Packaging verification uses pinned Cargo for core/runtime/CLI together, not `--no-verify`.

Real bounded fuzz and independently controlled cargo-mutants integrations are distinct from mocked orchestration tests. Full product survivor/timeout review remains a pre-publication condition, with active-platform scope and resource refusals reported honestly. Cross-compilation is not native execution. Apple Intel, Linux musl, Windows MSVC CI, exact-source aggregate failure injection, provenance and anonymous public-byte execution require their actual environments. No publication, tags, settings changes or external messages are authorized by this implementation.
