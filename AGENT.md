# AGENT.md — Autonomous Rust Engineering Doctrine

This file defines the operating standard for AI agents working on modern Rust projects.

It is not corporate boilerplate.
It is not motivational prose.
It is an execution doctrine for producing production-grade Rust code and production-grade tests with strong architecture, strong type design, and strict verification.

Use it as the default standard for:
- libraries
- services
- CLIs
- daemons
- backends
- systems tools
- Rust-backed desktop apps
- Tauri projects when applicable

If a repository has stricter local rules, follow those.
Otherwise, follow this document strictly.

---

## 0. Prime Directive

Produce Rust that is:

- correct
- explicit
- type-driven
- test-proven
- structurally clean
- secure at boundaries
- maintainable under change
- performant where it matters
- mechanically understandable

Do not optimize for:

- minimizing diffs at the expense of design
- preserving weak abstractions
- speculative generalization
- hiding complexity behind indirection
- “works for now”
- suppressing warnings instead of fixing causes

A fast wrong change is failure.
A messy working change is deferred failure.

---

## 1. Modern Stable Rust Baseline

Assume the modern stable Rust stack unless the repository explicitly overrides it.

Defaults:

- Rust: latest stable
- Edition: 2024
- Async runtime: Tokio
- Serialization/deserialization: Serde
- Observability: tracing
- Typed domain/library errors: thiserror
- Application / CLI / one-shot glue errors: anyhow

These are baseline defaults, not legacy holdovers.
Prefer mature GA tools over novelty.

When creating or revising `Cargo.toml`, default to:

```toml
[package]
edition = "2024"

[lints.rust]
unsafe_code = "warn"

[lints.clippy]
all = "warn"
```

Strengthen lint policy further if the repository already does so.

---

## 2. Agent Execution Protocol

### 2.1 Explore Before Designing

Before generating any design or implementation, map the current state of the repository.

Use available tools to inspect:

1. workspace layout and relevant `Cargo.toml` files
2. module and crate boundaries related to the task
3. existing type definitions, traits, and error types
4. existing tests for the affected area
5. repo-specific verification commands, scripts, `justfile`, `xtask`, CI mirrors, or coverage gates

Do not assume repository state.
Verify it.

### 2.2 Required Pre-Implementation Artifact

For any non-trivial task, produce a short, concrete design note in visible output before writing code.

It must cover:

- **Goal:** what is changing
- **Current shape:** what already exists
- **Boundaries:** which crates/modules/layers/types are involved
- **Invariants:** what must always be true
- **Failure model:** what can fail and how it is represented
- **Tests:** what will prove the change
- **Structure impact:** whether project/module/crate shape should change
- **Verification plan:** which commands will be run

Keep it brief and operational.
Do not write essays.
Do not skip it.

### 2.3 Standard Work Order

Always work in this sequence:

1. map repository state and constraints
2. define invariants and boundaries
3. design or revise the type model
4. design or revise project/module/crate structure if needed
5. define or revise the error model
6. add or revise tests
7. implement the smallest correct change
8. refactor until the result is obvious and local
9. run verification
10. fix failures at the root cause
11. rerun required verification
12. only then consider the task complete

### 2.4 Root-Cause Fixes Only

When verification fails:

- read the actual failure output
- identify the structural or logic cause
- fix the cause
- rerun the narrowest relevant check
- rerun the full required verification before declaring completion

Do not:

- guess at the failure
- suppress warnings with `#[allow(...)]` unless there is a proven false positive and the allowance is narrow and documented
- bypass failing tests unless the task explicitly requires quarantining a known issue
- claim completion while known required checks are still failing

### 2.5 Edit Discipline

Before editing existing code:

- read the relevant files first
- follow the repository's existing public API shape unless the task requires improvement
- preserve local conventions if they are sound
- improve design when needed instead of layering hacks on top

Do not edit blind.

---

## 3. Project Foundations

Project structure is not boilerplate.
It is part of the design.

### 3.1 Every Project Must Have an Intentional Shape

The tree must reflect real responsibilities and real boundaries.

Good signs:

- modules map to cohesive responsibilities
- dependency direction is clear
- tests have an obvious home
- configuration has a clear entry point
- crate roots stay thin
- transport/UI layers do not own domain rules

Bad signs:

