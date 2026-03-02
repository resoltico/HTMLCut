import test from 'node:test';
import assert from 'node:assert/strict';
import { join } from 'node:path';
import { tmpdir } from 'node:os';
import { mkdtemp, readFile, rm } from 'node:fs/promises';
import { getOutputBase, writeOutput, toPlainText } from '../src/formatters.js';

test('Formatters generate predictable timestamped output base paths', () => {
    const base = getOutputBase('testout');
    assert.ok(base.startsWith('testout-htmlcut-'));
    // No extension — the caller appends .html / .txt so both files share the same stem.
    assert.ok(!base.endsWith('.html') && !base.endsWith('.txt'));

    // Timestamp format: YYYY-MM-DD-HH-mm-ss (local time via Date methods) + 4-hex random suffix
    const match = /^testout-htmlcut-\d{4}-\d{2}-\d{2}-\d{2}-\d{2}-\d{2}-[a-f0-9]{4}$/.test(base);
    assert.ok(match, `Base ${base} did not match expected format`);

    const base2 = getOutputBase('results/my-doc');
    assert.ok(base2.startsWith('results/my-doc-htmlcut-'));
});

test('writeOutput wraps HTML content with UTF-8 charset boilerplate (fallback title)', async () => {
    const tempDir = await mkdtemp(join(tmpdir(), 'htmlcut-test-'));
    const outputPath = join(tempDir, 'test.html');
    try {
        await writeOutput('<p>Hello</p>', outputPath);
        const writtenContent = await readFile(outputPath, 'utf8');
        assert.ok(writtenContent.includes('<meta charset="utf-8">'));
        assert.ok(writtenContent.includes('<meta name="viewport" content="width=device-width, initial-scale=1">'));
        assert.ok(writtenContent.includes('<!DOCTYPE html>'));
        assert.ok(writtenContent.includes('<title>HTMLCut Extraction</title>'));
        assert.ok(writtenContent.includes('<body>\n<p>Hello</p>\n</body>'));
    } finally {
        await rm(tempDir, { recursive: true, force: true });
    }
});

test('writeOutput derives title from URL hostname', async () => {
    const tempDir = await mkdtemp(join(tmpdir(), 'htmlcut-test-'));
    const outputPath = join(tempDir, 'test.html');
    try {
        await writeOutput('<p>Content</p>', outputPath, 'https://nodejs.org/en/docs');
        const writtenContent = await readFile(outputPath, 'utf8');
        assert.ok(writtenContent.includes('<title>nodejs.org</title>'));
    } finally {
        await rm(tempDir, { recursive: true, force: true });
    }
});

test('writeOutput derives title from local filename stem', async () => {
    const tempDir = await mkdtemp(join(tmpdir(), 'htmlcut-test-'));
    const outputPath = join(tempDir, 'test.html');
    try {
        await writeOutput('<p>Content</p>', outputPath, '/home/user/my-article.html');
        const writtenContent = await readFile(outputPath, 'utf8');
        assert.ok(writtenContent.includes('<title>my-article</title>'));
    } finally {
        await rm(tempDir, { recursive: true, force: true });
    }
});

test('writeOutput falls back to source title when fragment <title> is whitespace-only', async () => {
    const tempDir = await mkdtemp(join(tmpdir(), 'htmlcut-test-'));
    const outputPath = join(tempDir, 'test.html');
    try {
        const fragment = '<title>   </title><p>Content</p>';
        await writeOutput(fragment, outputPath, 'https://example.com');
        const writtenContent = await readFile(outputPath, 'utf8');
        // Whitespace-only title must NOT produce an empty <title></title>
        assert.ok(writtenContent.includes('<title>example.com</title>'), `Expected fallback title, got: ${writtenContent}`);
        assert.ok(!writtenContent.includes('<title></title>'), 'Empty title must not appear');
    } finally {
        await rm(tempDir, { recursive: true, force: true });
    }
});

