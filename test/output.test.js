import test from 'node:test';
import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import { buildReport, formatPayload, getBundlePaths, writeBundle } from '../src/output.js';
import { withTempDir } from './helpers.js';

function createReport(bundle = null) {
    return buildReport({
        version: '2.0.0',
        source: {
            kind: 'file',
            input: '/tmp/input.html',
            bytesRead: 42,
        },
        baseUrl: null,
        pattern: {
            from: '<p>',
            to: '</p>',
            mode: 'literal',
            flags: 'u',
            capture: 'inner',
            all: false,
        },
        durationMs: 7,
        matches: [{
            index: 1,
            range: { start: 0, end: 3 },
            innerRange: { start: 0, end: 3 },
            outerRange: { start: 0, end: 7 },
            html: '<p>One</p>',
            text: 'One',
        }],
        bundle,
    });
}

test('formatPayload: renders text, html, json, and rejects unknown formats', () => {
    const report = createReport();
    assert.equal(formatPayload(report, 'text'), 'One');
    assert.equal(formatPayload(report, 'html'), '<p>One</p>');
    assert.equal(JSON.parse(formatPayload(report, 'json')).stats.matchCount, 1);
    assert.equal(formatPayload(report, 'none'), '');
    assert.throws(() => formatPayload(report, 'yaml'), /Unknown output format/);
});

test('writeBundle: no-ops when bundle output is not requested', async () => {
    await assert.doesNotReject(() => writeBundle(createReport()));
});

test('writeBundle: writes the selection bundle to disk', async () => {
    await withTempDir(async dir => {
        const bundle = getBundlePaths(dir);
        const report = createReport(bundle);
        await writeBundle(report);

        const html = await readFile(bundle.html, 'utf8');
        const text = await readFile(bundle.text, 'utf8');
        const json = JSON.parse(await readFile(bundle.report, 'utf8'));

        assert.match(html, /<!DOCTYPE html>/);
        assert.equal(text, 'One');
        assert.equal(json.documentTitle, 'input');
        assert.equal(json.bundle.html, bundle.html);
    });
});
