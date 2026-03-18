import test from 'node:test';
import assert from 'node:assert/strict';
import {
    deriveDocumentTitle,
    extractTitleFromHtml,
    renderHtmlAsText,
    resolveUrl,
    rewriteHtmlUrls,
    wrapHtmlDocument,
} from '../src/html.js';

test('renderHtmlAsText: renders headings, links, lists, and decodes entities', () => {
    const html = '<h2>Docs &amp; Guides</h2><p><a href="https://example.com">Read&nbsp;now</a></p><ul><li>Alpha</li><li>Beta</li></ul>';
    const text = renderHtmlAsText(html);

    assert.equal(text, 'Docs & Guides\n-------------\n\nRead now [https://example.com]\n\n- Alpha\n- Beta');
});

test('renderHtmlAsText: renders blockquotes, tables, and pre blocks cleanly', () => {
    const html = '<blockquote><p>Quoted text</p></blockquote><table><tr><th>Name</th><th>Value</th></tr><tr><td>A</td><td>1</td></tr></table><pre>  keep\n  spacing</pre>';
    const text = renderHtmlAsText(html);

    assert.equal(text, '> Quoted text\n\n| Name | Value |\n| ---- | ----- |\n| A    | 1     |\n\n  keep\n  spacing');
});

test('renderHtmlAsText: uses parse5 document mode for full HTML documents and ignores head content', () => {
    const html = '<!DOCTYPE html><html><head><title>Hidden</title></head><body><p>Visible</p></body></html>';
    const text = renderHtmlAsText(html);

    assert.equal(text, 'Visible');
});

test('renderHtmlAsText: uses head/body root fragments correctly', () => {
    assert.equal(renderHtmlAsText('<head><title>Head Title</title></head>'), 'Head Title');
    assert.equal(renderHtmlAsText('<body><p>Visible</p></body>'), 'Visible');
});

test('renderHtmlAsText: handles inline code, images, anchors, description lists, and separators', () => {
    const html = '<p>Use <code>npm install</code><br><img alt="Logo"></p><p><a href="#frag">Local</a> and <a href="https://example.com">https://example.com</a></p><dl><dt>Term</dt><dd>Definition</dd></dl><hr>';
    const text = renderHtmlAsText(html);

    assert.equal(text, 'Use `npm install`\n[Logo]\n\nLocal and https://example.com\n\nTerm\n\n  Definition\n\n---');
});

test('renderHtmlAsText: inserts a space when inline markup wraps numbered clause prefixes', () => {
    const html = '<p><strong>1.3.1.</strong>These terms apply.</p><p><strong>Note:</strong>Example text.</p>';
    const text = renderHtmlAsText(html);

    assert.equal(text, '1.3.1. These terms apply.\n\nNote: Example text.');
});

test('renderHtmlAsText: renders form controls with useful current-state text', () => {
    assert.equal(
        renderHtmlAsText('<label>Name <input value="Ada"></label>'),
        'Name Ada'
    );
    assert.equal(
        renderHtmlAsText('<label><input type="checkbox" checked> Accept</label>'),
        '[x] Accept'
    );
    assert.equal(
        renderHtmlAsText('<label>Size<select><option>Small</option><option selected>Large</option></select></label>'),
        'Size Large'
    );
    assert.equal(
        renderHtmlAsText('<label>Notes<textarea>Line 1\nLine 2</textarea></label>'),
        'Notes\nLine 1\nLine 2'
    );
});

test('renderHtmlAsText: handles progress-like controls and ruby annotations', () => {
    assert.equal(renderHtmlAsText('<progress value="30" max="100"></progress>'), '30%');
    assert.equal(renderHtmlAsText('<meter value="0.7">70%</meter>'), '70%');
    assert.equal(renderHtmlAsText('<ruby>漢<rt>kan</rt></ruby>'), '漢');
});

test('renderHtmlAsText: skips scripts and supports nested ordered lists', () => {
    const html = '<ol><li>Parent<ol><li>Child</li></ol></li></ol><script>alert(1)</script>';
    const text = renderHtmlAsText(html);

    assert.equal(text, '1. Parent\n    1. Child');
});

test('renderHtmlAsText: respects ordered-list semantics from HTML attributes', () => {
    assert.equal(
        renderHtmlAsText('<ol start="3" type="A"><li>Gamma</li><li>Delta</li></ol>'),
        'C. Gamma\nD. Delta'
    );
    assert.equal(
        renderHtmlAsText('<ol reversed start="4"><li>Four</li><li>Three</li></ol>'),
        '4. Four\n3. Three'
    );
    assert.equal(
        renderHtmlAsText('<ol><li value="7">Seven</li><li>Eight</li></ol>'),
        '7. Seven\n8. Eight'
    );
    assert.equal(
        renderHtmlAsText('<ol type="i"><li>One</li><li type="A">Two</li></ol>'),
        'i. One\nB. Two'
    );
});

test('renderHtmlAsText: preserves block structure through wrapper elements', () => {
    const html = '<span><div><h2>Wrapped Heading</h2><p>Paragraph one.</p><ol><li>First</li><li>Second</li></ol><ul><li>Bullet</li></ul></div></span>';
    const text = renderHtmlAsText(html);

    assert.equal(
        text,
        'Wrapped Heading\n---------------\n\nParagraph one.\n\n1. First\n2. Second\n\n- Bullet'
    );
});

