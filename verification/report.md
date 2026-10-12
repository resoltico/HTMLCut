# Implementation

Starting baseline: main at f03af70318cb96aee0d0a05c62ac17cb5a8974f1. Workspace version remains 22.0.0. W0–W11 have one commit each. The provided 57 contract and five outline cases remain byte-exact.

The product surface is recursive closed contracts, scoped/row-relative sibling extents, lexical value checks and named captures, WHATWG byte decoding/base URLs, structural text, filtered upstream HTML serialization, bounded outlines and repair facts. The library and CLI share that engine. Product code contains no unsafe block, subprocess launcher or environment-selected execution mode. Operator input/node/output/time policy and fixed depth admission replace the former execution controls; named output is atomic replacement after complete acquisition/evaluation.

File disposition follows the bill. The retained authored fixture pages still protect HTML repair and complete lexical rows; their regression checks every value. Selector consumer-stack controls are adapted and retained. check-release-verification.py remains because the release workflow still relies on exact-source successful main CI. Release attribution, source binding, signatures, checksums, provenance, trusted tool caching and mandatory-result controls remain, with their negative controls.

## Deviations and interpretations

- W0 only adds content; the existing .gitattributes necessarily has added lines rather than being a new file.
- Preparation acceptance uses the actual library decoding/parsing and complete title values before contract evaluation is connected. All cases will also run through the finished Document::extract and binary.
- Chunk equivalence applies to the 60 decodable pages; the malformed/unknown-label pages must refuse before parsing, and their exact error facts are tested instead.
- Unknown caller-supplied encoding labels refuse. Unknown in-document declarations are ignored by the WHATWG prescan algorithm, allowing later valid declarations or UTF-8 fallback. This resolves the broad wording of DESIGN 4 in favor of its expressly required standard algorithm.
- scraper's deterministic feature is required to preserve source attribute order. Source class order is read from the attribute, since scraper's cached class list sorts it.

- Regex compilation retains the existing aggregate 8 MiB allowance, partitioned between compiled patterns and their DFA caches. Each pattern does not receive an independent 8 MiB allowance that could multiply across the contract.

- W4 acceptance depends on readings scheduled for W5 (attributes, URLs and block text). The shared reading implementation lands alongside evaluation so W4 cases genuinely pass without stubs; W5 supplies the complete reading conformance and isolated rule QA.

- Reading QA follows DESIGN rather than the reference model on preceding unexcluded table cells, inherited preformatted context, whitespace-only verbatim lines, and empty br-only preformatted lines. No corpus expectation differs or was edited.
- Rule 10 is interpreted as joining lines without an added framing LF; a final blank line preserved by rule 7 can contribute a final separator. Removing that separator would discard the required blank line.
- An explicitly requested predicate attribute must exist even without equals/pattern, following DESIGN's missing-attribute clause; the model only reads when it has a value test.

- Outline identifiers use CSS escaping and quoted previews use JSON escaping. Selector headers containing control characters use a quoted JSON string, keeping the specified line budget meaningful. The five supplied outline expectations remain byte-identical.

- QA item 5 suggested nested :has(), which the strict upstream CSS parser rejects. Preserve that grammar rejection and use a valid selector list on depth 1000 for the watchdog experiment. No conformance expectation is involved or changed.

- DESIGN 6's blanket zero-stdout-on-failure cannot hold for transport failures after a consumer has already received bytes and closes its pipe. Actual boundary experiment observed 4096 bytes then exit 5/output_unwritable/Broken pipe. Preserve truthful error status, no publication on contract/preparation/limit refusals, and atomic named output; document this transport exception instead of claiming rollback of stdout.
- max_output_bytes bounds the complete canonical JSON result, before its command-line framing LF; raw output uses that same extraction policy and is no larger. Outline bytes include their line terminators. This keeps the library's one result representation authoritative.

- The valid descendant-chain timeout candidate completes too quickly in the optimized binary. The retained experiment finds a 256-branch valid :has() list (5265 bytes) that exceeds 50 ms; it exercises the watchdog without relaxing the deadline or assertions.