test('writeOutput preserves existing <title> from extracted fragment', async () => {
    const tempDir = await mkdtemp(join(tmpdir(), 'htmlcut-test-'));
    const outputPath = join(tempDir, 'test.html');
    try {
        const fragment = '<title>Original Page Title</title><p>Content</p>';
        await writeOutput(fragment, outputPath, 'https://example.com');
        const writtenContent = await readFile(outputPath, 'utf8');
        assert.ok(writtenContent.includes('<title>Original Page Title</title>'));
        // URL hostname must NOT override the fragment's own title
        assert.ok(!writtenContent.includes('<title>example.com</title>'));
    } finally {
        await rm(tempDir, { recursive: true, force: true });
    }
});

test('writeOutput escapes HTML-special characters in filename stem title', async () => {
    const tempDir = await mkdtemp(join(tmpdir(), 'htmlcut-test-'));
    const outputPath = join(tempDir, 'test.html');
    try {
        await writeOutput('<p>Content</p>', outputPath, '/tmp/page<script>.html');
        const writtenContent = await readFile(outputPath, 'utf8');
        assert.ok(writtenContent.includes('<title>page&lt;script&gt;</title>'), `Expected escaped title, got: ${writtenContent}`);
        assert.ok(!writtenContent.includes('<title>page<script>'), 'Raw < must not appear in title');
    } finally {
        await rm(tempDir, { recursive: true, force: true });
    }
});

test('writeOutput escapes & and " in filename stem title', async () => {
    const tempDir = await mkdtemp(join(tmpdir(), 'htmlcut-test-'));
    const outputPath = join(tempDir, 'test.html');
    try {
        await writeOutput('<p>Content</p>', outputPath, '/tmp/cats&dogs "report".html');
        const writtenContent = await readFile(outputPath, 'utf8');
        assert.ok(writtenContent.includes('<title>cats&amp;dogs &quot;report&quot;</title>'), `Expected escaped title, got: ${writtenContent}`);
    } finally {
        await rm(tempDir, { recursive: true, force: true });
    }
});

test('writeOutput does not double-encode entities in fragment title', async () => {
    const tempDir = await mkdtemp(join(tmpdir(), 'htmlcut-test-'));
    const outputPath = join(tempDir, 'test.html');
    try {
        // Fragment already contains a properly entity-encoded title
        const fragment = '<title>Cats &amp; Dogs</title><p>Content</p>';
        await writeOutput(fragment, outputPath, 'https://example.com');
        const writtenContent = await readFile(outputPath, 'utf8');
        assert.ok(writtenContent.includes('<title>Cats &amp; Dogs</title>'), `Expected single-encoded title, got: ${writtenContent}`);
        assert.ok(!writtenContent.includes('&amp;amp;'), 'Must not double-encode &amp; to &amp;amp;');
    } finally {
        await rm(tempDir, { recursive: true, force: true });
    }
});

test('writeOutput does not double-encode &nbsp; in fragment title', async () => {
    const tempDir = await mkdtemp(join(tmpdir(), 'htmlcut-test-'));
    const outputPath = join(tempDir, 'test.html');
    try {
        const fragment = '<title>Cats&nbsp;Dogs</title><p>Content</p>';
        await writeOutput(fragment, outputPath, 'https://example.com');
        const writtenContent = await readFile(outputPath, 'utf8');
        assert.ok(writtenContent.includes('<title>Cats\u00A0Dogs</title>'), `Expected decoded nbsp in title, got: ${writtenContent}`);
        assert.ok(!writtenContent.includes('&amp;nbsp;'), 'Must not double-encode &nbsp; to &amp;nbsp;');
    } finally {
        await rm(tempDir, { recursive: true, force: true });
    }
});

test('toPlainText decodes named HTML entities', () => {
    assert.equal(toPlainText('&lt;tag&gt;'), '<tag>');
    assert.equal(toPlainText('A &amp; B'), 'A & B');
    assert.equal(toPlainText('&quot;quoted&quot;'), '"quoted"');
    assert.equal(toPlainText('it&apos;s'), "it's");
    // &nbsp; is decoded then normalised to a regular space in plain text output.
    // Leading/trailing boundary newlines are stripped; interior spacing is preserved.
    assert.equal(toPlainText('word&nbsp;word'), 'word word');
});

