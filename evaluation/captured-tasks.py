# SPDX-License-Identifier: MPL-2.0
"""Source-bound public snapshot correctness, task economics and process measurements."""
import argparse
import collections
import hashlib
import json
import subprocess
import re
import shutil
import importlib.metadata
import platform
import tomllib
import sys
import tiktoken
from pathlib import Path
from urllib.parse import urljoin, urlsplit, urlunsplit

from measurements import paired, warm
from capture_reference import reference, from_document, reading_signature, technical_node, text as literal_text
from bs4 import BeautifulSoup
from lxml import html as lxml_html
from markdown_it import MarkdownIt
import trafilatura

ROOT = Path(__file__).resolve().parents[1]
parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument("--binary", type=Path, required=True)
parser.add_argument("--source-sha", required=True)
parser.add_argument("--captures", type=Path, required=True)
parser.add_argument("--baseline-binary", type=Path, required=True)
parser.add_argument("--comparators", type=Path, required=True)
parser.add_argument("--binding", type=Path, required=True)
parser.add_argument("--release-json", type=Path, required=True)
parser.add_argument("--output", type=Path, required=True)
parser.add_argument("--baseline-plans", type=Path, required=True)
parser.add_argument("--baseline-binding", type=Path, required=True)
parser.add_argument("--browser-source", type=Path, required=True)
parser.add_argument("--browser-evidence", type=Path, required=True)
parser.add_argument("--quote-values", type=Path, required=True)
parser.add_argument("--quote-baseline-plan", type=Path, required=True)
evaluation_options = parser.parse_args()
BINARY = evaluation_options.binary.resolve()
CAPTURES = evaluation_options.captures.resolve()
OUTPUT = evaluation_options.output.resolve()
if OUTPUT.is_relative_to(ROOT):
    parser.error("Execution evidence must be outside the repository")
OUTPUT.mkdir(parents=True, exist_ok=True)
binding = json.loads(evaluation_options.binding.read_text())
assert binding["source_commit"] == evaluation_options.source_sha
assert subprocess.check_output(["git", "rev-parse", "HEAD"], cwd=ROOT).decode().strip() == evaluation_options.source_sha
assert subprocess.check_output(["git", "rev-parse", "HEAD^{tree}"], cwd=ROOT).decode().strip() == binding["source_tree"]
assert not subprocess.check_output(["git", "status", "--porcelain"], cwd=ROOT), "Candidate evaluation requires clean bound source"
assert binding["binary_sha256"] == hashlib.sha256(BINARY.read_bytes()).hexdigest()
assert subprocess.check_output([str(BINARY), "--version"]).decode().strip() == "htmlcut " + tomllib.loads((ROOT / "Cargo.toml").read_text())["workspace"]["package"]["version"]
assert subprocess.check_output([str(evaluation_options.baseline_binary.resolve()), "--version"]).decode().strip() == "htmlcut 19.2.0"
baseline_binding = json.loads(evaluation_options.baseline_binding.read_text())
assert baseline_binding["source_commit"] == subprocess.check_output(["git", "rev-parse", "v19.2.0^{commit}"], cwd=ROOT).decode().strip()
assert baseline_binding["htmlcut"]["sha256"] == hashlib.sha256(evaluation_options.baseline_binary.read_bytes()).hexdigest()

manifest = json.loads((CAPTURES / "capture-manifest.json").read_text())
for entry in manifest["rows"]:
    if "sha256" in entry:
        assert hashlib.sha256((CAPTURES / entry["file"]).read_bytes()).hexdigest() == entry["sha256"]
rows = []
encoder = tiktoken.get_encoding("o200k_base")


def resolved_web_url(base, value):
    # RFC3986 6.2.3: an HTTP(S) authority's empty path normalizes to '/'.
    # This fixture oracle covers its observed ASCII HTTP(S)/relative URLs.
    parts = urlsplit(urljoin(base, value))
    assert parts.scheme in ("http", "https")
    return urlunsplit(parts._replace(path=parts.path or "/"))


def source(name):
    return (CAPTURES / name).read_bytes().decode("utf-8")


def soup(name):
    return BeautifulSoup(source(name), "lxml")


def value_plan(select, read=None, fields=None, expect=None):
    plan = dict(version=6, select=select, match="all", min=1)
    if fields is not None:
        plan["fields"] = fields
    elif read is not None:
        plan["read"] = read
    if expect:
        plan["expect"] = expect
    return plan


