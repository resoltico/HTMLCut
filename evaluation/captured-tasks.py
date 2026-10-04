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
import sys
import tiktoken
from pathlib import Path
from urllib.parse import urljoin, urlsplit, urlunsplit

from measurements import paired, warm
from capture_reference import reference, from_document, technical_node, text as literal_text
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
evaluation_options = parser.parse_args()
BINARY = evaluation_options.binary.resolve()
CAPTURES = evaluation_options.captures.resolve()
OUTPUT = evaluation_options.output.resolve()
if OUTPUT.is_relative_to(ROOT):
    parser.error("Execution evidence must be outside the repository")
OUTPUT.mkdir(parents=True, exist_ok=True)
binding = json.loads(evaluation_options.binding.read_text())
assert binding["source_commit"] == evaluation_options.source_sha
assert binding["binary_sha256"] == hashlib.sha256(BINARY.read_bytes()).hexdigest()
assert subprocess.check_output([str(BINARY), "--version"]).decode().strip() == "htmlcut 18.0.0"
assert subprocess.check_output([str(evaluation_options.baseline_binary.resolve()), "--version"]).decode().strip() == "htmlcut 17.0.0"
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


def value_plan(css, projection=None, fields=None, guards=None):
    plan = dict(schema="htmlcut.extraction.plan", version=4,
                strategy=dict(kind="css", selector=css), selection=dict(kind="all", min=1))
    if fields is not None:
        plan["projection"] = dict(kind="records", fields=fields)
    elif projection is not None:
        plan["projection"] = projection
    if guards:
        plan["guards"] = guards
    return plan


def execute(name, file, plan, expected, base=None, inline=None):
    path = OUTPUT / (name + ".plan.json")
    path.write_text(json.dumps(plan, ensure_ascii=False, separators=(",", ":")))
    command = [str(BINARY), "extract", "--file", str(CAPTURES / file), "--plan", str(path)]
    if base:
        command += ["--base-url", base]
    alternative = [sys.executable, str(ROOT / "evaluation/capture_reference.py"), "--task", name,
                   "--fixture", str(CAPTURES / file)]
    commands = dict(htmlcut=command, python=alternative)
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
    rows.append(dict(task=name, source=file, plan=plan, command=command,
                     correctness="complete independent equality", values=answer,
                     output_bytes=len(raw), output_tokens=len(encoder.encode(raw.decode())),
                     canonical_requested_tokens=len(encoder.encode(canonical.decode())), mandatory_envelope_tokens=0,
                     plan_tokens=len(encoder.encode(path.read_text())), timings=measured,
                     reference_command=alternative, reference_script_tokens=len(encoder.encode((ROOT / "evaluation/capture_reference.py").read_text()))))
    if inline is not None:
        import shlex
        rows[-1]["inline_authoring"] = dict(command=inline_command,
            tokens=len(encoder.encode(shlex.join(inline_command))),
            scope="Invocation tokens recorded separately from plan JSON and optional setup/evidence; absolute paths are part of this concrete command")
    content = source(file)
    rows[-1]["python_warm_parse_map"] = warm(lambda: reference(name, content), expected)
    if name != "exact-inline-source":
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
fields = [dict(name="title", selector="h3 a", projection=dict(kind="attribute", name="title")),
          dict(name="price", selector=".price_color", transforms=[dict(kind="normalize_whitespace")]),
          dict(name="stock", selector=".availability", transforms=[dict(kind="normalize_whitespace")]),
          dict(name="rating", selector=".star-rating", projection=dict(kind="attribute", name="class")),
          dict(name="url", selector="h3 a", projection=dict(kind="attribute", name="href"),
               transforms=[dict(kind="resolve_urls")])]
actual = execute("catalogue-records", "books.html", value_plan("article.product_pod", fields=fields),
                 catalogue, "https://books.toscrape.com/", inline=[
                     "--css", "article.product_pod", "--match", "all",
                     "--field", "title", "h3 a", "attribute:title",
                     "--field", "price", ".price_color", "normalized_text",
                     "--field", "stock", ".availability", "normalized_text",
                     "--field", "rating", ".star-rating", "attribute:class",
                     "--field", "url", "h3 a", "resolved_attribute:href"])
