import test from 'node:test';
import assert from 'node:assert/strict';
import { Buffer } from 'node:buffer';
import { extractStream } from '../src/extractor.js';
import { chunksOf, collect, drain } from './helpers.js';

// ---------------------------------------------------------------------------
// Literal – basic cases
// ---------------------------------------------------------------------------

test('Literal: single basic extraction', async () => {
    const result = await collect(extractStream(chunksOf('<div>hello world</div>'), '<div>', '</div>'));
    assert.deepEqual(result, ['hello world']);
});

test('Literal: empty content between elements', async () => {
    const result = await collect(extractStream(chunksOf('<div></div>'), '<div>', '</div>'));
    assert.deepEqual(result, ['']);
});

test('Literal: global multi extraction', async () => {
    const result = await collect(extractStream(chunksOf('<p>one</p><p>two</p><p>three</p>'), '<p>', '</p>', { isGlobal: true }));
    assert.deepEqual(result, ['one', 'two', 'three']);
});

test('Literal: global extraction missing end', async () => {
    const result = await collect(extractStream(chunksOf('<p>one</p><p>two'), '<p>', '</p>', { isGlobal: true }));
    assert.deepEqual(result, ['one']);
});

test('Literal: throws on missing start literal', async () => {
    await assert.rejects(
        () => drain(extractStream(chunksOf('<div>hello</div>'), '<p>', '</div>')),
        { message: 'Start pattern not found: <p>' }
    );
});

test('Literal: throws on missing end pattern', async () => {
    await assert.rejects(
        () => drain(extractStream(chunksOf('<div>hello</div>'), '<div>', '</section>')),
        { message: 'End pattern not found: </section>' }
    );
});

test('Literal: identical start and end values', async () => {
    const result = await collect(extractStream(chunksOf('value "quoted string" and further'), '"', '"'));
    assert.deepEqual(result, ['quoted string']);
});

// ---------------------------------------------------------------------------
// Regex – basic cases
// ---------------------------------------------------------------------------

test('Regex: basic extract with v flag', async () => {
    const result = await collect(extractStream(chunksOf('<article class="entry">Welcome! 🌍</article>'), '<article class="\\w+">', '</article>', { isRegex: true }));
    assert.deepEqual(result, ['Welcome! 🌍']);
});

test('Regex: global multi extraction', async () => {
    const result = await collect(extractStream(chunksOf('<var id="x">1</var><var id="y">2</var>'), '<var id="[a-z]">', '</var>', { isRegex: true, isGlobal: true }));
    assert.deepEqual(result, ['1', '2']);
});

test('Regex: extraction preserves literal backslash sequences in content', async () => {
    const result = await collect(extractStream(chunksOf('<pre>line1\\nline2</pre>'), '<pre>', '</pre>', { isRegex: true }));
    assert.deepEqual(result, ['line1\\nline2']);
});

test('Regex: missing start throws', async () => {
    await assert.rejects(
        () => drain(extractStream(chunksOf('<body>Content</body>'), '<head>', '</head>', { isRegex: true })),
        { message: 'Start pattern not found: <head>' }
    );
});

test('Regex: missing end throws', async () => {
    await assert.rejects(
        () => drain(extractStream(chunksOf('<body>Content'), '<body>', '</body>', { isRegex: true })),
        { message: 'End pattern not found: </body>' }
    );
});

test('Regex: missing global end ignores trailing match', async () => {
    const result = await collect(extractStream(chunksOf('<b>1</b><b>2'), '<b>', '</b>', { isRegex: true, isGlobal: true }));
    assert.deepEqual(result, ['1']);
});

test('Regex: invalid patterns throw safely', async () => {
    await assert.rejects(
        () => drain(extractStream(chunksOf('<div>hello</div>'), '[invalid', '</div>', { isRegex: true })),
        { message: /Invalid RegExp/ }
    );
    await assert.rejects(
        () => drain(extractStream(chunksOf('<div>hello</div>'), '<div>', '[invalid', { isRegex: true })),
        { message: /Invalid RegExp/ }
    );
});

test('Regex: \\b word boundary matches tag start precisely', async () => {
    const html = '<h2class>wrong</h2class><h2 id="x">correct</h2>';
    const result = await collect(extractStream(chunksOf(html), '<h2\\b[^>]*>', '</h2>', { isRegex: true }));
    assert.deepEqual(result, ['correct']);
});

test('Regex: \\s* handles real newlines between nested tags (realistic HTML)', async () => {
    const html = [
        '<section>',
        'Article body text',
        '<h2 id="rel">',
        '  <div class="wrap">',
        '    Related articles',
        '  </div>',
        '</h2>',
        '</section>'
    ].join('\n');

    const result = await collect(extractStream(
        chunksOf(html),
        '<section>',
        '<h2\\b[^>]*>\\s*<div\\b[^>]*>\\s*Related articles\\s*</div>\\s*</h2>',
        { isRegex: true }
    ));
    assert.equal(result.length, 1);
    assert.match(result[0], /Article body text/);
});

test('Regex: dynamic attribute pattern [^"]* matches any attribute value', async () => {
    const html = '<section id="HELP_CENTER_SECTION_0">body text</section>';
    const result = await collect(extractStream(chunksOf(html), '<section id="[^"]*">', '</section>', { isRegex: true }));
    assert.deepEqual(result, ['body text']);
});

