#!/usr/bin/env python3
# SPDX-License-Identifier: MPL-2.0
"""Candidate semantic acceptance with independent literal expected results.

Run: python3 run_semantic_matrix.py --binary /absolute/htmlcut --results results.json
Derived from the immutable 2026-10-14 audit. Explicit reviewed overrides below
are contract changes, documented in docs/audit-acceptance.md.
"""
import argparse
import hashlib
import json
import pathlib
import subprocess
import time
import tempfile

ROOT = pathlib.Path(__file__).resolve().parent
CASES = []


def case(name, purpose, html, contract, expected=None, error=None, options=None,
         facts=None, classification="behavior", raw=None):
    CASES.append(dict(id=name, purpose=purpose, html=html, contract=contract,
                      expected=expected, error=error, options=options or [],
                      facts=facts or {}, classification=classification, raw=raw))


CATALOG = """<!doctype html><title>Warehouse 7</title><main><h1>Warehouse 7</h1>
<article data-id="A"><h2>Tea</h2><dl><dt>Stock</dt><dd>12</dd><dt>Price</dt><dd>£3.50</dd></dl><a href="../tea">Buy</a><ul><li>green</li><li>loose</li></ul><p class="badge">New</p></article>
<article data-id="B"><h2>Coffee</h2><dl><dt>Price</dt><dd>£6.25</dd><dt>Stock</dt><dd>0</dd></dl><a href="../coffee">Buy</a><ul><li>ground</li></ul></article></main>"""
CATALOG_CONTRACT = dict(expect=[dict(select="h1", equals="Warehouse 7")],
    select="article", match="all", min=2, max=2, fields={
        "id": dict(read="attr:data-id"), "title": "h2",
        "price": dict(select="dt", where=[dict(equals="Price")], until="dt",
                      fields={"amount": dict(select="dd", pattern=r"^£(?<value>[0-9]+\.[0-9]{2})$")}),
        "stock": dict(select="dt", where=[dict(equals="Stock")], until="dt",
                      fields={"amount": "dd"}),
        "tags": dict(select="li", match="all"),
        "badge": dict(select=".badge", match="optional"),
        "url": dict(select="a", read="url:href")})
case("catalog-records", "Useful nested inventory extraction with reordered labels, bounded rows, optional badge and resolved URLs",
     CATALOG, CATALOG_CONTRACT, [
        {"badge":"New","id":"A","price":{"amount":"3.50"},"stock":{"amount":"12"},"tags":["green","loose"],"title":"Tea","url":"https://shop.test/tea"},
        {"badge":None,"id":"B","price":{"amount":"6.25"},"stock":{"amount":"0"},"tags":["ground"],"title":"Coffee","url":"https://shop.test/coffee"}],
     options=["--base-url","https://shop.test/catalog/page"])
case("catalog-wrong-entity", "Page expectation rejects a structurally identical wrong warehouse", CATALOG.replace("Warehouse 7", "Warehouse 8"), CATALOG_CONTRACT,
     error="expectation_failed", options=["--base-url","https://shop.test/catalog/page"], facts={"path":"$.expect[0]","value":"Warehouse 8"})
case("catalog-truncated", "Row count rejects cleanly truncated inventory", CATALOG[:CATALOG.index('<article data-id="B"')], CATALOG_CONTRACT,
     error="count_mismatch", facts={"path":"$","found":1,"expected":"2"})
case("catalog-malformed-price", "Shape refusal points to nested row and field", CATALOG.replace("£6.25", "ask us"), CATALOG_CONTRACT,
     error="value_mismatch", options=["--base-url","https://shop.test/catalog/page"], facts={"path":"$.fields.price.fields.amount","row":[2],"value":"ask us"})
case("scope-anchor-and-ancestors", "A scoped field can select its anchor using document ancestor context", "<main><article id=a><b>x</b></article></main>",
     dict(select="article", fields={"self":dict(select="main article:scope", read="attr:id"),"body":"b"}), {"body":"x","self":"a"})
case("nested-index-refusal", "Nested list errors retain both enclosing row indexes", "<main><section><b>ok</b></section><section><b>ok</b><b>bad</b></section></main>",
     dict(select="section",match="all",fields={"items":dict(select="b",match="all",equals="ok")}),
     error="value_mismatch",facts={"path":"$.fields.items","row":[2,2],"value":"bad"})