- everything in `main.rs` or `lib.rs`
- flat `src/` with unrelated concerns mixed together
- “utils” or “common” containing large amounts of unrelated behavior
- one crate or module that everything depends on for unrelated reasons
- structure chosen by file length rather than responsibility

### 3.2 Start with a Shape That Matches the Problem

Typical larger-project shape:

```text
src/
  domain/
  application/
  infrastructure/
  interfaces/
  support/
```

Typical medium-project shape:

```text
src/
  lib.rs
  core/
  io/
  api/
  config/
```

Typical CLI-heavy shape:

```text
src/
  main.rs
  cli/
  commands/
  core/
  infra/
```

Do not cargo-cult these.
Choose a shape that matches actual responsibilities.

### 3.3 Split Modules by Boundary and Ownership

Split modules because:

- a concept owns a real responsibility
- visibility can be tightened
- tests become more honest
- boundaries become clearer
- unrelated changes stop colliding

Do not split modules merely to reduce file length or create an illusion of architecture.

### 3.4 Crate Roots Are Composition Roots

`main.rs`, `lib.rs`, and equivalent crate roots are composition roots.

They may:

- declare modules
- wire dependencies
- expose deliberate public surfaces

They must not:

- accumulate business logic
- accumulate parsing and validation rules
- become giant switchboards
- become god files

If roots grow, extract.

### 3.5 Multi-Crate Workspaces Need Real Boundaries

Use workspaces when multiple crates make the architecture cleaner.

A crate split is justified when:

- boundaries are stable
- dependency direction becomes cleaner
- compile/test isolation becomes meaningful
- the crate has a coherent reason to exist

A crate split is not justified when:

- it centralizes a fake “shared” crate
- it mostly exists for aesthetics
- it introduces churn without separation
- it turns into a dumping ground for generic helpers

### 3.6 No Dumping Grounds

Be suspicious of:

- `utils`
- `helpers`
- `common`
- `misc`
- `shared`

These names often hide unowned abstractions.
If such a module or crate exists, each item inside it must still have a crisp reason to be there.

### 3.7 Project Scaffold Must Support Testing and Verification

A serious Rust project should make it obvious where to put:

- core code
- boundary code
- integration tests
- examples
- benchmarks, when relevant
- fuzz targets, when relevant
- scripts or `xtask` automation, when relevant
- docs or generated artifacts, when relevant

The tree should reduce ambiguity, not create it.

---

## 4. Cargo, Features, Workspace Metadata, and Configuration

### 4.1 `Cargo.toml` Is a Design Surface

`Cargo.toml` should communicate:

- what the crate is
- what it depends on
- which features are real
- what is optional
- what the package identity is
- what lint posture the crate expects

Rules:

- no unused dependencies
- no accidental default-feature sprawl
- no duplicate sources of truth for package identity, version, or description
- no sloppy feature creep

If the workspace owns version and description, do not duplicate them elsewhere.

### 4.2 Dependency Discipline

Every dependency adds:

- maintenance cost
- security surface
- compile time
- compatibility pressure
- cognitive load

Prefer crates that are:

- mature
- widely used
- actively maintained
- well documented
- idiomatic in modern Rust

Do not reinvent commodity infrastructure without a strong reason.

### 4.3 Feature Flags Must Stay Coherent

Use feature flags for:

- optional integrations
- platform-specific support
- optional heavy dependencies
- rare runtime alternatives when genuinely required

Do not use feature flags to:

- create combinatorial chaos
- hide core correctness
- multiply weakly tested modes
- patch over poor architecture

When a feature materially changes behavior, tests must cover that mode.

### 4.4 Configuration Must Be Typed and Validated

Configuration is a real boundary.

Rules:

- parse configuration once near startup
- validate configuration eagerly
- represent config with typed structs, enums, and newtypes
- propagate typed config inward
- avoid passing raw environment variables deep into the system
- avoid repeated ad hoc parsing

Do not:

- scatter env access throughout the codebase
- make config stringly typed
- make config global unless there is a strong reason

### 4.5 Canonical Sources of Truth Must Stay Canonical

If a repository defines a canonical source for:

- package metadata
- operation catalogs
- error code systems
- protocol definitions
- public version identity
- config schema meaning

then update that source directly.
Do not create parallel authorities.

---

## 5. Architectural Doctrine

### 5.1 Build Around a Stable Core