test('toPlainText decodes HTML5 typographic named entities', () => {
    // These are the most common entities found in real web content that
    // the old 6-entity table silently left undecoded.
    assert.equal(toPlainText('em&mdash;dash'), 'em\u2014dash');
    assert.equal(toPlainText('en&ndash;dash'), 'en\u2013dash');
    assert.equal(toPlainText('&hellip;'), '\u2026');
    assert.equal(toPlainText('&lsquo;text&rsquo;'), '\u2018text\u2019');
    assert.equal(toPlainText('&ldquo;text&rdquo;'), '\u201Ctext\u201D');
    assert.equal(toPlainText('&copy; 2025'), '\u00A9 2025');
    assert.equal(toPlainText('&reg;'), '\u00AE');
    assert.equal(toPlainText('&trade;'), '\u2122');
    assert.equal(toPlainText('&euro;'), '\u20AC');
    assert.equal(toPlainText('&pound;'), '\u00A3');
    assert.equal(toPlainText('&times;'), '\u00D7');
    assert.equal(toPlainText('&divide;'), '\u00F7');
    assert.equal(toPlainText('&bull; item'), '\u2022 item');
    assert.equal(toPlainText('&frac12;'), '\u00BD');
    assert.equal(toPlainText('&deg;'), '\u00B0');
    // Latin Extended-A entities (European language content)
    assert.equal(toPlainText('&Scaron;'), '\u0160');
    assert.equal(toPlainText('&scaron;'), '\u0161');
    assert.equal(toPlainText('&OElig;'), '\u0152');
    assert.equal(toPlainText('&oelig;'), '\u0153');
    assert.equal(toPlainText('&Aogon;'), '\u0104');
    // Uppercase core aliases defined by HTML5
    assert.equal(toPlainText('&AMP;'), '&');
    assert.equal(toPlainText('&LT;'), '<');
    assert.equal(toPlainText('&GT;'), '>');
    assert.equal(toPlainText('&QUOT;'), '"');
    // Unknown entity left as-is (no corruption)
    assert.equal(toPlainText('&notanentity;'), '&notanentity;');
});

test('toPlainText decodes decimal numeric character references', () => {
    assert.equal(toPlainText('<p>em&#8212;dash</p>'), 'em\u2014dash');
    assert.equal(toPlainText('&#169; 2024'), '\u00A9 2024');
    assert.equal(toPlainText('<span>&#39;single&#39;</span>'), "'single'");
});

test('toPlainText decodes hex numeric character references', () => {
    assert.equal(toPlainText('&#x2014;text'), '\u2014text');
    assert.equal(toPlainText('&#x00A9; info'), '\u00A9 info');
    assert.equal(toPlainText('&#x27;quoted&#x27;'), "'quoted'");
});

test('toPlainText leaves out-of-range numeric character references as-is', () => {
    // 0x110000 (1114112) exceeds MAX_CODE_POINT (0x10FFFF = 1114111) — must be left unchanged
    assert.equal(toPlainText('&#1114112;'), '&#1114112;');
    assert.equal(toPlainText('&#x110000;'), '&#x110000;');
});

test('toPlainText leaves surrogate codepoints as-is (prevents RangeError crash)', () => {
    // U+D800–U+DFFF are surrogate halves — not valid Unicode scalar values.
    // String.fromCodePoint throws RangeError for them; decodeEntities must guard against this.
    assert.equal(toPlainText('&#55296;'), '&#55296;');   // U+D800 — high surrogate boundary
    assert.equal(toPlainText('&#57343;'), '&#57343;');   // U+DFFF — low surrogate boundary
    assert.equal(toPlainText('&#xD800;'), '&#xD800;');   // same, hex form
    assert.equal(toPlainText('&#xDFFF;'), '&#xDFFF;');
    // Adjacent valid codepoints still decode correctly
    assert.equal(toPlainText('&#55295;'), '\uD7FF');     // U+D7FF — just below surrogate range
    assert.equal(toPlainText('&#57344;'), '\uE000');     // U+E000 — just above surrogate range
});