def execute(name, file, plan, expected, base=None, inline=None):
    path = OUTPUT / (name + ".plan.json")
    path.write_text(json.dumps(plan, ensure_ascii=False, separators=(",", ":")))
    command = [str(BINARY), "extract", "--file", str(CAPTURES / file), "--plan", str(path)]
    if base:
        command += ["--base-url", base]
    alternative = [sys.executable, str(ROOT / "evaluation/capture_reference.py"), "--task", name,
                   "--fixture", str(CAPTURES / file)]
    historical_plan = (evaluation_options.quote_baseline_plan if name == "rendered-quote-records"
                       else evaluation_options.baseline_plans / (name + ".plan.json"))
    historical_command = [str(evaluation_options.baseline_binary.resolve()), "extract", "--file", str(CAPTURES / file), "--plan", str(historical_plan)]
    if base:
        historical_command += ["--base-url", base]
    if not historical_plan.is_file():
        raise ValueError(f"Missing explicit historical task plan: {historical_plan}")
    bs4_command = [sys.executable, str(ROOT / "evaluation/capture_reference_bs4.py"), name, str(CAPTURES / file)]
    commands = dict(htmlcut=command, baseline=historical_command, python=alternative, bs4=bs4_command)
    if inline is not None:
        inline_command = [str(BINARY), "extract", "--file", str(CAPTURES / file), *inline]
        if base:
            inline_command += ["--base-url", base]
        commands["htmlcut-inline"] = inline_command
    measured, outputs = paired(commands, expected,
                               lambda tool, data: json.loads(data), rss_directory=OUTPUT / "rss" / name)
    raw = outputs["htmlcut"]
    if inline is not None:
        assert outputs["htmlcut-inline"] == raw
    answer = json.loads(raw)
    assert answer == expected, name
    canonical = (json.dumps(expected, ensure_ascii=False, sort_keys=True, separators=(",", ":")) + "\n").encode()
    assert raw == canonical, (name, "mandatory output overhead or wrong canonical encoding")
    (OUTPUT / (name + ".stdout.json")).write_bytes(raw)
    rows.append(dict(task=name, source=str(file), plan=plan, command=command, commands=commands,
                     baseline_plan_sha256=hashlib.sha256(historical_plan.read_bytes()).hexdigest(),
                     correctness="complete independent equality", values=answer,
                     output_bytes=len(raw), output_tokens=len(encoder.encode(raw.decode())),
                     canonical_requested_tokens=len(encoder.encode(canonical.decode())), mandatory_envelope_tokens=0,
                     plan_tokens=len(encoder.encode(path.read_text())), timings=measured,
                     reference_command=alternative, reference_script_tokens=len(encoder.encode((ROOT / "evaluation/capture_reference.py").read_text()))))
    candidate_request = json.dumps(plan, ensure_ascii=False, sort_keys=True, separators=(",", ":"))
    historical_request = json.dumps(json.loads(historical_plan.read_text()), ensure_ascii=False, sort_keys=True, separators=(",", ":"))
    request_tokens = dict(candidate=len(encoder.encode(candidate_request)), baseline=len(encoder.encode(historical_request)))
    rows[-1]["request_tokens"] = request_tokens
    if name in ("catalogue-records", "news-score-records", "rendered-quote-records"):
        assert request_tokens["candidate"] < request_tokens["baseline"], (name, request_tokens)
    if inline is not None:
        import shlex
        rows[-1]["inline_authoring"] = dict(command=inline_command,
            tokens=len(encoder.encode(shlex.join(inline_command))),
            scope="Invocation tokens recorded separately from plan JSON and optional setup/evidence; absolute paths are part of this concrete command")
    content = source(file)
    rows[-1]["python_warm_parse_map"] = warm(lambda: reference(name, content), expected)
    document = lxml_html.fromstring(content)
    rows[-1]["python_reused_document_map"] = warm(lambda: from_document(name, document), expected)
    receipt = OUTPUT / (name + ".receipt.json")
    evidence_command = command + ["--receipt", str(receipt), "--overwrite"]
    assert subprocess.check_output(evidence_command) == raw
    receipt_bytes = receipt.read_bytes()
    receipt_value = json.loads(receipt_bytes)
    assert receipt_value["data_sha256"] == hashlib.sha256(raw[:-1]).hexdigest()
    rows[-1]["receipt"] = dict(command=evidence_command, bytes=len(receipt_bytes), tokens=len(encoder.encode(receipt_bytes.decode())), optional=True)

    return answer


