# Extraction-contract evaluation

The [contract matrix](contract-matrix/matrix.py) contains the handoff's original
synthetic snapshots, version-7 and version-8 plans, SHA-256 manifest and independent
expected values. The supplied pilot reports `baseline.json` and `warm.json` describe
the handoff environment; they are historical evidence, not current candidate results.
New execution reports are retained under `contract-matrix/results`.

```sh
python3 -m venv /path/outside/repository/evaluation-python
/path/outside/repository/evaluation-python/bin/python -m pip install -r evaluation/requirements.txt
/path/outside/repository/evaluation-python/bin/python evaluation/contract-matrix/matrix.py --binary /absolute/path/htmlcut-v21 --plan-version 7 --repeats 15 --output /path/v21.json
/path/outside/repository/evaluation-python/bin/python evaluation/contract-matrix/matrix.py --binary /absolute/path/htmlcut-candidate --plan-version 8 --repeats 15 --output /path/candidate.json
```

The runner validates all complete values and intended refusals, uses strict root/
field cardinalities in the Python references and records binary/script identities,
versions, commands and raw timings. The article comparison normalizes whitespace
while checking complete ordered text. A guard/count failure must produce no stdout.
The browser test runs offline JavaScript in Chromium if Playwright is available;
reader/converter and jq checks describe their different appropriate tasks.
A missing tool is reported as unexecuted, not silently passed.

Fresh CLI launches include process startup. `warm.py` measures preloaded Python
parse/extract separately; Rust reusable document/plan examples measure core work.
`task-economics.py` measures the maintained small corpus and named tokenizer sizes;
`captured-tasks.py` accepts separately frozen caller captures and comparisons. Native
resource evidence uses real kernel boundaries, not parser mocks or exit-0-only checks.
See [independent design QA](design-qa/README.md) and the implementation report for
actual native platforms, exact commands, complete failures and unresolved gates.

Output bytes, source bytes and plan/help sizes are distinct from model tokens.
Named tokenizer counts are proxies, not billing or a blind agent study. Equal
compact JSON yields equal output tokens for the same tokenizer. No universal speed,
market-demand or commercial-value claim follows from these synthetic tasks.
Historical [v15 results](v15-results.md) retain their original scope and attribution.