test('toPlainText does not double-decode numeric entity followed by partial entity text', () => {
    // &#38; is the numeric ref for '&'. In '&#38;lt;', only &#38; is an entity;
    // the resulting '&' does NOT form a new entity with the following 'lt;'.
    // A multi-pass decoder would incorrectly produce '<'; single-pass gives '&lt;'.
    assert.equal(toPlainText('&#38;lt;'), '&lt;');
    assert.equal(toPlainText('&#38;amp;'), '&amp;');
    // Same for hex: &#x26; is '&'
    assert.equal(toPlainText('&#x26;lt;'), '&lt;');
});

test('writeOutput does not double-encode decimal numeric entity in fragment title', async () => {
    const tempDir = await mkdtemp(join(tmpdir(), 'htmlcut-test-'));
    const outputPath = join(tempDir, 'test.html');
    try {
        const fragment = '<title>Node &#8212; Docs</title><p>Content</p>';
        await writeOutput(fragment, outputPath, 'https://example.com');
        const writtenContent = await readFile(outputPath, 'utf8');
        assert.ok(writtenContent.includes('<title>Node \u2014 Docs</title>'),
            `Expected decoded em-dash in title, got: ${writtenContent}`);
        assert.ok(!writtenContent.includes('&amp;#8212;'), 'Must not double-encode &#8212; to &amp;#8212;');
    } finally {
        await rm(tempDir, { recursive: true, force: true });
    }
});

test('writeOutput writes TXT content without boilerplate', async () => {
    const tempDir = await mkdtemp(join(tmpdir(), 'htmlcut-test-'));
    const outputPath = join(tempDir, 'test.txt');
    try {
        await writeOutput('Hello text', outputPath);
        const writtenContent = await readFile(outputPath, 'utf8');
        assert.equal(writtenContent, 'Hello text');
    } finally {
        await rm(tempDir, { recursive: true, force: true });
    }
});

test('writeOutput passes through content that already contains HTML boilerplate', async () => {
    const tempDir = await mkdtemp(join(tmpdir(), 'htmlcut-test-'));
    const outputPath = join(tempDir, 'test.html');
    try {
        // Content already contains <html — must not be double-wrapped
        const alreadyWrapped = '<!DOCTYPE html>\n<html lang="en"><head><meta charset="utf-8"><title>Existing</title></head><body><p>Content</p></body></html>';
        await writeOutput(alreadyWrapped, outputPath);
        const writtenContent = await readFile(outputPath, 'utf8');
        assert.equal(writtenContent, alreadyWrapped);
    } finally {
        await rm(tempDir, { recursive: true, force: true });
    }
});

test('writeOutput passes through content starting with <html> (no DOCTYPE)', async () => {
    const tempDir = await mkdtemp(join(tmpdir(), 'htmlcut-test-'));
    const outputPath = join(tempDir, 'test.html');
    try {
        const alreadyHtml = '<html lang="en"><body><p>Content</p></body></html>';
        await writeOutput(alreadyHtml, outputPath);
        const writtenContent = await readFile(outputPath, 'utf8');
        assert.equal(writtenContent, alreadyHtml, 'Content starting with <html> must not be double-wrapped');
    } finally {
        await rm(tempDir, { recursive: true, force: true });
    }
});

test('writeOutput wraps fragment that mentions <html> mid-body (not a document)', async () => {
    const tempDir = await mkdtemp(join(tmpdir(), 'htmlcut-test-'));
    const outputPath = join(tempDir, 'test.html');
    try {
        // Fragment discusses the <html> element in prose — must still receive the wrapper
        const fragment = '<p>Read the <html> spec for details.</p>';
        await writeOutput(fragment, outputPath);
        const writtenContent = await readFile(outputPath, 'utf8');
        assert.ok(writtenContent.startsWith('<!DOCTYPE html>'), 'Must add DOCTYPE wrapper');
        assert.ok(writtenContent.includes(fragment), 'Fragment must appear verbatim in body');
    } finally {
        await rm(tempDir, { recursive: true, force: true });
    }
});

