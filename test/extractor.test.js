import test from 'node:test';
import assert from 'node:assert/strict';
import { extractMatches } from '../src/extractor.js';

test('extractMatches: literal mode returns inner content and offsets', () => {
    const matches = extractMatches('<article>Hello</article>', {
        from: '<article>',
        to: '</article>',
    });

    assert.deepEqual(matches, [{
        index: 1,
        html: 'Hello',
        range: { start: 9, end: 14 },
        innerRange: { start: 9, end: 14 },
        outerRange: { start: 0, end: 24 },
    }]);
});

test('extractMatches: outer capture preserves the delimiters', () => {
    const matches = extractMatches('<section>Hi</section>', {
        from: '<section>',
        to: '</section>',
        capture: 'outer',
    });

    assert.equal(matches[0].html, '<section>Hi</section>');
    assert.deepEqual(matches[0].range, { start: 0, end: 21 });
});

test('extractMatches: regex mode supports repeated extraction', () => {
    const matches = extractMatches('<H2>One</H2><h2>Two</h2>', {
        from: '<h2>',
        to: '</h2>',
        mode: 'regex',
        flags: 'i',
        all: true,
    });

    assert.deepEqual(matches.map(match => match.html), ['One', 'Two']);
});

test('extractMatches: missing end after a later start is a hard failure', () => {
    assert.throws(
        () => extractMatches('<p>One</p><p>Two', {
            from: '<p>',
            to: '</p>',
            all: true,
        }),
        error => error.message.includes('End pattern was not found')
    );
});

test('extractMatches: invalid regex is surfaced as a usage error', () => {
    assert.throws(
        () => extractMatches('<p>One</p>', {
            from: '[bad',
            to: '</p>',
            mode: 'regex',
        }),
        error => error.message.includes('Invalid regular expression')
    );
});

test('extractMatches: missing start, unknown capture, and unknown mode all fail clearly', () => {
    assert.throws(
        () => extractMatches('<p>One</p>', {
            from: '<article>',
            to: '</article>',
        }),
        error => error.message.includes('Start pattern was not found')
    );

    assert.throws(
        () => extractMatches('<p>One</p>', {
            from: '<p>',
            to: '</p>',
            capture: 'sideways',
        }),
        error => error.message.includes('Unknown capture mode')
    );

    assert.throws(
        () => extractMatches('<p>One</p>', {
            from: '<p>',
            to: '</p>',
            mode: 'glob',
        }),
        error => error.message.includes('Unknown pattern mode')
    );
});