catalogue = reference("catalogue-records", source("books.html"))
fields = dict(title=dict(select="h3 a", read="attr:title"),
              price=dict(select=".price_color"), stock=dict(select=".availability"),
              rating=dict(select=".star-rating", read="attr:class"),
              url=dict(select="h3 a", read="url:href"))
actual = execute("catalogue-records", "books.html", value_plan("article.product_pod", fields=fields),
                 catalogue, "https://books.toscrape.com/", inline=[
                     "--select", "article.product_pod", "--all",
                     "--field", "title", "h3 a", "attr:title",
                     "--field", "price", ".price_color", "text",
                     "--field", "stock", ".availability", "text",
                     "--field", "rating", ".star-rating", "attr:class",
                     "--field", "url", "h3 a", "url:href"])
ratings = dict(One=1, Two=2, Three=3, Four=4, Five=5)
shortlist = lambda data: [dict(book, stock=book["stock"].strip(), rating=ratings[book["rating"].split()[-1]])
                         for book in data if float(book["price"][1:]) <= 25 and ratings[book["rating"].split()[-1]] >= 3]
assert shortlist(actual) == shortlist(catalogue)
rows[-1]["shortlist"] = shortlist(actual)
rows[-1]["caller_mapping"] = "Primitive price/rating/stock mapping only; no caller HTML reparsing"

titles = reference("catalogue-titles", source("books.html"))
execute("catalogue-titles", "books.html", value_plan("h3 a", "attr:title"), titles, inline=["--select", "h3 a", "--all", "--read", "attr:title"])
title_command = rows[-1]["command"]
reference_command = rows[-1]["reference_command"]
title_commands = {
    "htmlcut": title_command,
    "python": reference_command,
    "htmlq": [str(evaluation_options.comparators / "htmlq"), "--filename", str(CAPTURES / "books.html"), "--attribute", "title", "h3 a"],
    "pup": [str(evaluation_options.comparators / "pup"), "--file", str(CAPTURES / "books.html"), "--plain", "h3 a attr{title}"],
}
measurements, outputs = paired(title_commands, titles,
    lambda name, data: data.decode().splitlines() if name in ("htmlq", "pup") else json.loads(data),
    rss_directory=OUTPUT / "rss/paired-title-tools")
rows.append(dict(task="paired-catalogue-title-tools", commands=title_commands, timings=measurements,
                 output_tokens={name: len(encoder.encode(data.decode())) for name, data in outputs.items()},
                 correctness="all four tools compared in each randomized trial on the same frozen file"))

news = reference("news-records", source("news.html"))
execute("news-records", "news.html", value_plan(".titleline", fields=dict(
    title=dict(select=":scope > a", read="literal"),
    url=dict(select=":scope > a", read="url:href"))), news, "https://news.ycombinator.com/")
score_plan = value_plan("tr.athing", fields=dict(
    title=dict(select=".titleline > a", read="literal"),
    url=dict(select=".titleline > a", read="url:href"),
    score=dict(select=":scope + tr .score", match="optional")))
score_plan["following_siblings"] = 1
execute("news-score-records", "news.html", score_plan,
        reference("news-score-records", source("news.html")), "https://news.ycombinator.com/")
wiki_capture = next(row for row in manifest["rows"] if row["file"] == "wiki-countries.html")
rows.append(dict(task="public-wikipedia-acquisition", acquisition=wiki_capture,
                 correctness="not evaluated: frozen acquisition failed", htmlcut_success=False))
details = reference("product-details", source("book-details.html"))
execute("product-details", "book-details.html", value_plan(".table.table-striped tr", fields=dict(
    label=dict(select="th", read="literal"), value=dict(select="td", read="literal"))), details)
price = reference("guarded-product-price", source("book-details.html"))[0]
price_plan = dict(version=6, select=".product_main .price_color", read="literal", expect=[
    dict(select=".product_main h1", read="literal", equals="A Light in the Attic")])
execute("guarded-product-price", "book-details.html", price_plan, [price])