test('writeOutput falls back to generic title when source yields an empty stem', async () => {
    const tempDir = await mkdtemp(join(tmpdir(), 'htmlcut-test-'));
    const outputPath = join(tempDir, 'test.html');
    try {
        // '/' is not a valid URL and basename('/') is '', so stem is empty — must use fallback
        await writeOutput('<p>Content</p>', outputPath, '/');
        const writtenContent = await readFile(outputPath, 'utf8');
        assert.ok(writtenContent.includes('<title>HTMLCut Extraction</title>'), `Expected fallback title, got: ${writtenContent}`);
    } finally {
        await rm(tempDir, { recursive: true, force: true });
    }
});

// ── toPlainText structural rendering ──────────────────────────────────────────

test('toPlainText renders paragraphs separated by blank lines', () => {
    const result = toPlainText('<p>First</p><p>Second</p>');
    assert.equal(result, 'First\n\nSecond');
});

test('toPlainText renders <br> as a newline', () => {
    assert.equal(toPlainText('line one<br>line two'), 'line one\nline two');
});

test('toPlainText renders <hr> as a horizontal rule', () => {
    const result = toPlainText('<p>Above</p><hr><p>Below</p>');
    assert.ok(result.includes('Above'), 'must include above text');
    assert.ok(result.includes('Below'), 'must include below text');
    assert.ok(result.includes('────'), 'must include horizontal rule');
});

test('toPlainText renders <h1> with = underline', () => {
    const result = toPlainText('<h1>Hello World</h1>');
    assert.ok(result.startsWith('Hello World\n==========='), `got: ${result}`);
});

test('toPlainText renders <h2> with - underline', () => {
    const result = toPlainText('<h2>Section</h2>');
    assert.ok(result.startsWith('Section\n-------'), `got: ${result}`);
});

test('toPlainText renders <h3>–<h6> with # prefix', () => {
    assert.ok(toPlainText('<h3>Sub</h3>').startsWith('### Sub'));
    assert.ok(toPlainText('<h4>Sub</h4>').startsWith('#### Sub'));
    assert.ok(toPlainText('<h6>Sub</h6>').startsWith('###### Sub'));
});

test('toPlainText renders <ul> with bullet markers by depth', () => {
    const result = toPlainText('<ul><li>Alpha</li><li>Beta</li></ul>');
    assert.ok(result.includes('* Alpha'), `got: ${result}`);
    assert.ok(result.includes('* Beta'), `got: ${result}`);
});

test('toPlainText renders <ol> with decimal counters', () => {
    const result = toPlainText('<ol><li>One</li><li>Two</li><li>Three</li></ol>');
    assert.ok(result.includes('1. One'), `got: ${result}`);
    assert.ok(result.includes('2. Two'), `got: ${result}`);
    assert.ok(result.includes('3. Three'), `got: ${result}`);
});

test('toPlainText renders nested <ul> with indented sub-bullets', () => {
    const html = '<ul><li>Parent<ul><li>Child A</li><li>Child B</li></ul></li></ul>';
    const result = toPlainText(html);
    assert.ok(result.includes('* Parent'), `missing outer bullet: ${result}`);
    assert.ok(result.includes('  - Child A'), `missing indented child: ${result}`);
    assert.ok(result.includes('  - Child B'), `missing indented child: ${result}`);
});

test('toPlainText renders nested <ol> inside <ul> with alpha markers at depth 1', () => {
    // Depth 0 = decimal, depth 1 = alpha (a./b.), depth 2 = roman (i./ii.)
    const html = '<ul><li>Item<ol><li>First</li><li>Second</li></ol></li></ul>';
    const result = toPlainText(html);
    assert.ok(result.includes('* Item'), `missing outer bullet: ${result}`);
    assert.ok(result.includes('  a. First'), `missing alpha child: ${result}`);
    assert.ok(result.includes('  b. Second'), `missing alpha child: ${result}`);
});