ratings = dict(One=1, Two=2, Three=3, Four=4, Five=5)
shortlist = lambda data: [dict(book, stock=book["stock"].strip(), rating=ratings[book["rating"].split()[-1]])
                         for book in data if float(book["price"][1:]) <= 25 and ratings[book["rating"].split()[-1]] >= 3]
assert shortlist(actual) == shortlist(catalogue)
rows[-1]["shortlist"] = shortlist(actual)
rows[-1]["caller_mapping"] = "Primitive price/rating/stock mapping only; no caller HTML reparsing"

titles = reference("catalogue-titles", source("books.html"))
execute("catalogue-titles", "books.html", value_plan("h3 a", dict(kind="attribute", name="title")), titles, inline=["--css", "h3 a", "--match", "all", "--read", "attribute:title"])
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
execute("news-records", "news.html", value_plan(".titleline", fields=[
    dict(name="title", selector=":scope > a"),
    dict(name="url", selector=":scope > a", projection=dict(kind="attribute", name="href"),
         transforms=[dict(kind="resolve_urls")])]), news, "https://news.ycombinator.com/")
score_plan = value_plan("tr.athing", fields=[
    dict(name="title", selector=".titleline > a"),
    dict(name="url", selector=".titleline > a", projection=dict(kind="attribute", name="href"),
         transforms=[dict(kind="resolve_urls")]),
    dict(name="score", selector=":scope + tr .score", selection=dict(kind="optional"),
         transforms=[dict(kind="normalize_whitespace")])])
score_plan["projection"]["following_siblings"] = 1
execute("news-score-records", "news.html", score_plan,
        reference("news-score-records", source("news.html")), "https://news.ycombinator.com/")
wiki = reference("wiki-population-records", source("wiki-countries.html"))
execute("wiki-population-records", "wiki-countries.html", value_plan("table.wikitable tbody tr:has(td)", fields=[
    dict(name="country", selector="th, td:nth-child(1)", transforms=[dict(kind="normalize_whitespace")]),
    dict(name="pop2022", selector="td:nth-child(2)", transforms=[dict(kind="normalize_whitespace")]),
    dict(name="pop2023", selector="td:nth-child(3)", transforms=[dict(kind="normalize_whitespace")]),
    dict(name="change", selector="td:nth-child(4)", transforms=[dict(kind="normalize_whitespace")])]), wiki)
details = reference("product-details", source("book-details.html"))
execute("product-details", "book-details.html", value_plan(".table.table-striped tr", fields=[
    dict(name="label", selector="th"), dict(name="value", selector="td")]), details)
price = reference("guarded-product-price", source("book-details.html"))[0]
execute("guarded-product-price", "book-details.html", value_plan(".product_main .price_color", guards=[
    dict(scope="document", selector=".product_main h1", min=1, max=1, read=dict(kind="dom_text"),
         predicate=dict(kind="exact", value="A Light in the Attic"))]), [price])

css = 'dl:has(> dt[id="pathlib.PurePath.full_match"])'
technical = soup("python-pathlib.html").select_one(css)
execute("technical-literal", "python-pathlib.html", value_plan(css), [technical.get_text()])
plan = value_plan(css, dict(kind="markdown"))
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
rows.append(dict(task="technical-markdown", markdown=markdown, code_payloads=code,
                 correctness="all complete pre payloads plus one framing LF; every source word occurrence retained",
                 source_word_occurrences=sum(expected_words.values()),
                 comparator="trafilatura 2.3.0", comparator_markdown=reading,
                 scope="Declared selected subsection versus heuristic reading of that same subsection; formatting conventions differ"))

browser = json.loads((CAPTURES / "browser-quotes.json").read_text())["values"]
execute("rendered-quote-records", "quotes-rendered-elements.html", value_plan(".quote", fields=[
    dict(name="text", selector=".text"), dict(name="author", selector=".author"),
    dict(name="tags", selector=".tag", selection=dict(kind="all", min=0))]), browser)