- The new authored-snapshot regression initially guessed 30 rows and Title 001. Inspection of the retained source establishes 20 rows with Complete title N — Alice's book and GBP N.00; the new regression oracle is corrected against that source and checks every row. The supplied conformance corpus is unchanged.

- Published changelog prose/dates/claims are retained; links to replaced docs are retargeted to their immutable release-tag versions so historical claims do not point to current semantics.

- W10's literal cargo-tree absence conflicts with retained tempfile: it transitively needs rustix/libc/Windows support. Remove direct bindings, retain maintained transitive platform implementation, and report this deviation rather than replacing atomic publication with custom platform code.

- License review of unmodified foldhash 0.2.0: retain Orson Peters' copyright and the original Zlib notice/permissions in target attribution. deny.toml allows Zlib only for that exact crate version required by source-order-preserving scraper/indexmap. The notice generator derives accepted texts from that policy; there is no general Zlib allowance.


No supplied conformance expectation was questioned, edited or removed. The incorrect newly written authored-snapshot oracle and invalid timeout experiment were reported before correction; neither changed the executable specification.

# Verification

The final local full gate passed on native macOS ARM: 68 Rust test executions (including both 62-case corpus runners), 29 Python tests, warnings-denied Clippy/rustdoc, formatting, dependency/advisory/license policy and frozen-corpus admission. An independently committed clean copy had the same staged tree and passed the full gate and signed native-package verification.

Rust 1.88.0 builds and passes the workspace suite; 1.87.0 is refused by the locked encoding_rs/ICU minimum declarations. Development remains pinned to 1.99.0. Both fuzz targets completed exactly 10,000 iterations with no finding; seeds derive from conformance and retained document inputs, with all mutations/crashes external.

QA items: malformed offsets and UTF-8/UTF-16 cross-chunk cases pass; the standard prescan covers pragmas, duplicates, comments/quoted false declarations, late declarations and substitutions; named captures pass; Windows atomic replacement awaits native CI; the source-built optimized CLI ends a genuine valid slow selector at 50 ms without data or a test mode; the minimum compiler is established above; template fragment exposure and skip/read behavior are tested.

Measured on macOS ARM with the source-built optimized dist executable: 3,380,416 bytes, median 2.4077915 ms for one small get (three warmups, 30 fresh-process trials), and peak resident memory 4,489,216 bytes on the largest conformance page, 1,291 bytes (the intended ambiguous-field refusal, exit 3). Raw samples, command, binary hash and method are in [measurements.json](measurements.json). This is not a cross-platform performance claim.

Line counts against the baseline: core Rust 7,752 → 3,930; CLI Rust 5,821 → 843; scripts 2,747 → 2,146. Counts include blank lines/comments and crate Rust tests/examples; script counts cover scripts directory shell/Python/PowerShell/C#, excluding workflows and release tests.

Checks still awaiting the PR head: full native Linux/macOS Intel/Windows execution and packaged-binary corpus execution on all four targets. Exact-head CI and artifacts are authoritative; local success does not establish those platforms. The report in the PR description will record final outcomes. macOS 12 itself, published candidate registry installation, public candidate download/provenance execution and blind-agent/billed-token studies have not run. Publication is required before public-download/registry checks; the older OS and studies are outside this implementation's executed evidence.

# External rollout

Push the branch and open a PR against main after local completion, then require all final-head CI gates before readiness. No merge, repository setting/description edit, tag, GitHub Release or crates.io publication is authorized. Temporary tags used by frozen/release negative controls exist only in isolated test repositories, not HTMLCut.

CI retains four native packages and minimum-toolchain/fuzz/full-maintainer gates; the aggregate rejects missing, failed, cancelled or skipped dependencies. Public release automation remains bound to exact-source main CI and verifies downloaded packages only after publication. Enabling additional repository-required settings and actual public release are separate operator actions.
