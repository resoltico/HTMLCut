<!--
AFAD:
  afad: "4.0"
  version: "19.0.0"
  domain: EVALUATION
  updated: "2026-10-05"
RETRIEVAL_HINTS:
  keywords: [fidelity, corpus, task economics, tokenizer]
  questions: ["How is extraction correctness and task cost measured?"]
-->

# Extraction-contract evaluation

The offline corpus is original synthetic HTML authored for HTMLCut under the repository MPL-2.0 license. Its manifest records immutable byte hashes, origin and scope. It covers technical linked identifiers, hidden content, lists/tables/preformatted content, alternative text, Unicode, malformed HTML and caller-supplied rendered DOM without executing JavaScript.

Task economics are correctness-qualified measurements. Bare data, one-time description/inspection/plan costs and opt-in receipt/bundle costs are measured separately. Named tokenizer counts are proxies, not agent billing or measured model reasoning. Fresh-process timing and prepared in-process reuse are separate. Domain mapping remains caller code. No universal performance or market-adoption claim is implied.

The maintained benchmark script records commands, fixture hashes, tool versions, complete validated values, observed retries and raw timing samples. Final-candidate results belong in the implementation evidence summary after actual execution; a configured benchmark is not a result.

Install the current evaluation dependencies from [requirements.txt](requirements.txt) into a
caller-owned Python environment. The report records actual installed versions and a named
tokenizer; it does not silently reuse a historical measurement after a dependency/source change.


For a clean checkout, create a caller-owned environment and install the pinned evaluation inputs:

```bash
python3 -m venv /path/outside/repository/evaluation-python
/path/outside/repository/evaluation-python/bin/python -m pip install -r evaluation/requirements.txt
/path/outside/repository/evaluation-python/bin/python evaluation/task-economics.py --binary /path/to/htmlcut --output /path/outside/repository/offline.json
```

`captured-tasks.py` requires a frozen caller-owned capture directory containing catalogue,
product-details, Hacker News, Wiki population-table Unicode cells, Python pathlib and rendered/unrendered quote inputs, with
`capture-manifest.json` hashes and complete `browser-quotes.json` values. Browser rendering and
network acquisition happen before evaluation; the runner never fetches a page. Supply verified
native current/baseline binaries, their source/binary binding record, installed htmlq/pup paths,
and a captured release API JSON response. `--help` lists the required paths.

The public suite compares complete direct records and caller shortlists, scalar titles,
news, product fields, guarded price, protected technical code, reading output, inline JSON and
rendered quotes; it checks input-independent moved-bundle replay and inspection size. Fresh
processes have three warmups and fifteen randomized checked launches. RSS uses separate checked
launches and retained native time output. Python warm parse/map and reused-document mapping use
cached input bytes; they do not include file I/O. Title tools share one randomized trial order.
The code reports versioned dependencies, actual binary/input hashes and canonical requested
values. Optional receipts and setup descriptions/schema costs remain separate from data.

Heuristic reading output and already structured API/JSON inputs have their own scope: HTMLCut
is unnecessary for the latter. Browser measurements identify their automation/acquisition scope.
These controls qualify the specified snapshots; they are not a browser visibility model, general
HTML conformance study, market adoption result, or measurement of billed agent reasoning.