raw = source("quotes-unrendered.html")
inline = raw.split("var data = ", 1)[1].split(";\n    for", 1)[0]
mapped = [dict(text=q["text"], author=q["author"]["name"], tags=q["tags"]) for q in json.loads(inline)]
assert mapped == browser
plan = dict(schema="htmlcut.extraction.plan", version=4, strategy=dict(kind="slice", start=dict(kind="literal", value="var data = "), end=dict(kind="literal", value=";\n    for")), projection=dict(kind="source"))
execute("exact-inline-source", "quotes-unrendered.html", plan, [inline])
result = subprocess.run([str(BINARY), "extract", "--file", str(CAPTURES / "quotes-unrendered.html"), "--css", ".quote"], capture_output=True)
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
replay = subprocess.run([str(BINARY), "run", str(moved)], check=True, capture_output=True)
assert replay.stdout == result.stdout
rows.append(dict(task="moved-public-catalogue-replay", all_values_equal=True, original_inputs_deleted=True, original_immutable_capture_retained=True, bundle_bytes=moved.stat().st_size))
inspections = []
for version, executable in [(17, evaluation_options.baseline_binary.resolve()), (18, BINARY)]:
    command = [str(executable), "inspect", "--file", str(CAPTURES / "books.html"), "--css", "article.product_pod", "--samples", "3"]
    result = subprocess.run(command, check=True, capture_output=True)
    (OUTPUT / (str(version) + "-inspection.json")).write_bytes(result.stdout)
    answer = json.loads(result.stdout)
    assert answer["count"] == 20 and len(answer["samples"]) == 3 and not answer["samples_complete"]
    assert answer["version"] == (3 if version == 17 else 4)
    assert not any(field in result.stdout.decode() for field in ("cursor", "handle", "propose"))
    inspections.append(dict(version=version, binary_sha256=hashlib.sha256(executable.read_bytes()).hexdigest(),
                            command=command, bytes=len(result.stdout), tokens=len(encoder.encode(result.stdout.decode()))))
rows.append(dict(task="inspection-size", measurements=inspections, scope="Same immutable catalogue and same complete count/three requested row samples in published v17 and current v18; token counts are proxies, with no assumed reduction"))
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
for name, command in [("index", [str(BINARY), "describe"]), ("extract", [str(BINARY), "describe", "extract"]),
                      ("plan-schema", [str(BINARY), "schema", "htmlcut.extraction.plan"])]:
    data = subprocess.check_output(command)
    (OUTPUT / (name + ".json")).write_bytes(data)
    setup[name] = dict(command=command, bytes=len(data), tokens=len(encoder.encode(data.decode())))
report = dict(binary_sha256=hashlib.sha256(BINARY.read_bytes()).hexdigest(),
              source=evaluation_options.source_sha, rows=rows, binding=binding, captures=manifest, setup=setup,
              evaluator_sources={name: hashlib.sha256((ROOT / "evaluation" / name).read_bytes()).hexdigest()
                  for name in ("captured-tasks.py", "capture_reference.py", "measurements.py")},
              browser_evidence=json.loads((CAPTURES / "browser-quotes.json").read_text()),
              comparator_binaries={name: dict(path=str(evaluation_options.comparators / name),
                  sha256=hashlib.sha256((evaluation_options.comparators / name).read_bytes()).hexdigest(),
                  version=subprocess.check_output([str(evaluation_options.comparators / name), "--version"]).decode().strip()) for name in ("htmlq", "pup")},
              tokenizer=dict(name="tiktoken", encoding="o200k_base", version=importlib.metadata.version("tiktoken")),
              dependencies={name: importlib.metadata.version(name) for name in ("lxml", "beautifulsoup4", "tiktoken", "markdown-it-py", "trafilatura")},
              python=platform.python_version(),
              scope="Complete frozen public values; randomized 3-warmup/15 fresh launches and separate RSS; warm mapping excludes file reading; no billed reasoning/agent adoption claim")
(OUTPUT / "correctness.json").write_text(json.dumps(report, ensure_ascii=False, indent=2) + "\n")
print("Complete public-value controls and economics:", len(rows))
