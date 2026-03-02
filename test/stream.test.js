/**
 * @fileoverview Tests for extractStream() — streaming mechanics, edge cases,
 * and timeout guards. Uses the internal timeoutMs option to exercise
 * guards without real delays.
 */
import test from 'node:test';
import assert from 'node:assert/strict';
import { Buffer } from 'node:buffer';
import { extractStream } from '../src/extractor.js';
import { chunksOf, drain } from './helpers.js';

// ---------------------------------------------------------------------------
// extractStream – timeout guard
// ---------------------------------------------------------------------------

test('extractStream: timeout throws on 0ms limit with string chunk', async () => {
    // A generator that yields real chunks; timeoutMs=0 triggers immediately
    async function* slow() {
        yield '<div>text</div>';
        yield '<div>more</div>';
    }
    await assert.rejects(
        () => drain(extractStream(slow(), '<div>', '</div>', { timeoutMs: 0 })),
        { message: 'Extraction timeout: pattern too complex or input too large' }
    );
});

// ---------------------------------------------------------------------------
// extractStream – window trimming (regex path, no start match, large window)
// ---------------------------------------------------------------------------

test('extractStream: regex path trims oversized window when start not found', async () => {
    // Window larger than STREAM_WINDOW_MAX (32768) with no start pattern.
    // The trim path runs and the generator ultimately throws "Start pattern not found".
    const OVER_MAX = 33000;
    const bigChunk = 'x'.repeat(OVER_MAX);
    await assert.rejects(
        () => drain(extractStream(chunksOf(bigChunk), '<div>', '</div>', { isRegex: true })),
        { message: 'Start pattern not found: <div>' }
    );
});

// ---------------------------------------------------------------------------
// extractStream – basic literal extraction (Buffer chunk)
// ---------------------------------------------------------------------------

test('extractStream: literal extraction from Buffer chunk', async () => {
    const results = [];
    const buf = Buffer.from('<div>buffered</div>', 'utf8');
    for await (const frag of extractStream(chunksOf(buf), '<div>', '</div>')) {
        results.push(frag);
    }
    assert.deepEqual(results, ['buffered']);
});

// ---------------------------------------------------------------------------
// extractStream – match split across two chunks (literal)
// ---------------------------------------------------------------------------

test('extractStream: literal match spanning two chunks', async () => {
    // Split mid-content: chunk1 = '<div>hel', chunk2 = 'lo</div>'
    const results = [];
    for await (const frag of extractStream(chunksOf('<div>hel', 'lo</div>'), '<div>', '</div>')) {
        results.push(frag);
    }
    assert.deepEqual(results, ['hello']);
});

// ---------------------------------------------------------------------------
// extractStream – multi-byte UTF-8 character split across chunk boundary
// ---------------------------------------------------------------------------

test('extractStream: multi-byte char (é) split across chunk boundary is decoded correctly', async () => {
    // 'é' = U+00E9 = bytes [0xC3, 0xA9] in UTF-8.
    // chunk1 ends with the first byte (0xC3); chunk2 starts with the second (0xA9).
    // TextDecoder with { stream: true } must hold the incomplete byte across chunks.
    const chunk1 = Buffer.concat([Buffer.from('<div>caf'), Buffer.from([0xC3])]);
    const chunk2 = Buffer.concat([Buffer.from([0xA9]), Buffer.from('</div>')]);
    const results = [];
    for await (const frag of extractStream(chunksOf(chunk1, chunk2), '<div>', '</div>')) {
        results.push(frag);
    }
    assert.deepEqual(results, ['caf\u00E9']);
});

// ---------------------------------------------------------------------------
// extractStream – window trim on literal path (start pattern longer than content)
// ---------------------------------------------------------------------------

test('extractStream: literal window trim keeps overlap when start not found', async () => {
    // Feed data that doesn't contain the start pattern.
    // buf.length > startPattern.length triggers the trim path.
    const PAD_SIZE = 20;
    const bigPad = 'y'.repeat(PAD_SIZE);
    await assert.rejects(
        () => drain(extractStream(chunksOf(bigPad, 'no match here'), '<START>', '<END>')),
        { message: 'Start pattern not found: <START>' }
    );
});

test('extractStream: 1-char start pattern clears buffer when not found (slice(-0) guard)', async () => {
    // slice(-0) === slice(0) in JS returns the full string — without the guard, a
    // 1-char pattern that is never found would never trim the buffer.
    // This test verifies the error is thrown and the match does NOT bleed across
    // chunk boundaries incorrectly: chunk1 ends with '>', chunk2 has no pattern.
    await assert.rejects(
        () => drain(extractStream(chunksOf('no match', '>>no end here'), '>', '<')),
        // start IS found (there is a '>') so we expect end-not-found, not start-not-found
        { message: 'End pattern not found: <' }
    );
    // And when the 1-char start is absent entirely, the buffer is cleared each chunk.
    await assert.rejects(
        () => drain(extractStream(chunksOf('aaa', 'bbb', 'ccc'), '>', '<')),
        { message: 'Start pattern not found: >' }
    );
});