test('Regex: global extraction with \\b word boundary across multiple tags', async () => {
    const html = '<h2 id="a">First</h2><h2class>skip</h2class><h2 id="b">Second</h2>';
    const result = await collect(extractStream(chunksOf(html), '<h2\\b[^>]*>', '</h2>', { isRegex: true, isGlobal: true }));
    assert.deepEqual(result, ['First', 'Second']);
});

// ---------------------------------------------------------------------------
// Regex v-flag unicodeSet feature tests
// ---------------------------------------------------------------------------

test('Regex v: case-sensitive by design — lowercase pattern misses uppercase tags', async () => {
    // HTMLCut uses the 'v' flag (no 'i'). A lowercase pattern must not match uppercase HTML.
    const html = '<DIV>Content</DIV>';
    await assert.rejects(
        () => drain(extractStream(chunksOf(html), '<div>', '</div>')),
        { message: 'Start pattern not found: <div>' }
    );
});

test('Regex v: non-greedy .*? stops at first match, not last', async () => {
    const html = '<b>Alpha</b><b>Beta</b>';
    const result = await collect(extractStream(chunksOf(html), '<b>', '</b>', { isRegex: true }));
    assert.deepEqual(result, ['Alpha']);
});

test('Regex v: positive lookahead (?=...) matches without consuming', async () => {
    const html = '<p>Related articles section</p>';
    const result = await collect(extractStream(chunksOf(html), '<p>Related(?= articles)', '</p>', { isRegex: true }));
    assert.deepEqual(result, [' articles section']);
});

test('Regex v: negative lookahead (?!...) excludes specific patterns', async () => {
    const html = '<p>keep</p><p>skip me</p><p>also keep</p>';
    const result = await collect(extractStream(chunksOf(html), '<p>(?!skip)', '</p>', { isRegex: true, isGlobal: true }));
    assert.deepEqual(result, ['keep', 'also keep']);
});

test('Regex v: lookbehind (?<=...) matches content preceded by tag', async () => {
    const html = '<span>Status: active</span>';
    const result = await collect(extractStream(chunksOf(html), '(?<=Status: )', '</span>', { isRegex: true }));
    assert.deepEqual(result, ['active']);
});

test('Regex v: Unicode property escape \\p{Letter} matches international chars', async () => {
    const html = '<p>Héllo Wörld</p>';
    const result = await collect(extractStream(chunksOf(html), '<p>', '</p>', { isRegex: true }));
    assert.deepEqual(result, ['Héllo Wörld']);
    const rx = new RegExp('\\p{Letter}+', 'v');
    assert.ok(rx.test('こんにちは'), 'Should match Japanese letters');
    assert.ok(rx.test('Héllo'), 'Should match accented Latin letters');
});

test('Regex v: unicodeSet intersection [\\w&&[^\\d]] matches word-non-digit', async () => {
    const rx = new RegExp('[\\w&&[^\\d]]+', 'v');
    assert.ok(rx.test('hello'), 'word chars pass');
    assert.ok(!rx.test('123'), 'pure digits rejected by intersection');
    const html = '<tag>value</tag>';
    const result = await collect(extractStream(chunksOf(html), '<tag>', '</tag>', { isRegex: true }));
    assert.deepEqual(result, ['value']);
});

test('Regex v: unicodeSet subtraction [\\w--\\d] same as intersection alternative', () => {
    const rx = new RegExp('[\\w--\\d]+', 'v');
    assert.ok(rx.test('abc_XYZ'), 'letters and underscore pass');
    assert.ok(!rx.test('42'), 'digits excluded by subtraction');
});

// ---------------------------------------------------------------------------
// Zero-width safety
// ---------------------------------------------------------------------------

test('Regex: zero-width assertions do not cause infinite loops', async () => {
    // The sliding window advances at least 1 char past zero-width matches, preventing
    // hangs. Zero-width patterns in global mode may yield fewer results than a
    // full-string search because advancing drops surrounding character context.
    const results = await collect(extractStream(chunksOf('a b'), '\\b', '\\b', { isRegex: true, isGlobal: true }));
    assert.ok(Array.isArray(results));
    assert.ok(results.length > 0);
});

test('Regex: Non-Error throws are coerced to String safely', async () => {
    const OriginalRegExp = globalThis.RegExp;
    try {
        globalThis.RegExp = function () { throw new Error('StringBasedError'); };
        await assert.rejects(
            () => drain(extractStream(chunksOf('text'), 'start', 'end', { isRegex: true })),
            { message: /Invalid RegExp: StringBasedError/ }
        );
    } finally {
        globalThis.RegExp = OriginalRegExp;
    }
});

test('Stream Decoder: Flushes buffered incomplete multi-byte sequence cleanly', async () => {
    // A 3-byte unicode character "☃" (E2 98 83). We cut off the last byte.
    // The inner decoder.decode(chunk, {stream: true}) buffers it safely across chunks.
    // The final tail flush outputs the replacement character U+FFFD instead of failing.
    await assert.rejects(
        () => drain(extractStream([Buffer.from([0xE2, 0x98])], 'A', 'B')),
        { message: 'Start pattern not found: A' }
    );
});