css = 'dl:has(> dt[id="pathlib.PurePath.full_match"])'
technical = soup("python-pathlib.html").select_one(css)
execute("technical-literal", "python-pathlib.html", value_plan(css, "literal"), [technical.get_text()])
plan = value_plan(css, "markdown")
path = OUTPUT / "technical-markdown.plan.json"
path.write_text(json.dumps(plan))
result = subprocess.run([str(BINARY), "extract", "--file", str(CAPTURES / "python-pathlib.html"), "--plan", str(path)], check=True, capture_output=True)
markdown = json.loads(result.stdout)[0]
tokens = MarkdownIt("commonmark").parse(markdown)
code = [token.content for token in tokens if token.type == "fence"]
assert code == [pre.get_text() + "\n" for pre in technical.select("pre")]
plain = " ".join(token.content for token in tokens if token.type == "fence") + " " + " ".join(child.content for token in tokens for child in (token.children or []) if child.type in ("text", "code_inline"))
expected_words = collections.Counter(re.findall(r"\w+", technical.get_text()))
actual_words = collections.Counter(re.findall(r"\w+", plain))
assert not (expected_words - actual_words), ("technical retained words missing", expected_words - actual_words)
reading = trafilatura.extract(str(technical), output_format="markdown", include_links=True, include_tables=True)
expected_reading = reading_signature(lxml_html.fromstring(str(technical)), source=True)
actual_reading = reading_signature(lxml_html.fragment_fromstring(MarkdownIt("commonmark").render(markdown), create_parent=True))
assert actual_reading == expected_reading, "Technical reading changed ordered characters, roles, links or code"
comparator_reading = reading_signature(lxml_html.fragment_fromstring(MarkdownIt("commonmark").render(reading or ""), create_parent=True))
rows.append(dict(task="technical-markdown", markdown=markdown, code_payloads=code,
                 correctness="complete ordered prose characters/inline roles/links and exact code blocks; ASCII prose spacing not scored",
                 semantic_atoms=actual_reading, comparator_semantics_match=comparator_reading == expected_reading,
                 source_word_occurrences=sum(expected_words.values()),
                 comparator="trafilatura 2.3.0", comparator_markdown=reading,
                 scope="Declared selected subsection versus heuristic reading of that same subsection; formatting conventions differ"))

browser = json.loads(evaluation_options.quote_values.read_text())
quote_plan = value_plan(".quote", fields=dict(
    text=dict(select=".text"), author=dict(select=".author"),
    tags=dict(select=".tag", match="all", min=0)))
quote_plan.update(min=10, max=10)
execute("rendered-quote-records", evaluation_options.browser_source.resolve(), quote_plan, browser)
raw = source("quotes-unrendered.html")
inline = raw.split("var data = ", 1)[1].split(";\n    for", 1)[0]
mapped = [dict(text=q["text"], author=q["author"]["name"], tags=q["tags"]) for q in json.loads(inline)]
assert mapped == browser
rows.append(dict(task="exact-inline-source", candidate_supported=False,
                 alternative="Caller-owned explicit string boundary and JSON decoding",
                 exact_initializer_sha256=hashlib.sha256(inline.encode()).hexdigest(),
                 values=mapped, correctness="Complete direct-JSON values equal independent rendered records"))
result = subprocess.run([str(BINARY), "extract", "--file", str(CAPTURES / "quotes-unrendered.html"), "--select", ".quote"], capture_output=True)
assert result.returncode == 3 and not result.stdout
rows.append(dict(task="unrendered-javascript", expected_boundary="no script execution", exit_code=result.returncode))

# Use disposable actual input files, remove them, then move and replay the real public catalogue bundle.
original_source = OUTPUT / "replay-input.html"
original_plan = OUTPUT / "replay-input.plan.json"
original_source.write_bytes((CAPTURES / "books.html").read_bytes())
original_plan.write_bytes((OUTPUT / "catalogue-records.plan.json").read_bytes())
bundle = OUTPUT / "catalogue.htmlcut.tar"
result = subprocess.run([str(BINARY), "extract", "--file", str(original_source), "--plan", str(original_plan), "--base-url", "https://books.toscrape.com/", "--bundle", str(bundle), "--overwrite"], check=True, capture_output=True)
original_source.unlink()
original_plan.unlink()
moved = OUTPUT / "moved-catalogue.htmlcut.tar"
bundle.replace(moved)
replay = subprocess.run([str(BINARY), "replay", str(moved)], check=True, capture_output=True)
assert replay.stdout == result.stdout
rows.append(dict(task="moved-public-catalogue-replay", all_values_equal=True, original_inputs_deleted=True, original_immutable_capture_retained=True, bundle_bytes=moved.stat().st_size))
inspections = []
for version, executable, select_flag in [("19.2.0", evaluation_options.baseline_binary.resolve(), "--css"), ("20.0.0", BINARY, "--select")]:
    command = [str(executable), "inspect", "--file", str(CAPTURES / "books.html"), select_flag, "article.product_pod", "--samples", "3"]
    result = subprocess.run(command, check=True, capture_output=True)
    (OUTPUT / (str(version) + "-inspection.json")).write_bytes(result.stdout)
    answer = json.loads(result.stdout)
    assert answer["count"] == 20 and len(answer["samples"]) == 3 and not answer["samples_complete"]
    assert not any(field in result.stdout.decode() for field in ("cursor", "handle", "propose"))
    inspections.append(dict(version=version, binary_sha256=hashlib.sha256(executable.read_bytes()).hexdigest(),
                            command=command, bytes=len(result.stdout), tokens=len(encoder.encode(result.stdout.decode()))))