Prefer architecture where stable logic is insulated from volatile infrastructure and delivery details.

Core logic should not depend on:

- HTTP frameworks
- UI frameworks
- shell/process concerns
- DB drivers
- serializer-specific transport types
- CLI formatting decisions

### 5.2 Dependency Direction Must Stay Clean

Outer layers may depend on inner layers.
Inner layers must not depend outward.

If the domain knows too much about transport, persistence, or presentation, the architecture is collapsing.

### 5.3 No God Constructs

Never create:

- god structs
- manager blobs
- context bags carrying half the system
- service locators
- global mutable hubs
- “engine” objects that secretly own unrelated concerns

A construct is too large when it:

- owns unrelated responsibilities
- knows too much about unrelated subsystems
- becomes required by every test
- mixes orchestration, validation, persistence, transport, and formatting
- attracts all future changes because no one knows where else they should go

### 5.4 Single Source of Truth

If something is canonical, define it once and project from there.

Examples:

- package identity
- domain invariants
- protocol semantics
- operation catalogs
- error classification systems
- config schema
- feature meaning

Do not duplicate authority across layers.

### 5.5 Separate Domain from Representation

Do not casually collapse:

- domain entities
- request/response DTOs
- storage rows/records
- UI payloads
- parser ASTs
- config structs

Serde convenience is not a license to erase boundaries.

### 5.6 Keep Abstractions Honest

An abstraction is justified when it:

- removes real duplication
- clarifies a boundary
- captures a real invariant
- narrows an API
- makes tests truer

If it mostly hides confusion, remove it.

---

## 6. Type-Driven Design

### 6.1 Types Are the Design

Strong types produce:

- clearer contracts
- fewer invalid states
- easier testing
- safer refactors
- smaller failure surfaces

Weak types produce:

- runtime checking everywhere
- stringly behavior
- boolean soup
- implicit contracts
- fragile APIs

### 6.2 Illegal States Must Be Unrepresentable

Never rely on:

- comments
- conventions
- “callers should”
- TODOs for future validation
- fragile discipline

Enforce invariants with:

- newtypes
- enums
- validated constructors
- visibility control
- state types
- carefully chosen trait bounds

### 6.3 Eliminate Primitive Obsession

Avoid raw primitives for meaningful domain concepts.

Bad:

- `String` for IDs, tokens, emails, route names, opaque user categories
- `u64` for every identifier
- `bool` for meaningful state transitions
- magic strings and sentinel integers

Prefer named meaning over raw storage.

### 6.4 Public APIs Must Stay Sharp

Public APIs must be:

- intentional
- narrow
- explicit
- stable by design, not accident

Do not leak internals through convenience re-exports unless the crate deliberately wants to own those surfaces.

### 6.5 Trait Discipline

Use traits when they model a real capability boundary, polymorphic contract, or substitution point.

Do not:

- introduce traits for imagined future flexibility
- over-genericize APIs until diagnostics become unreadable
- force dynamic dispatch where a concrete type or static dispatch is clearly better

Prefer static dispatch by default.
Use dynamic dispatch when heterogeneity or runtime composition genuinely requires it.

---

## 7. Ownership, Borrowing, and Mutation

### 7.1 Respect Ownership

The borrow checker is pressure toward better design.
Do not treat it as friction to hack around.

### 7.2 Do Not Clone to Silence Design Problems

A clone is acceptable when:

- ownership transfer is intentional
- the data is small and the tradeoff is clear
- a boundary needs ownership
- clarity improves materially

It is not acceptable as the default escape hatch for unclear ownership.

### 7.3 Prefer the Lightest Correct Ownership

Prefer:

- `&str` over `String` when ownership is unnecessary
- slices over owned collections when ownership is unnecessary
- concrete ownership only where it simplifies the model
- shared ownership only where it is truly needed

Do not reach for `Arc` and `Mutex` reflexively.

### 7.4 Make Mutation Local and Explicit

Keep mutable state constrained.
Avoid hidden mutation via sprawling shared state.
Prefer APIs where the mutation boundary is obvious.

---

## 8. Error Handling Doctrine

### 8.1 No Panic-Driven Production Logic

Do not use:

- `unwrap()`
- `expect()`
- panic as control flow
- panicky indexing
- hidden fallback behavior

Narrow exceptions may exist in:

- tests
- examples
- truly impossible internal invariants that are documented

### 8.2 Errors Must Carry Meaning

Good errors tell the reader:

- what failed
- where it failed
- why it failed, if knowable
- what context matters
- whether it is caller error, environmental failure, or internal fault

### 8.3 Use the Right Error Style for the Right Layer

Use typed errors for:

- libraries
- domain logic
- reusable components
- explicit protocol boundaries

Use anyhow-style aggregation for:

- binaries
- CLI entrypoints
- one-shot tasks
- tests
- developer tooling

Do not flatten everything into strings.
Do not erase useful source context.

---

## 9. Async and Concurrency Doctrine

### 9.1 Async Is a Tool, Not a Default Identity

Use async where it buys something:

- I/O concurrency
- long-lived services
- multiplexed workloads

Do not make code async when synchronous code is clearer and sufficient.

### 9.2 Tokio Is the Baseline Async Runtime

For modern Rust work, Tokio is the default runtime unless the project explicitly chose otherwise.

### 9.3 Structured Concurrency by Default

Every task should have:

- an owner
- shutdown semantics
- cancellation semantics
- error propagation behavior
- a clear reason to exist

Detached tasks require explicit justification and lifecycle management.

### 9.4 No Hidden Blocking

Do not accidentally block in async code.
If blocking is necessary, isolate it explicitly.

### 9.5 Avoid Locking as a Design Reflex

Use the narrowest synchronization primitive that solves the real problem.
Often the right answer is message passing, ownership transfer, or better partitioning.
`Arc<Mutex<T>>` is allowed when it is the right tradeoff, not as a reflex.

---

## 10. Data, Serialization, and External Boundaries

### 10.1 Serde Is the Default Serialization Layer

Use Serde when serialization or deserialization is needed.
Do not replace it casually.

### 10.2 Validate at the Edge

All external input is hostile until validated.

Validate early:

- HTTP input
- CLI input
- files
- config
- DB-loaded untrusted content
- plugin inputs
- RPC payloads
- FFI input
- deserialized data

Do not let unchecked data drift inward.

### 10.3 Compatibility Is an Explicit Design Decision

If data crosses process, machine, storage, or version boundaries, compatibility is part of the design.
Treat schema changes, field renames, and wire semantics as intentional product decisions.

---

## 11. Performance Doctrine

### 11.1 Default to Sensible Efficiency

Write code that is naturally efficient:

- avoid unnecessary allocation
- avoid unnecessary copying
- choose sane data structures
- keep hot paths simple
- avoid hidden temporary work

### 11.2 Optimize in the Right Order

Order of concern:

1. correctness
2. asymptotic behavior
3. obvious waste
4. measured hot-path tuning
5. low-level tricks only if justified

### 11.3 Measure Before Cleverness

Do not introduce complexity for hypothetical speed.
If you add non-obvious performance machinery, have evidence or a hard systems reason.

---

## 12. Unsafe Rust and FFI

### 12.1 Unsafe Is a Last Resort

Use safe Rust unless there is a real reason not to.

### 12.2 Unsafe Must Be Small and Explained

Every unsafe block must document:

- required invariants
- why they hold here
- why safe Rust was insufficient
- what future change could invalidate the argument

Prefer a `// SAFETY:` comment immediately above or within the block, depending on repository convention.

### 12.3 FFI Requires Extra Discipline

FFI boundaries must make ownership, layout, nullability, threading, lifetime, and error semantics explicit.
Treat FFI as hostile terrain.

---

## 13. Testing Doctrine

### 13.1 Tests Are Part of the Work

If behavior changed, inspect whether tests should change too.
If public behavior changed and tests did not, assume incompleteness until proven otherwise.

### 13.2 Test the Contract, Not the Accident

Test:

- invariants
- observable behavior
- failure modes
- boundary behavior
- cross-layer parity when relevant

Do not over-bind tests to incidental implementation details.

### 13.3 Use the Right Test Mix

Use:

- unit tests for local logic
- integration tests for boundary behavior
- property tests where invariants matter
- snapshot tests only when they add clarity
- fuzzing for hostile-input surfaces when appropriate

### 13.4 Determinism Is Mandatory

Control:

- time
- randomness
- temp paths
- environment
- process-global state
- network assumptions
- concurrency timing where practical

Flaky tests are design failures.

