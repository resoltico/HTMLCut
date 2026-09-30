# SHA-256 ARM64 provenance correction

Upstream: RustCrypto/hashes sha2 0.11.0, registry checksum recorded in the pre-change Cargo.lock; source, MIT/Apache licensing and authorship retained. Public crate API and hash semantics are unchanged.

The ARM64 SHA-256 hardware backend loads four u32 round constants through NEON. Its original pointer originated from a shared reference to one u32, giving a four-byte borrow range for a sixteen-byte load. Miri strict provenance detected this in snapshot hashing on nightly-2026-08-25. Each load now takes a pointer from the complete four-element constant slice; no algorithm or backend is bypassed, and Miri flags remain strict.

Scope: eight round-constant load sites in src/sha256/aarch64_sha2.rs. State and message-block loads already use complete slices. Independent identity vectors and SHA-256 known-answer tests remain required, along with hardware-path Miri and native package verification. This fork travels as a direct core dependency, rather than a workspace-only override.