test('renderHtmlAsText: uses prefixed headings for h3 and deeper levels', () => {
    const text = renderHtmlAsText('<h3>Section</h3><h4>Detail</h4>');
    assert.equal(text, '### Section\n\n#### Detail');
});

test('renderHtmlAsText: renders table captions and keeps colspan structure readable', () => {
    const html = '<table><caption>Stats</caption><thead><tr><th colspan="2">Head</th></tr></thead><tbody><tr><td>A</td><td>B</td></tr></tbody></table>';
    const text = renderHtmlAsText(html);

    assert.equal(
        text,
        'Table: Stats\n\n| Head |     |\n| ---- | --- |\n| A    | B   |'
    );
});

test('rewriteHtmlUrls: rewrites relative URLs when a base is available', () => {
    const rewritten = rewriteHtmlUrls('<a href="../guide.html">Guide</a><img src="/logo.svg">', 'https://example.com/docs/page.html');
    assert.equal(rewritten, '<a href="https://example.com/guide.html">Guide</a><img src="https://example.com/logo.svg">');
});

test('rewriteHtmlUrls: preserves full-document structure when rewriting a complete HTML page', () => {
    const html = '<!DOCTYPE html><html><head><title>T</title></head><body><a href="/x">X</a></body></html>';
    const rewritten = rewriteHtmlUrls(html, 'https://example.com/base/page.html');

    assert.equal(
        rewritten,
        '<!DOCTYPE html><html><head><title>T</title></head><body><a href="https://example.com/x">X</a></body></html>'
    );
});

test('rewriteHtmlUrls: preserves html, head, and body root fragments with parse5 outer serialization', () => {
    assert.equal(
        rewriteHtmlUrls('<body class="page"><a href="/x">X</a></body>', 'https://example.com/base/page.html'),
        '<body class="page"><a href="https://example.com/x">X</a></body>'
    );
    assert.equal(
        rewriteHtmlUrls('<head><link rel="canonical" href="/x"></head>', 'https://example.com/base/page.html'),
        '<head><link rel="canonical" href="https://example.com/x"></head>'
    );
    assert.equal(
        rewriteHtmlUrls('<html lang="en"><body><a href="/x">X</a></body></html>', 'https://example.com/base/page.html'),
        '<html lang="en"><head></head><body><a href="https://example.com/x">X</a></body></html>'
    );
});

test('rewriteHtmlUrls: leaves fragments unchanged without a base URL', () => {
    const fragment = '<a href="guide.html">Guide</a>';
    assert.equal(rewriteHtmlUrls(fragment, null), fragment);
});

test('resolveUrl: leaves absolute and fragment URLs untouched', () => {
    assert.equal(resolveUrl('https://openai.com', 'https://example.com'), 'https://openai.com');
    assert.equal(resolveUrl('#section', 'https://example.com/docs'), '#section');
    assert.equal(resolveUrl('//cdn.example.com/app.js', 'https://example.com/docs'), 'https://cdn.example.com/app.js');
    assert.equal(resolveUrl('guide.html', 'not-a-url'), 'guide.html');
});

test('deriveDocumentTitle: prefers fragment title, then base URL, then file stem', () => {
    assert.equal(deriveDocumentTitle({
        matches: [{ html: '<title>Inside Title</title><p>Body</p>' }],
        input: '/tmp/report.html',
        baseUrl: null,
    }), 'Inside Title');

    assert.equal(deriveDocumentTitle({
        matches: [{ html: '<p>Body</p>' }],
        input: '-',
        baseUrl: 'https://example.com/path/page.html',
    }), 'example.com');

    assert.equal(deriveDocumentTitle({
        matches: [{ html: '<p>Body</p>' }],
        input: '/tmp/report.html',
        baseUrl: null,
    }), 'report');
});

test('extractTitleFromHtml: reads titles from full documents and fragments', () => {
    assert.equal(
        extractTitleFromHtml('<!DOCTYPE html><html><head><title>Doc Title</title></head><body><p>x</p></body></html>'),
        'Doc Title'
    );
    assert.equal(extractTitleFromHtml('<title>Fragment Title</title><p>x</p>'), 'Fragment Title');
});

test('wrapHtmlDocument: preserves a full HTML document and wraps multi-match fragments', () => {
    const existing = '<!DOCTYPE html><html><head><title>Keep</title></head><body><p>Body</p></body></html>';
    assert.equal(wrapHtmlDocument({
        matches: [{ index: 1, html: existing }],
        title: 'Ignored',
    }), existing);

    const wrapped = wrapHtmlDocument({
        matches: [
            { index: 1, html: '<p>One</p>' },
            { index: 2, html: '<p>Two</p>' },
        ],
        title: 'Selection',
    });

    assert.match(wrapped, /<!DOCTYPE html>/);
    assert.match(wrapped, /<title>Selection<\/title>/);
    assert.match(wrapped, /data-match-index="2"/);
});
