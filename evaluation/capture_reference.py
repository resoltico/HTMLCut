"""Independent complete answers from one libxml parse or caller-owned inline JSON."""
import argparse
import json
from pathlib import Path
from urllib.parse import urljoin, urlsplit, urlunsplit


def compact(value):
    return json.dumps(value, ensure_ascii=False, sort_keys=True, separators=(",", ":"))


def web_url(base, value):
    # RFC3986 6.2.3: this corpus's ASCII HTTP(S) authorities use '/' for an empty path.
    parts = urlsplit(urljoin(base, value))
    if parts.scheme not in ("http", "https"):
        raise ValueError("The declared corpus contains a non-web destination")
    return urlunsplit(parts._replace(path=parts.path or "/"))


def one(root, xpath):
    values = root.xpath(xpath)
    if len(values) != 1:
        raise ValueError(f"Expected one node for {xpath}")
    return values[0]


def text(node):
    return "".join(node.itertext())


def technical_node(root):
    return one(root, '//dl[dt[@id="pathlib.PurePath.full_match"]]')


def inline_source(source):
    start, end = "var data = ", ";\n    for"
    if source.count(start) != 1:
        raise ValueError("Inline initializer is not unique")
    content = source.split(start, 1)[1]
    if end not in content:
        raise ValueError("Inline initializer has no closing boundary")
    return content.split(end, 1)[0]


def from_document(task, root):
    if task == "catalogue-titles":
        return [node.get("title") for node in root.xpath('//article[contains(@class,"product_pod")]//h3/a')]
    if task == "catalogue-records":
        return [dict(title=one(node, ".//h3/a").get("title"),
                     price=text(one(node, './/p[contains(@class,"price_color")]')),
                     stock=text(one(node, './/p[contains(@class,"availability")]')),
                     rating=one(node, './/p[contains(@class,"star-rating")]').get("class"),
                     url=web_url("https://books.toscrape.com/", one(node, ".//h3/a").get("href")))
                for node in root.xpath('//article[contains(@class,"product_pod")]')]
    if task == "news-records":
        return [dict(title=text(node), url=web_url("https://news.ycombinator.com/", node.get("href")))
                for node in root.xpath('//span[contains(@class,"titleline")]/a')]
    if task == "product-details":
        return [dict(label=text(one(node, "./th")), value=text(one(node, "./td")))
                for node in root.xpath('//table[contains(@class,"table-striped")]//tr')]
    if task == "guarded-product-price":
        product = one(root, '//div[contains(@class,"product_main")]')
        if text(one(product, "./h1")) != "A Light in the Attic":
            raise ValueError("Product context changed")
        return [text(one(product, './p[contains(@class,"price_color")]'))]
    if task == "technical-literal":
        return [text(technical_node(root))]
    if task == "rendered-quote-records":
        return [dict(text=text(one(node, './span[@class="text"]')),
                     author=text(one(node, './/small[@class="author"]')),
                     tags=[text(tag) for tag in node.xpath('.//a[@class="tag"]')])
                for node in root.xpath('//div[@class="quote"]')]
    raise ValueError(f"Unknown document task: {task}")


def reference(task, source):
    if task == "release-assets":
        return [dict(name=item["name"], size=item["size"], url=item["url"]) for item in json.loads(source)["assets"]]
    if task == "exact-inline-source":
        return [inline_source(source)]
    if task == "inline-json-records":
        return [dict(text=item["text"], author=item["author"]["name"], tags=item["tags"])
                for item in json.loads(inline_source(source))]
    from lxml import html
    return from_document(task, html.fromstring(source))


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--task", required=True)
    parser.add_argument("--fixture", type=Path, required=True)
    options = parser.parse_args()
    print(compact(reference(options.task, options.fixture.read_bytes().decode("utf-8"))))