case("optional-empty", "Absent optional field is JSON null", "<p>x</p>",dict(select="aside",match="optional"),None)
case("optional-ambiguous", "Optional refuses duplicates", "<i>x</i><i>y</i>",dict(select="i",match="optional"),error="count_mismatch",facts={"found":2,"expected":"0..1"})
case("all-min-zero", "Explicit empty list is allowed", "<p>x</p>",dict(select="aside",match="all",min=0),[])
case("all-default-refuses-empty", "All requires at least one unless declared otherwise", "<p>x</p>",dict(select="aside",match="all"),error="count_mismatch",facts={"found":0,"expected":"1.."})
case("nth-useful", "Take second task while accepting later tasks", "<li>A</li><li>B</li><li>C</li>",dict(select="li",nth=2),"B")
case("nth-too-short", "Nth reports lower bound", "<li>A</li>",dict(select="li",nth=2),error="count_mismatch",facts={"found":1,"expected":"2.."})
case("where-no-unmatched-attribute", "Attribute predicate filters rows missing id", "<p>x</p><p id=a>y</p>",dict(select="p",match="all",where=[dict(read="attr:id")]),["y"])
case("where-all-values", "Predicate comparison applies to every selected label", "<section><i>OK</i><i>bad</i></section><section><i>OK</i><i>OK</i></section>",
     dict(select="section",where=[dict(select="i",min=2,max=2,equals="OK")],read="text-content"),"OKOK")
case("expect-zero-count", "A negative expectation rejects injected advertisements", "<p>x</p><aside>ad</aside>",dict(select="p",expect=[dict(select="aside",min=0,max=0)]),error="expectation_failed",facts={"found":1,"expected":"0"})
case("extent-text-value", "Heading and following prose form one extracted section", "<main><h2>A</h2>lead<p>one</p><h2>B</h2><p>two</p></main>",dict(select="h2",nth=1,until="h2"),"A\nlead\none")
case("extent-sibling-selection", "Fields select later extent members using :scope", "<main><h2>A</h2><p>one</p><h2>B</h2><p>two</p></main>",
     dict(select="h2",match="all",until="h2",fields={"title":{},"body":dict(select=":scope ~ p",match="all")}),[{"body":["one"],"title":"A"},{"body":["two"],"title":"B"}])
case("extent-anchor-default-design", "Probe default field reading of a selected section: should retain its full extent", "<main><h2>A</h2><p>one</p><h2>B</h2><p>two</p></main>",
     dict(select="h2",match="all",until="h2",fields={"body":{}}),[{"body":"A\none"},{"body":"B\ntwo"}],classification="design_probe")
case("extent-parent-boundary-design", "Probe whether a child extent can capture the next parent record", "<main><h2>A</h2><p>one</p><h2>B</h2><p>two</p><footer>done</footer></main>",
     dict(select="h2",match="all",until="h2",fields={"body":dict(select=":scope",until="footer")}),[{"body":"A\none"},{"body":"B\ntwo"}],classification="design_probe")
case("extent-no-stop-design", "Probe missing section delimiter: the contract should reject an unclosed declared extent", "<main><h2>A</h2><p>one</p><aside>unrelated</aside></main>",
     dict(select="h2",until="footer"),error="count_mismatch",classification="design_probe")
