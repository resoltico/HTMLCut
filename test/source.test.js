import test from 'node:test';
import assert from 'node:assert/strict';
import { writeFile } from 'node:fs/promises';
import { join } from 'node:path';
import { parseByteSize, readSource } from '../src/source.js';
import { withTempDir, withServer } from './helpers.js';

test('parseByteSize: accepts friendly units and rejects invalid values', () => {
    assert.equal(parseByteSize('1kb'), 1024);
    assert.equal(parseByteSize('1.5mb'), 1572864);
    assert.equal(parseByteSize(64), 64);
    assert.throws(() => parseByteSize('banana'), /Invalid byte size/);
});

test('readSource: reads local files and resolves them to absolute paths', async () => {
    await withTempDir(async dir => {
        const inputPath = join(dir, 'input.html');
        await writeFile(inputPath, '<p>hello</p>', 'utf8');

        const source = await readSource(inputPath);
        assert.equal(source.kind, 'file');
        assert.match(source.input, /input\.html$/);
        assert.equal(source.text, '<p>hello</p>');
        assert.equal(source.baseUrl, null);
    });
});

test('readSource: enforces response size limits while reading URLs', async () => {
    await withServer((req, res) => {
        res.writeHead(200, { 'content-type': 'text/html' });
        res.end('0123456789');
    }, async ({ url }) => {
        await assert.rejects(
            () => readSource(url, { maxBytes: 5 }),
            error => error.message.includes('Response exceeds 5 bytes limit')
        );
    });
});

test('readSource: surfaces HTTP failures and file-system failures clearly', async () => {
    await withServer((req, res) => {
        res.writeHead(404, 'Not Found');
        res.end();
    }, async ({ url }) => {
        await assert.rejects(
            () => readSource(url),
            error => error.message.includes('HTTP 404 Not Found')
        );
    });

    await assert.rejects(
        () => readSource('/definitely/not/here.html'),
        error => error.message.includes('Could not access file')
    );
});

test('readSource: fails cleanly when the input path is not a readable file stream', async () => {
    await withTempDir(async dir => {
        await assert.rejects(
            () => readSource(dir),
            error => error.message.includes('Could not read file')
        );
    });
});