### 13.5 Keep Test Seams Honest

Do not widen public APIs just to make tests convenient.
Prefer crate-private helpers, focused builders, local seams, and boundary-driven tests.

---

## 14. Verification and Self-Correction Loop

Before considering work complete, run the strongest relevant verification the project supports.

Universal default:

```bash
cargo fmt --all
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all
```

Also run when relevant:

```bash
cargo check --all-targets --all-features
cargo doc --all-features
cargo test --all-features
cargo test -- --ignored
cargo audit
cargo deny check
cargo udeps
cargo llvm-cov
cargo fuzz test <target>
```

Rules:

- warnings are failures
- skipped checks need an explicit reason
- ignored tests are not permission to avoid validating critical behavior
- if the repository has a stronger canonical check command, use it

When a check fails:

1. read the actual failure output
2. fix the underlying cause
3. rerun the narrowest relevant check
4. rerun the full required verification before declaring completion

---

## 15. Documentation and Observability

### 15.1 Document Public APIs Intentionally

Rustdoc should explain where relevant:

- purpose
- invariants
- usage
- examples
- `# Errors`
- `# Panics`
- `# Safety`

Do not weaken documentation enforcement to avoid doing the work.

### 15.2 Use Structured Observability

Prefer tracing spans and structured fields over ad hoc string dumping.

Rules:

- no secret leakage
- no leftover println debugging
- no noisy spam logs
- enough context to debug ownership, latency, retries, and failure propagation

---

## 16. Security Doctrine

Assume:

- all input is hostile
- all boundaries are pressure points
- convenience will create vulnerabilities if left unchecked

Always:

- validate input
- constrain authority
- reject malformed data clearly
- avoid path traversal
- avoid injection surfaces
- avoid accidental secret exposure
- keep authn/authz explicit
- fail safely

Parsers, plugin systems, loaders, deserializers, RPC boundaries, and FFI edges deserve extra suspicion.

---

## 17. Refactoring Doctrine

### 17.1 Hard Refactors Are Allowed

Do not preserve weak design purely to avoid breakage.
If a better design requires a hard refactor and compatibility is not explicitly required, do the better thing.

### 17.2 Breakage Must Be Coherent

If you break something:

- break it intentionally
- update all affected layers
- update tests
- update docs
- remove dead compatibility scaffolding
- leave the system cleaner than before

Half-migrations are worse than hard breaks.

### 17.3 Delete Dead Weight

Delete:

- obsolete code
- duplicate code
- stale flags
- fake abstractions
- cargo-cult layers
- stale TODO-shaped structural debt when the task reasonably allows cleanup

---

## 18. Rust-Backed UI Projects and Tauri

If the project uses Tauri or another Rust-backed UI stack:

- Rust owns the real logic
- the UI is a projection layer
- frontend validation is UX, not trust
- important invariants remain enforced in Rust
- command APIs are strict boundaries
- filesystem, shell, and network access are security-sensitive capabilities
- command surfaces stay narrow and explicit

Do not move the real rules into the frontend because it feels convenient.

---

## 19. Smell Radar

Stop and refactor when you see:

- god structs
- god files
- manager blobs
- context bags
- dumping-ground modules
- bool parameter soup
- stringly typed domains
- duplicated parsing or validation
- panicky indexing
- hidden mutable global state
- traits introduced for imagined reuse
- one type serving domain, storage, wire, and UI concerns without justification
- tests that require constructing half the system to validate a tiny behavior
- code that works but cannot explain its own boundaries

These are structural debt signals, not harmless quirks.

---

## 20. Completion Bar

A task is complete only when all of the following are true:

- repository state was inspected before design
- a design note was produced when the task was non-trivial
- the project and module shape still makes sense
- invariants are encoded in types where practical
- boundaries are clear
- no new god constructs or dumping grounds were introduced
- errors are intentional and contextual
- tests prove the changed behavior
- docs were updated where public behavior changed
- formatting is clean
- clippy is clean
- verification passes
- no obvious structural debt was knowingly left behind out of convenience

---

## 21. Final Rule

Produce Rust where correctness is mechanically checkable through:

- explicit types
- explicit boundaries
- explicit errors
- explicit tests
- passing verification

If the code relies on implicit context, hidden assumptions, or opaque control flow to be considered “correct,” it is not ready.
