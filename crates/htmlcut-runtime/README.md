# htmlcut-runtime

The shared local application boundary for HTMLCut frontends. `Job` accepts typed extraction, outline and schema requests. It owns preflight, local acquisition, complete exports, stream delivery and policy-preserving named publication. The CLI uses this facade; graphical hosts can use the same operations without command parsing or a process watchdog.

`PreparedDocument` is immutable, `Send + Sync`, and holds semantic state only. Each job receives fresh policy, cancellation and an absolute deadline. `Computed` retains owned values or bounded outline facts; `Job::save` re-admits canonical output under the Save job's limits. No engine document, compiled contract or DOM handle is exposed through this facade.

See [the headless example](examples/headless.rs). Library deadlines and cancellation are cooperative: blocking OS/upstream calls are not forcibly interrupted. Only the disposable CLI owns hard enforcement and process exit. A commit claim excludes cancellation; the actual rename result wins even past the deadline. Extraordinary blocked filesystem calls or hard termination can leave staging residue. Publication does not promise fsync durability or prevent final namespace races.

Original HTMLCut code is MPL-2.0. Dependencies retain their notices and licenses in the supplied legal files.
