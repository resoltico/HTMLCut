# SPDX-License-Identifier: MPL-2.0
"""One-shot explicit CSS extraction comparator; current frozen-task semantics only."""
import json
import sys
from pathlib import Path
from urllib.parse import urljoin, urlsplit, urlunsplit
from bs4 import BeautifulSoup

task, filename = sys.argv[1:]
source = Path(filename).read_text(encoding="utf-8")

def resolved(base, value):
    p = urlsplit(urljoin(base, value))
    return urlunsplit(p._replace(path=p.path or "/"))

if task == "exact-inline-source":
    answer = [source.split("var data = ", 1)[1].split(";\n    for", 1)[0]]
elif task == "direct-inline-json":
    answer = [dict(text=q["text"], author=q["author"]["name"], tags=q["tags"])
              for q in json.loads(source.split("var data = ", 1)[1].split(";\n    for", 1)[0])]
else:
    soup = BeautifulSoup(source, "lxml")
    def one(root, css):
        matches = root.select(css)
        if len(matches) != 1:
            raise ValueError(f"Expected exactly one node for {css}")
        return matches[0]
    def literal(root, css):
        return one(root, css).get_text()
    if task == "catalogue-titles":
        answer = [a["title"] for a in soup.select("article.product_pod h3 a")]
    elif task == "catalogue-records":
        answer = [dict(title=one(book, "h3 a")["title"], price=literal(book, ".price_color"),
                       stock=" ".join(literal(book, ".availability").split()),
                       rating=" ".join(one(book, ".star-rating")["class"]),
                       url=resolved("https://books.toscrape.com/", one(book, "h3 a")["href"]))
                  for book in soup.select("article.product_pod")]
    elif task == "news-records":
        answer = [dict(title=a.get_text(), url=resolved("https://news.ycombinator.com/", a["href"]))
                  for a in soup.select(".titleline > a")]
    elif task == "news-score-records":
        answer = []
        for row in soup.select("tr.athing"):
            a = one(row, ".titleline > a")
            following = row.find_next_sibling()
            scores = following.select(".score")
            if len(scores) > 1:
                raise ValueError("Ambiguous score")
            answer.append(dict(title=a.get_text(), url=resolved("https://news.ycombinator.com/", a["href"]),
                               score=" ".join(scores[0].get_text().split()) if scores else None))
    elif task == "product-details":
        answer = [dict(label=literal(row, "th"), value=literal(row, "td"))
                  for row in soup.select(".table.table-striped tr")]
    elif task == "guarded-product-price":
        if literal(soup, ".product_main h1") != "A Light in the Attic":
            raise ValueError("Wrong product")
        answer = [literal(soup, ".product_main .price_color")]
    elif task == "rendered-quote-records":
        answer = [dict(text=literal(row, ".text"), author=literal(row, ".author"),
                       tags=[tag.get_text() for tag in row.select(".tag")])
                  for row in soup.select(".quote")]
    elif task == "technical-literal":
        answer = [literal(soup, 'dl:has(> dt[id="pathlib.PurePath.full_match"])')]
    else:
        raise ValueError(task)
print(json.dumps(answer, ensure_ascii=False, sort_keys=True, separators=(",", ":")))
