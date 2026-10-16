# Bounded fuzzing

Install the cargo-fuzz version and nightly in [contributor-tools.toml](../tools/contributor-tools.toml), with nightly `rust-src`. Use a fresh install root outside the checkout:

```sh
python3 -m tools.htmlcut_maintenance install cargo-fuzz --root /outside/checkout/cargo-fuzz
python3 -m tools.htmlcut_maintenance qa fuzz
python3 -m tools.htmlcut_maintenance qa fuzz --runs 250 --require-mutations /outside/checkout/fuzz-results
```

Add the install root's `bin` to PATH. The command builds both targets discovered from this manifest, prepares exact independent corpus inputs, then runs each harness sequentially through the shared bounded process owner. It checks complete iteration counts and post-initialization mutation progress for full smoke and explicitly requested short runs. The default full smoke is 10,000 runs per target; short runs state their scope.

Inputs are at most 64 KiB, including the two-byte contract-length framing used by `extract`. This accommodates the literal stress cases. Seed preparation rejects nonregular/oversized sources before opening/building, refuses excessive or empty discovery and records exact seed identities. `contract_bytes` probes closed contract admission; `extract` probes incremental parser refusal and extraction/outline under bounded reading/output policy and one operation context.

Owned build roots are outside the checkout. Build allowance is 1 GiB sampled disk, 600 seconds and a 4 MiB log; each target permits 64 MiB sampled output, 180 seconds and a 4 MiB log. LibFuzzer also has ten-second per-input and 2 GiB RSS limits. Sampling is not a filesystem quota. Existing Cargo/rustup state is outside those transient allowances and is never pruned. Confirmed successful default runs remove temporary output; explicit output roots and failures retain evidence in a new child directory. Unconfirmed cleanup retains affected products and fails.

See [QA boundaries](../docs/qa-resources.md). A short real integration or mocked runner test is not the full required fuzz or mutation review.