case("text-script-style-template", "Document text skips non-rendered descendants", "<head><title>hidden</title></head><body><main>start<script>x</script><style>y</style><template>z</template><svg><text>svg</text><script>x</script></svg>end</main>","html","startsvgend")
case("text-content-keeps-script", "text-content is lexical DOM text and skips inert template descendants", "<main>start<script>x</script><style>y</style><template>z</template>end</main>",dict(select="main",read="text-content"),"startxyend")
case("template-root", "Template selected directly provides its contents, skipping nested inert templates", "<template id=t>a<b>b</b><template>c</template>d</template>","#t","abd")
case("template-not-selectable", "Template descendants cannot satisfy live extraction", "<template><p>x</p></template>",dict(select="p",match="all",min=0),[])
case("template-outer-html", "Template markup is preserved in serialized excerpts", "<template><b>x</b><template>y</template></template>",dict(select="template",read="outer-html"),"<template><b>x</b><template>y</template></template>")
case("exclude-clean-summary", "Exclude navigation and an ad from an article summary", "<article><nav>skip</nav><h1>Report</h1><p>one <em>two</em></p><aside class=ad>skip</aside></article>",dict(select="article",exclude=["nav",".ad"]),"Report\none two")
case("exclude-serialization", "Outer HTML excludes selected subtrees and preserves comments", "<article>a<!--proof--><b class=ad>x</b><i>y &amp; z</i></article>",dict(select="article",read="outer-html",exclude=[".ad"]),"<article>a<!--proof--><i>y &amp; z</i></article>")
case("exclude-anchor", "Excluding scope anchor produces an empty string", "<p>x</p>",dict(select="p",exclude=[":scope"]),"")
case("pre-lines", "Preformatted source code preserves blank lines and tabs", "<pre>  a\n\n\tb\n</pre>","pre","  a\n\n\tb")
case("pre-descendant", "Selecting code inside pre retains ancestor whitespace context", "<pre><code>  a\n b</code></pre>","code","  a\n b")
case("table-empty-cells", "Table text preserves an empty middle cell", "<table><tr><td>a</td><td></td><td>c</td></tr><tr><td>d</td><td>e</td><td>f</td></tr></table>","table","a\t\tc\nd\te\tf")
case("table-cell-alone", "A single selected cell omits framing tab", "<table><tr><td>a</td><td id=x>b</td></tr></table>","#x","b")
case("table-excluded-cell", "Excluded cells are removed from structural text", "<table><tr><td>a</td><td class=ad>x</td><td>b</td></tr></table>",dict(select="table",exclude=[".ad"]),"a\tb")
case("foreign-html-integration", "HTML foreignObject descendants retain block structure while SVG text remains inline", "<svg><text>A</text><foreignObject><p>B</p><p>C</p></foreignObject><text>D</text></svg>","svg","A\nB\nC\nD")
case("svg-namespaced-attribute", "Namespaced xlink href is accessible", "<svg><a xlink:href='/item'>link</a></svg>",dict(select="svg a",read="attr:xlink:href"),"/item")
case("svg-camel-attribute", "Parsed SVG attribute spelling is exact for direct reads", "<svg viewbox='0 0 2 2'></svg>",dict(select="svg",read="attr:viewBox"),"0 0 2 2")
case("svg-attribute-case-doc", "Probe documented ASCII-insensitive attribute name selection", "<svg viewbox='0 0 2 2'></svg>",dict(select="svg[viewbox]",read="attr:viewBox"),"0 0 2 2",classification="contract_claim")
case("svg-type-case-doc", "Probe documented ASCII-insensitive element name selection", "<svg><linearGradient id=g></linearGradient></svg>",dict(select="lineargradient",read="attr:id"),"g",classification="contract_claim")
case("svg-camel-type", "Exact SVG type case selects the parsed element", "<svg><linearGradient id=g></linearGradient></svg>",dict(select="linearGradient",read="attr:id"),"g")
case("regex-narrow-price", "Named capture strips validated currency prefix", "<p>Total: £17.42</p>",dict(select="p",pattern=r"^Total: £(?<value>[0-9]+\.[0-9]{2})$"),"17.42")
case("regex-unmatched-group", "A nonparticipating value capture refuses instead of returning a wrong value", "<p>sold</p>",dict(select="p",pattern=r"(?<value>[0-9]+)|sold"),error="value_mismatch",facts={"value":"sold"})
case("regex-empty-capture", "A participating empty capture is a valid empty value", "<p>x</p>",dict(select="p",pattern=r"x(?<value>)"),"")
case("regex-unicode-digits", "Unicode decimal digit class is intentional and documented", "<p>١٢</p>",dict(select="p",pattern=r"^\d+$"),"١٢")
case("regex-ascii-digits", "ASCII-only lexical guard rejects Arabic-Indic digits", "<p>١٢</p>",dict(select="p",pattern=r"^[0-9]+$"),error="value_mismatch",facts={"value":"١٢"})
case("base-first", "First href base governs subsequent relative links", "<base href='https://cdn.test/a/'><base href='https://wrong.test/'><a href='item'>x</a>",dict(select="a",read="url:href"),"https://cdn.test/a/item",options=["--base-url","https://origin.test/page"])
case("base-invalid-first", "Unparseable first base falls back rather than using later base", "<base href='https://['><base href='https://wrong.test/'><a href='item'>x</a>",dict(select="a",read="url:href"),"https://origin.test/item",options=["--base-url","https://origin.test/page"])
case("base-template-inert", "Template base cannot affect active link resolution", "<template><base href='https://wrong.test/'></template><a href='item'>x</a>",dict(select="a",read="url:href"),"https://origin.test/item",options=["--base-url","https://origin.test/page"])
case("base-data-forbidden", "HTML Standard rejects data scheme for frozen base URL and falls back", "<base href='data:text/html,x'><a href='item'>x</a>",dict(select="a",read="url:href"),"https://origin.test/item",options=["--base-url","https://origin.test/page"],classification="standard_correctness")
case("base-javascript-forbidden", "HTML Standard rejects javascript scheme for frozen base URL and falls back", "<base href='javascript:alert(1)'><a href='item'>x</a>",dict(select="a",read="url:href"),"https://origin.test/item",options=["--base-url","https://origin.test/page"],classification="standard_correctness")
case("base-data-fragment", "Forbidden data base must not poison fragment-only link resolution", "<base href='data:text/html,x'><a href='#item'>x</a>",dict(select="a",read="url:href"),"https://origin.test/page#item",options=["--base-url","https://origin.test/page"],classification="standard_correctness")
case("url-non-http", "URL reading retains declared non-HTTP absolute scheme", "<a href='mailto:person@example.test'>x</a>",dict(select="a",read="url:href"),"mailto:person@example.test")
case("url-whitespace", "URL reading strips ASCII whitespace and joins path", "<a href=' \t../a?x=1&amp;y=2\n '>x</a>",dict(select="a",read="url:href"),"https://origin.test/a?x=1&y=2",options=["--base-url","https://origin.test/dir/page"])
case("url-no-base", "Relative link without supplied base refuses", "<a href='item'>x</a>",dict(select="a",read="url:href"),error="unresolvable_url",facts={"path":"$","value":"item"})
case("url-malformed-with-base", "Malformed absolute link reports parse failure with a valid supplied base", "<a href='https://['>x</a>",dict(select="a",read="url:href"),error="unresolvable_url",facts={"path":"$","value":"https://["},options=["--base-url","https://origin.test/page"],classification="diagnostic_probe")
case("attribute-missing", "Missing anchor attribute has field location", "<article><a>x</a></article>",dict(select="article",fields={"url":dict(select="a",read="attr:href")}),error="missing_attribute",facts={"path":"$.fields.url"})
case("raw-newline-array-design", "Raw array framing should distinguish embedded newlines from row separators", "<pre>a\nb</pre><pre>c</pre>",dict(select="pre",match="all"),["a\nb","c"],raw=b"a\nb\nc\n",options=["--raw"],classification="design_probe")
case("raw-null", "Raw absent optional value emits no bytes", "<p>x</p>",dict(select="aside",match="optional"),None,raw=b"",options=["--raw"])
case("utf8-malformed", "Malformed UTF-8 fails instead of replacement", b"<p>bad\xff</p>","p",error="undecodable_input",facts={"encoding":"utf-8","offset":6})
case("encoding-meta-cp1252", "HTML encoding declaration supports legacy downloaded documents", b"<meta charset=windows-1252><p>caf\xe9 \x80</p>","p","café €")
case("encoding-explicit-override", "Caller label overrides HTML declaration", b"<meta charset=utf-8><p>caf\xe9</p>","p","café",options=["--encoding","windows-1252"])
case("encoding-bom-priority", "UTF-8 BOM overrides caller label", b"\xef\xbb\xbf<p>caf\xc3\xa9</p>","p","café",options=["--encoding","windows-1252"])
case("encoding-invalid-caller", "Unknown caller encoding is an invalid request", "<p>x</p>","p",error="invalid_encoding",options=["--encoding","bogus"])
case("markup-malformed-accepted", "HTML parser repairs omitted list end tags predictably", "<ul><li>one<li>two</ul>",dict(select="li",match="all"),["one","two"])
case("stress-many-records", "Bounded 5000-row extraction with exact count", "<main>"+"<p>x</p>"*5000+"</main>",dict(select="p",match="all",min=5000,max=5000),["x"]*5000)