rows.append(dict(task="inspection-size", measurements=inspections, scope="Same immutable catalogue and same complete count/three requested row samples in published v19.2 and current v20 candidate; token counts are proxies, with no assumed reduction"))
release_json = evaluation_options.release_json.resolve()
release_values = reference("release-assets", release_json.read_bytes().decode("utf-8"))
api_commands = {
    "python-json": [sys.executable, str(ROOT / "evaluation/capture_reference.py"), "--task", "release-assets", "--fixture", str(release_json)],
    "jq": ["jq", "-c", "-S", "[.assets[] | {name,size,url}]", str(release_json)],
}
api_timings, api_outputs = paired(api_commands, release_values, lambda tool, data: json.loads(data), rss_directory=OUTPUT / "rss/release-assets")
rows.append(dict(task="release-assets", values=release_values, commands=api_commands, timings=api_timings,
                 input_sha256=hashlib.sha256(release_json.read_bytes()).hexdigest(),
                 htmlcut_used=False, reason="The caller API already provides structured JSON; no HTML extraction is needed"))
# Inspect setup artifacts only when actually requested; their proxy cost remains separate from data.
setup = {}
for name, command in [("index", [str(BINARY), "--help"]), ("extract", [str(BINARY), "extract", "--help"]),
                      ("plan-schema", [str(BINARY), "schema", "htmlcut.extraction.plan"])]:
    data = subprocess.check_output(command)
    (OUTPUT / (name + ".json")).write_bytes(data)
    setup[name] = dict(command=command, bytes=len(data), tokens=len(encoder.encode(data.decode())))
report = dict(binary_sha256=hashlib.sha256(BINARY.read_bytes()).hexdigest(),
              source=evaluation_options.source_sha, rows=rows, binding=binding,
              baseline_binding=baseline_binding,
              browser_inputs={name: dict(path=str(path),sha256=hashlib.sha256(path.read_bytes()).hexdigest()) for name,path in (("fragment",evaluation_options.browser_source),("method",evaluation_options.browser_evidence),("values",evaluation_options.quote_values))}, captures=manifest, setup=setup,
              evaluator_sources={name: hashlib.sha256((ROOT / "evaluation" / name).read_bytes()).hexdigest()
                  for name in ("captured-tasks.py", "capture_reference.py", "capture_reference_bs4.py", "measurements.py")},
              browser_evidence=json.loads(evaluation_options.browser_evidence.read_text()),
              comparator_binaries={name: dict(path=str(evaluation_options.comparators / name),
                  sha256=hashlib.sha256((evaluation_options.comparators / name).read_bytes()).hexdigest(),
                  version=subprocess.check_output([str(evaluation_options.comparators / name), "--version"]).decode().strip()) for name in ("htmlq", "pup")},
              tokenizer=dict(name="tiktoken", encoding="o200k_base", version=importlib.metadata.version("tiktoken")),
              dependencies={name: importlib.metadata.version(name) for name in ("lxml", "beautifulsoup4", "tiktoken", "markdown-it-py", "trafilatura")},
              python=platform.python_version(),
              scope="Complete frozen public values; randomized 3-warmup/15 fresh launches and separate RSS; warm mapping excludes file reading; no billed reasoning/agent adoption claim")
(OUTPUT / "correctness.json").write_text(json.dumps(report, ensure_ascii=False, indent=2) + "\n")
print("Complete public-value controls and economics:", len(rows))