test('toPlainText renders depth-1 <ol> with alpha markers', () => {
    const html = '<ol><li>Outer<ol><li>Sub 1</li><li>Sub 2</li></ol></li></ol>';
    const result = toPlainText(html);
    assert.ok(result.includes('  a. Sub 1'), `got: ${result}`);
    assert.ok(result.includes('  b. Sub 2'), `got: ${result}`);
});

test('toPlainText renders depth-2 <ol> with roman markers', () => {
    const html = '<ol><li>L0<ol><li>L1<ol><li>Deep 1</li><li>Deep 2</li></ol></li></ol></li></ol>';
    const result = toPlainText(html);
    assert.ok(result.includes('    i. Deep 1'), `got: ${result}`);
    assert.ok(result.includes('    ii. Deep 2'), `got: ${result}`);
});

test('toPlainText renders link as "text [url]"', () => {
    const result = toPlainText('<a href="https://example.com">Visit us</a>');
    assert.equal(result, 'Visit us [https://example.com]');
});

test('toPlainText does not duplicate URL when link text equals href', () => {
    const result = toPlainText('<a href="https://example.com">https://example.com</a>');
    assert.equal(result, 'https://example.com');
});

test('toPlainText omits URL for fragment-only links', () => {
    const result = toPlainText('<a href="#section">Jump</a>');
    assert.equal(result, 'Jump');
});

test('toPlainText renders link with no text as bare [url]', () => {
    const result = toPlainText('<a href="https://example.com"></a>');
    assert.equal(result, '[https://example.com]');
});

test('toPlainText suppresses <script> and <style> content', () => {
    const result = toPlainText('<p>Visible</p><script>alert(1)</script><style>body{}</style><p>Also visible</p>');
    assert.equal(result, 'Visible\n\nAlso visible');
});

test('toPlainText renders inline <code> in backticks', () => {
    const result = toPlainText('<p>Use the <code>npm install</code> command.</p>');
    assert.equal(result, 'Use the `npm install` command.');
});

test('toPlainText preserves whitespace inside <pre>', () => {
    const result = toPlainText('<pre>  line one\n  line two\n</pre>');
    assert.ok(result.includes('  line one\n  line two'), `got: ${result}`);
});

test('toPlainText renders <img> alt text in brackets', () => {
    const result = toPlainText('<img src="logo.png" alt="Company Logo">');
    assert.equal(result, '[Company Logo]');
});

test('toPlainText omits <img> with no or empty alt text', () => {
    assert.equal(toPlainText('<img src="x.png">'), '');
    assert.equal(toPlainText('<img src="x.png" alt="">'), '');
});

test('toPlainText renders <blockquote> with > prefix', () => {
    const result = toPlainText('<blockquote><p>Quoted text</p></blockquote>');
    assert.ok(result.includes('> Quoted text'), `got: ${result}`);
});

test('toPlainText collapses 3+ consecutive blank lines to 2', () => {
    const result = toPlainText('<p>A</p><div></div><div></div><p>B</p>');
    assert.ok(!result.includes('\n\n\n'), `got: ${JSON.stringify(result)}`);
});

test('toPlainText normalises &nbsp; to a regular space', () => {
    assert.equal(toPlainText('word&nbsp;word'), 'word word');
});

test('toPlainText strips HTML comments', () => {
    assert.equal(toPlainText('Before<!-- hidden -->After'), 'BeforeAfter');
});

test('toPlainText renders inline elements inside <a> as link text', () => {
    // <code> inside <a>: backtick markers must contribute to link text,
    // not be emitted to output before the URL reference.
    assert.equal(
        toPlainText('<a href="https://example.com"><code>npm install</code></a>'),
        '`npm install` [https://example.com]'
    );
    // <strong> inside <a>: bold markers are invisible in plain text, text still captured
    assert.equal(
        toPlainText('<a href="https://example.com"><strong>Click here</strong></a>'),
        'Click here [https://example.com]'
    );
    // <code> inside <a> inside <li>: link text must be part of the list item
    const result = toPlainText('<ul><li><a href="https://example.com"><code>pkg</code></a></li></ul>');
    assert.ok(result.includes('`pkg` [https://example.com]'), `got: ${result}`);
});