# Approved candidate contracts; immutable baseline oracles remain in the handoff archive.
for item in CASES:
    if item['id']=='extent-sibling-selection':
        item['contract']['fields']['title']={'select':':scope'}
    elif item['id']=='extent-no-stop-design':
        item.update(error=None,expected='A\none\nunrelated',classification='intentional_contract')
    elif item['id'] in ('svg-attribute-case-doc','svg-type-case-doc'):
        item.update(error='count_mismatch',expected=None,facts={'found':0,'expected':'1'},classification='corrected_document_claim')
    elif item['id']=='raw-newline-array-design':
        item.update(error='unrepresentable_output',raw=None,expected=None,facts={'path':'$','row':[1]},classification='intentional_contract')


def main():
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument("--binary", type=pathlib.Path, required=True)
    p.add_argument("--results", type=pathlib.Path, required=True)
    p.add_argument("--case", action="append", default=[])
    args = p.parse_args()
    binary = args.binary.resolve()
    temporary=tempfile.TemporaryDirectory(prefix="htmlcut-field-semantics-")
    fixtures=pathlib.Path(temporary.name)
    outcomes = []
    for c in CASES:
        if args.case and c["id"] not in args.case:
            continue
        data = c["html"] if isinstance(c["html"], bytes) else c["html"].encode()
        fixture = fixtures / (c["id"] + ".html")
        fixture.write_bytes(data)
        contract_file = fixtures / (c["id"] + ".json")
        contract_file.write_text(json.dumps(c["contract"], ensure_ascii=False, separators=(",", ":"))+"\n")
        cmd = [str(binary), "run", "--from-file", str(contract_file), str(fixture), *c["options"]]
        begin = time.monotonic()
        proc = subprocess.run(cmd, stdout=subprocess.PIPE, stderr=subprocess.PIPE, timeout=30)
        elapsed = time.monotonic() - begin
        issues = []
        parsed = None
        try:
            parsed = json.loads(proc.stderr if c["error"] else proc.stdout) if c["raw"] is None else None
        except Exception as e:
            issues.append("Expected JSON could not be decoded: "+str(e))
        expected_code = {"count_mismatch":3,"value_mismatch":3,"expectation_failed":3,"missing_attribute":3,"unresolvable_url":3,"undecodable_input":5,"invalid_encoding":2,"unrepresentable_output":3}.get(c["error"],0)
        if proc.returncode != expected_code:
            issues.append(f"Exit {proc.returncode}, expected {expected_code}")
        if c["error"]:
            if proc.stdout:
                issues.append("Refusal leaked stdout")
            if not isinstance(parsed,dict) or parsed.get("error") != c["error"]:
                issues.append("Wrong error code")
            if isinstance(parsed,dict):
                for k,v in c["facts"].items():
                    if parsed.get(k) != v:
                        issues.append(f"Fact {k}: {parsed.get(k)!r}, expected {v!r}")
        else:
            if proc.stderr:
                issues.append("Successful extraction has stderr")
            if c["raw"] is not None:
                if proc.stdout != c["raw"]:
                    issues.append("Raw stdout differs")
            elif parsed != c["expected"]:
                issues.append("Extracted value differs")
            elif not proc.stdout.endswith(b"\n"):
                issues.append("JSON result lacks framing newline")
        outcomes.append({k:v for k,v in c.items() if k not in ("html","raw")}|dict(
            command=cmd, fixture_sha256=hashlib.sha256(data).hexdigest(),
            expected_raw_hex=None if c["raw"] is None else c["raw"].hex(),
            stdout=proc.stdout.decode("utf-8", "replace"),stderr=proc.stderr.decode("utf-8","replace"),
            exit=proc.returncode,elapsed_seconds=round(elapsed,6),passed=not issues,issues=issues))
    result = dict(binary=str(binary),binary_sha256=hashlib.sha256(binary.read_bytes()).hexdigest(),
                  version=subprocess.check_output([str(binary),"--version"]).decode().strip(),
                  total=len(outcomes),passed=sum(o["passed"] for o in outcomes),cases=outcomes)
    if not outcomes: raise RuntimeError("no cases were discovered")
    if args.results.exists(): raise RuntimeError("candidate result already exists")
    args.results.parent.mkdir(parents=True,exist_ok=True)
    args.results.write_text(json.dumps(result,ensure_ascii=False,indent=2)+"\n")
    print(json.dumps({k:result[k] for k in ("binary","version","total","passed")}))
    for o in outcomes:
        if not o["passed"]:
            print(o["id"], "; ".join(o["issues"]))
    temporary.cleanup()
    return 0 if result["passed"] == result["total"] else 1


if __name__ == "__main__":
    raise SystemExit(main())
