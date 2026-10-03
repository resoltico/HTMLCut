<!--
AFAD:
  afad: "4.0"
  version: "17.0.0"
  domain: EVALUATION
  updated: "2026-10-03"
RETRIEVAL_HINTS:
  keywords: [fidelity, corpus, task economics, tokenizer]
  questions: ["How is extraction correctness and task cost measured?"]
-->

# Extraction-contract evaluation

The offline corpus is original synthetic HTML authored for HTMLCut under the repository MIT license. Its manifest records immutable byte hashes, origin and scope. It covers technical linked identifiers, hidden content, lists/tables/preformatted content, alternative text, Unicode, malformed HTML and caller-supplied rendered DOM without executing JavaScript.

Task economics are correctness-qualified measurements. Bare data, one-time description/inspection/plan costs and opt-in receipt/bundle costs are measured separately. Named tokenizer counts are proxies, not agent billing or measured model reasoning. Fresh-process timing and prepared in-process reuse are separate. Domain mapping remains caller code. No universal performance or market-adoption claim is implied.

The maintained benchmark script records commands, fixture hashes, tool versions, complete validated values, observed retries and raw timing samples. Final-candidate results belong in the implementation evidence summary after actual execution; a configured benchmark is not a result.

Install the current evaluation dependencies from [requirements.txt](requirements.txt) into a
caller-owned Python environment. The report records actual installed versions and a named
tokenizer; it does not silently reuse a historical measurement after a dependency/source change.
