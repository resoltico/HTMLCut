# Dependency responsibilities

HTMLCut uses the maintained, unmodified published `scraper` stack for HTML5 parsing,
DOM storage and CSS matching. Cargo.toml declares supported dependency ranges;
Cargo.lock pins reproducible builds. Scraper's optional CLI and unused parser-error
collection features are disabled; repaired HTML is still parsed by upstream. The graph contains no privately customized
html5ever, markup5ever, selectors, tendril or servo_arc packages and no patch paths.

Product code owns the closed extraction plan, logical admission, cardinalities,
contextual guards, row-relative fields, bounded text/HTML projection and result
encoding. Parser and selector engine internals remain upstream responsibilities.
Logical limits do not claim callback-by-callback upstream interruption.
The CLI worker supplies the [physical resource boundary](architecture.md).

Upstream pointer checks from the removed forks are not claims that unmodified
upstream code passes strict Miri. Worker isolation confines parser/selector crashes
to the child, and the supervisor refuses incomplete or abnormal responses.
In-process Rust callers must isolate adversarial work themselves.

There is one core package and one CLI package, with libFuzzer harnesses for owned
contracts and historical regressions. Nightly remains necessary for libFuzzer
instrumentation; normal builds/tests use stable. Native packages derive complete
attribution from the actual locked target graph and compiler runtime. ISC-adapted
projection/serialization portions retain their attribution in the core package.
The [release protocol](release-protocol.md) requires clean downstream compilation
and native package execution before publication.
