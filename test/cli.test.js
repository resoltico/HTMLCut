import test from 'node:test';
import assert from 'node:assert/strict';
import { execFile, spawn } from 'node:child_process';
import { promisify } from 'node:util';
import http from 'node:http';
import { join } from 'node:path';
import { tmpdir } from 'node:os';
import { readFile, writeFile, rm, mkdir, truncate, readdir } from 'node:fs/promises';
import { Buffer } from 'node:buffer';
import { setInterval, clearInterval, setImmediate, setTimeout } from 'node:timers';

const execFileAsync = promisify(execFile);
const CLI_PATH = join(process.cwd(), 'src/cli.js');

test('CLI: Executing with --help exits cleanly successfully', async () => {
    const { stdout, stderr } = await execFileAsync(CLI_PATH, ['--help'], { env: { ...process.env, NODE_NO_WARNINGS: '1' } });
    assert.match(stdout, /HTMLCut v\d+\.\d+\.\d+/);
    assert.equal(stderr, '');
});

test('CLI: Executing with --version prints semver and exits cleanly', async () => {
    const { stdout, stderr } = await execFileAsync(CLI_PATH, ['--version'], { env: { ...process.env, NODE_NO_WARNINGS: '1' } });
    assert.match(stdout, /^\d+\.\d+\.\d+\n$/);
    assert.equal(stderr, '');
});

test('CLI: Missing arguments throws an error cleanly', async () => {
    try {
        await execFileAsync(CLI_PATH, ['--input', 'invalid']);
        assert.fail('Should have failed execution with missing arguments');
    } catch (err) {
        const errOutput = err.stderr || '';
        assert.match(errOutput, /Missing required arguments/);
        assert.equal(err.code, 1);
    }
});

test('CLI: Missing --input argument throws an error cleanly', async () => {
    try {
        await execFileAsync(CLI_PATH, ['--start', '<div>', '--end', '</div>']);
        assert.fail('Should have failed with missing input');
    } catch (err) {
        assert.match(err.stderr || '', /Missing required arguments/);
        assert.equal(err.code, 1);
    }
});

test('CLI: Parses local file successfully', async () => {
    const tmpDir = join(tmpdir(), `htmlcut-test-${Date.now()}`);
    await mkdir(tmpDir, { recursive: true });

    const inputPath = join(tmpDir, 'test.html');
    await writeFile(inputPath, '<div>Success Local</div>', 'utf8');

    const outputPath = join(tmpDir, 'out');

    try {
        const { stdout, stderr } = await execFileAsync(CLI_PATH, [
            '-i', inputPath,
            '-s', '<div>',
            '-e', '</div>',
            '-o', outputPath
        ], { env: { ...process.env, NODE_NO_WARNINGS: '1' } });

        assert.match(stderr, /Successfully extracted 1 fragment\b/);
        assert.equal(stdout, '');
    } finally {
        await rm(tmpDir, { recursive: true, force: true });
    }
});

test('CLI: Fetch integration limits payload correctly over 50MB and cancels socket', async () => {
    // Start local http server mocking a stream
    const server = http.createServer((req, res) => {
        res.writeHead(200, { 'Content-Type': 'text/html' });

        // Spam massive chunks
        const massiveChunk = 'A'.repeat(5 * 1024 * 1024); // 5MB

        let sent = 0;
        const interval = setInterval(() => {
            if (sent >= 15) { // Try to send 75MB over the limit
                res.end();
                clearInterval(interval);
                return;
            }
            res.write(massiveChunk);
            sent++;
        }, 10);

        // When the client `.cancel()` invokes, the connection breaks cleanly
        res.on('close', () => {
            clearInterval(interval);
        });
    });

    await new Promise(resolve => server.listen(0, resolve));
    const port = server.address().port;
    const url = `http://localhost:${port}`;

    try {
        await execFileAsync(CLI_PATH, [
            '-i', url,
            '-s', '<none>',
            '-e', '</none>'
        ]);
        assert.fail('Should have aborted fetch based on payload size');
    } catch (err) {
        const errOutput = err.stderr || '';
        assert.match(errOutput, /Payload exceeds 50MB limit/);
        assert.equal(err.code, 1);
    } finally {
        server.close();
    }
});

test('CLI: Executing with --history leverages isolated db and prints valid JSON', async () => {
    const tmpDir = join(tmpdir(), `htmlcut-test-hist-${Date.now()}`);
    await mkdir(tmpDir, { recursive: true });
    const isolatedDbPath = join(tmpDir, 'test_hist.db');
    const envOptions = { env: { ...process.env, NODE_NO_WARNINGS: '1', HTMLCUT_DB_PATH: isolatedDbPath } };

    try {
        // 1. Empty DB run
        const { stdout: outEmpty, stderr: errEmpty } = await execFileAsync(CLI_PATH, ['--history'], envOptions);
        assert.deepEqual(JSON.parse(outEmpty), {});
        assert.equal(errEmpty, '');

        // 2. Populate DB
        const inputPath = join(tmpDir, 'test.html');
        await writeFile(inputPath, '<div>Success</div>', 'utf8');
        const outputPath = join(tmpDir, 'out');

        await execFileAsync(CLI_PATH, ['-i', inputPath, '-s', '<div>', '-e', '</div>', '-o', outputPath, '--track'], envOptions);

        // 3. Verify Populated DB Output
        const { stdout: outPopulated } = await execFileAsync(CLI_PATH, ['--history'], envOptions);
        const parsedHistory = JSON.parse(outPopulated);
        assert.ok(parsedHistory.successful, 'Expected successful property');
        assert.equal(parsedHistory.successful[0].source, inputPath);
    } finally {
        await rm(tmpDir, { recursive: true, force: true });
    }
});

test('CLI: Throws on 404 fetch response', async () => {
    const server = http.createServer((req, res) => {
        res.writeHead(404, 'Not Found');
        res.end();
    });
    await new Promise(resolve => server.listen(0, resolve));
    const port = server.address().port;
    const url = `http://localhost:${port}`;

    try {
        await execFileAsync(CLI_PATH, ['-i', url, '-s', '<a>', '-e', '</a>', '-t'], {
            env: { ...process.env, NODE_NO_WARNINGS: '1', HTMLCUT_DB_PATH: ':memory:' },
        });
        assert.fail('Should have failed on 404');
    } catch (err) {
        const errOutput = err.stderr || '';
        assert.match(errOutput, /HTTP 404: Not Found/);
        assert.equal(err.code, 1);
    } finally {
        server.close();
    }
});

test('CLI: Proceeds normally when content-length header is within limit', async () => {
    const body = '<p>Hello</p>';
    const server = http.createServer((req, res) => {
        res.writeHead(200, { 'Content-Type': 'text/html', 'Content-Length': String(body.length) });
        res.end(body);
    });
    await new Promise(resolve => server.listen(0, resolve));
    const port = server.address().port;
    const url = `http://localhost:${port}`;

    const tmpDir = join(tmpdir(), `htmlcut-test-${Date.now()}`);
    await mkdir(tmpDir, { recursive: true });

    try {
        const { stderr } = await execFileAsync(CLI_PATH, [
            '-i', url, '-s', '<p>', '-e', '</p>', '-o', join(tmpDir, 'out')
        ], { env: { ...process.env, NODE_NO_WARNINGS: '1' } });
        assert.match(stderr, /Successfully extracted 1 fragment\b/);
    } finally {
        server.close();
        await rm(tmpDir, { recursive: true, force: true });
    }
});

test('CLI: Throws on content-length header exceeding 50MB', async () => {
    const server = http.createServer((req, res) => {
        res.writeHead(200, { 'Content-Length': String(51 * 1024 * 1024) });
        res.end();
    });
    await new Promise(resolve => server.listen(0, resolve));
    const port = server.address().port;
    const url = `http://localhost:${port}`;

    try {
        await execFileAsync(CLI_PATH, ['-i', url, '-s', '<a>', '-e', '</a>']);
        assert.fail('Should have failed on content-length');
    } catch (err) {
        const errOutput = err.stderr || '';
        assert.match(errOutput, /Payload exceeds 50MB limit/);
        assert.equal(err.code, 1);
    } finally {
        server.close();
    }
});

test('CLI: Throws on empty response body (204 No Content)', async () => {
    const server = http.createServer((req, res) => {
        res.writeHead(204);
        res.end();
    });
    await new Promise(resolve => server.listen(0, resolve));
    const port = server.address().port;
    const url = `http://localhost:${port}`;

    try {
        await execFileAsync(CLI_PATH, ['-i', url, '-s', '<a>', '-e', '</a>']);
        assert.fail('Should fail on empty body');
    } catch (err) {
        const errOutput = err.stderr || '';
        assert.match(errOutput, /Empty response body/);
        assert.equal(err.code, 1);
    } finally {
        server.close();
    }
});

test('CLI: Throws on local file exceeding 50MB', async () => {
    const tmpDir = join(tmpdir(), `htmlcut-test-${Date.now()}`);
    await mkdir(tmpDir, { recursive: true });
    const inputPath = join(tmpDir, 'massive.html');
    await writeFile(inputPath, '');
    await truncate(inputPath, 51 * 1024 * 1024); // 51MB sparse file

    try {
        await execFileAsync(CLI_PATH, ['-i', inputPath, '-s', '<a>', '-e', '</a>']);
        assert.fail('Should have failed on massive file');
    } catch (err) {
        const errOutput = err.stderr || '';
        assert.match(errOutput, /File exceeds 50MB limit/);
        assert.equal(err.code, 1);
    } finally {
        await rm(tmpDir, { recursive: true, force: true });
    }
});

test('CLI: Throws on special file exceeding 50MB avoiding stat', async () => {
    if (process.platform === 'win32') { return; }
    try {
        await execFileAsync(CLI_PATH, ['-i', '/dev/zero', '-s', '<a>', '-e', '</a>']);
        assert.fail('Should have failed on massive stream file');
    } catch (err) {
        const errOutput = err.stderr || '';
        assert.match(errOutput, /File exceeds 50MB limit/);
        assert.equal(err.code, 1);
    }
});

test('CLI: Throws on output stream write failure (permission)', async () => {
    const tmpDir = join(tmpdir(), `htmlcut-test-${Date.now()}`);
    await mkdir(tmpDir, { recursive: true });

    // Invalid output path forces fs write to fail
    const outputPath = '/non_existent_folder_123456789/out';
    const inputPath = join(tmpDir, 'test.html');
    await writeFile(inputPath, '<div>Success</div>', 'utf8');

    try {
        await execFileAsync(CLI_PATH, ['-i', inputPath, '-s', '<div>', '-e', '</div>', '-o', outputPath, '--track'], {
            env: { ...process.env, HTMLCUT_DB_PATH: '/invalid/db/dir/test.db' }
        });
        assert.fail('Should fail on write');
    } catch (err) {
        const errOutput = err.stderr || '';
        assert.match(errOutput, /Outputs could not be written/);
        assert.equal(err.code, 1);
    } finally {
        await rm(tmpDir, { recursive: true, force: true });
    }
});

test('CLI: HTTPS positional argument fallback and native Error.cause serialization', async () => {
    try {
        // No '-i' flag, so 'https://127.0.0.1:0' is treated as positionals[0]
        // Port 0 will guarantee connection refused, triggering a native fetch TypeError with a nested .cause
        await execFileAsync(CLI_PATH, ['https://127.0.0.1:0', '-s', '<a>', '-e', '</a>']);
        assert.fail('Should fail on invalid https');
    } catch (err) {
        const errOutput = err.stderr || '';
        assert.match(errOutput, /Error: fetch failed:/);
        assert.match(errOutput, /connect/); // Verifies the error.cause ternary logic
        assert.equal(err.code, 1);
    }
});

test('CLI: Single extraction writes correct content to .html and .txt files', async () => {
    const tmpDir = join(tmpdir(), `htmlcut-test-${Date.now()}`);
    await mkdir(tmpDir, { recursive: true });

    try {
        const inputPath = join(tmpDir, 'input.html');
        await writeFile(inputPath, '<article><p>Hello World</p></article>', 'utf8');
        const outputBase = join(tmpDir, 'out');

        await execFileAsync(CLI_PATH, [
            '-i', inputPath, '-s', '<article>', '-e', '</article>', '-o', outputBase
        ], { env: { ...process.env, NODE_NO_WARNINGS: '1' } });

        // Find the generated file (has timestamp in name)
        const files = await readdir(tmpDir);
        const htmlFile = files.find(f => f.includes('htmlcut') && f.endsWith('.html'));
        const txtFile = files.find(f => f.includes('htmlcut') && f.endsWith('.txt'));

        const htmlContent = await readFile(join(tmpDir, htmlFile), 'utf8');
        const txtContent = await readFile(join(tmpDir, txtFile), 'utf8');

        assert.equal(htmlContent, '<!DOCTYPE html>\n<html lang="en">\n<head>\n    <meta charset="utf-8">\n    <meta name="viewport" content="width=device-width, initial-scale=1">\n    <title>input</title>\n</head>\n<body>\n<p>Hello World</p>\n</body>\n</html>');
        assert.equal(txtContent, 'Hello World');
    } finally {
        await rm(tmpDir, { recursive: true, force: true });
    }
});

test('CLI: Global extraction writes multiple fragments on separate lines', async () => {
    const tmpDir = join(tmpdir(), `htmlcut-test-${Date.now()}`);
    await mkdir(tmpDir, { recursive: true });

    try {
        const inputPath = join(tmpDir, 'input.html');
        await writeFile(inputPath, '<p>Alpha</p><p>Beta</p><p>Gamma</p>', 'utf8');
        const outputBase = join(tmpDir, 'out');

        const { stderr: globalStderr } = await execFileAsync(CLI_PATH, [
            '-i', inputPath, '-s', '<p>', '-e', '</p>', '-g', '-o', outputBase
        ], { env: { ...process.env, NODE_NO_WARNINGS: '1' } });

        assert.match(globalStderr, /Successfully extracted 3 fragments\b/);

        const files = await readdir(tmpDir);
        const txtFile = files.find(f => f.includes('htmlcut') && f.endsWith('.txt'));
        const txtContent = await readFile(join(tmpDir, txtFile), 'utf8');

        // Must be 3 separate lines — NOT "Alpha\nBeta\nGamma" with literal backslash-n
        const lines = txtContent.split('\n');
        assert.equal(lines.length, 3);
        assert.equal(lines[0], 'Alpha');
        assert.equal(lines[1], 'Beta');
        assert.equal(lines[2], 'Gamma');
    } finally {
        await rm(tmpDir, { recursive: true, force: true });
    }
});

test('CLI: HTML stripping safely ignores ">" inside attributes', async () => {
    const tmpDir = join(tmpdir(), `htmlcut-test-${Date.now()}`);
    await mkdir(tmpDir, { recursive: true });

    try {
        // This input contains `>` characters nested cleanly within a quoted attribute.
        const inputPath = join(tmpDir, 'input.html');
        await writeFile(inputPath, '<div data-rule="a > b">Real Text <span data-type="<test>">Inside</span></div>', 'utf8');
        const outputBase = join(tmpDir, 'out');

        await execFileAsync(CLI_PATH, [
            '-i', inputPath, '-s', '<div data-rule="a > b">', '-e', '</div>', '-o', outputBase
        ], { env: { ...process.env, NODE_NO_WARNINGS: '1' } });

        const files = await readdir(tmpDir);
        const txtFile = files.find(f => f.includes('htmlcut') && f.endsWith('.txt'));
        const txtContent = await readFile(join(tmpDir, txtFile), 'utf8');

        assert.equal(txtContent, 'Real Text Inside');
    } finally {
        await rm(tmpDir, { recursive: true, force: true });
    }
});

test('CLI: HTML entity decoding in .txt output converts common entities', async () => {
    const tmpDir = join(tmpdir(), `htmlcut-test-${Date.now()}`);
    await mkdir(tmpDir, { recursive: true });

    try {
        // Input contains several HTML entities that should be decoded in the .txt file.
        const rawHtml = '<p>5 &lt; 10 &amp; &quot;quoted&quot; &amp; it&#39;s &nbsp;fine &mdash; done &rsquo;s</p>';
        const inputPath = join(tmpDir, 'input.html');
        await writeFile(inputPath, rawHtml, 'utf8');
        const outputBase = join(tmpDir, 'out');

        await execFileAsync(CLI_PATH, [
            '-i', inputPath, '-s', '<p>', '-e', '</p>', '-o', outputBase
        ], { env: { ...process.env, NODE_NO_WARNINGS: '1' } });

        const files = await readdir(tmpDir);
        const txtFile = files.find(f => f.includes('htmlcut') && f.endsWith('.txt'));
        const txtContent = await readFile(join(tmpDir, txtFile), 'utf8');

        assert.ok(txtContent.includes('5 < 10'), `Expected decoded <, got: ${txtContent}`);
        assert.ok(!txtContent.includes('&amp;'), `Expected &amp; to be decoded, got: ${txtContent}`);
        assert.ok(txtContent.includes('"quoted"'), `Expected decoded ", got: ${txtContent}`);
        assert.ok(txtContent.includes("it's"), `Expected decoded ', got: ${txtContent}`);
        assert.ok(txtContent.includes('\u2014'), `Expected decoded &mdash; (—), got: ${txtContent}`);
        assert.ok(txtContent.includes('\u2019'), `Expected decoded &rsquo; ('), got: ${txtContent}`);
    } finally {
        await rm(tmpDir, { recursive: true, force: true });
    }
});

test('CLI: Reads from stdin when -i - is specified', async () => {
    // execFileAsync with options.input hangs on Node.js v24+ because the child's
    // stdin EOF is never delivered through that API. Use spawn + manual stdin.end()
    // instead — the same pattern used by the size-guard test below.
    const tmpDir = join(tmpdir(), `htmlcut-test-${Date.now()}`);
    await mkdir(tmpDir, { recursive: true });
    const outputBase = join(tmpDir, 'out');

    await new Promise((resolve, reject) => {
        const child = spawn(process.execPath, [
            '--no-warnings=ExperimentalWarning', CLI_PATH,
            '-i', '-', '-s', '<h1>', '-e', '</h1>', '-o', outputBase,
        ], { env: { ...process.env, NODE_NO_WARNINGS: '1' } });

        let stdout = '';
        let stderr = '';
        child.stdout.on('data', d => { stdout += d; });
        child.stderr.on('data', d => { stderr += d; });

        child.stdin.write('<h1>Stdin Title</h1>');
        child.stdin.end();

        child.on('close', async code => {
            try {
                assert.equal(code, 0, `Expected exit 0. stderr=${stderr}`);
                assert.match(stderr, /Successfully extracted 1 fragment\b/);
                assert.equal(stdout, '');

                const files = await readdir(tmpDir);
                const htmlFile = files.find(f => f.includes('htmlcut') && f.endsWith('.html'));
                const txtFile = files.find(f => f.includes('htmlcut') && f.endsWith('.txt'));

                const htmlContent = await readFile(join(tmpDir, htmlFile), 'utf8');
                const txtContent = await readFile(join(tmpDir, txtFile), 'utf8');

                // '-' is not a meaningful source; title must fall back to the generic label,
                // not the literal '-' character.
                assert.ok(htmlContent.includes('<title>HTMLCut Extraction</title>'),
                    `Expected fallback title for stdin source, got: ${htmlContent}`);
                assert.equal(txtContent, 'Stdin Title');

                await rm(tmpDir, { recursive: true, force: true });
                resolve();
            } catch (err) {
                reject(err);
            }
        });
        child.on('error', reject);
    });
});

test('CLI: Reads from stdin with --regex pattern matching attributes', async () => {
    // Exercises the regex extraction code path end-to-end through stdin.
    // A literal stdin test exists above; this one specifically targets --regex so
    // regressions in the regex branch of extractStream are caught at the CLI level.
    // curl without -L does NOT follow redirects — fetch() does. This test uses a
    // controlled local payload so the content is never redirect-dependent.
    const tmpDir = join(tmpdir(), `htmlcut-test-${Date.now()}`);
    await mkdir(tmpDir, { recursive: true });
    const outputBase = join(tmpDir, 'out');

    await new Promise((resolve, reject) => {
        const child = spawn(process.execPath, [
            '--no-warnings=ExperimentalWarning', CLI_PATH,
            '-i', '-', '-s', '<h1[^>]*>', '-e', '</h1>', '--regex', '-o', outputBase,
        ], { env: { ...process.env, NODE_NO_WARNINGS: '1' } });

        let stdout = '';
        let stderr = '';
        child.stdout.on('data', d => { stdout += d; });
        child.stderr.on('data', d => { stderr += d; });

        child.stdin.write('<header><h1 class="main-title">Node.js</h1></header>');
        child.stdin.end();

        child.on('close', async code => {
            try {
                assert.equal(code, 0, `Expected exit 0. stderr=${stderr}`);
                assert.match(stderr, /Successfully extracted 1 fragment\b/);
                assert.equal(stdout, '');

                const files = await readdir(tmpDir);
                const htmlFile = files.find(f => f.includes('htmlcut') && f.endsWith('.html'));
                const txtFile = files.find(f => f.includes('htmlcut') && f.endsWith('.txt'));

                const htmlContent = await readFile(join(tmpDir, htmlFile), 'utf8');
                const txtContent = await readFile(join(tmpDir, txtFile), 'utf8');

                assert.ok(htmlContent.includes('<title>HTMLCut Extraction</title>'),
                    `Expected fallback title for stdin source, got: ${htmlContent}`);
                assert.equal(txtContent, 'Node.js');

                await rm(tmpDir, { recursive: true, force: true });
                resolve();
            } catch (err) {
                reject(err);
            }
        });
        child.on('error', reject);
    });
});

test('CLI: --track logs failed extraction to history', async () => {
    // Covers the catch-block shouldTrack=true branch where logExtraction SUCCEEDS.
    // The existing write-failure test uses an invalid DB path (so logExtraction also
    // fails), leaving the "inner catch not triggered" path uncovered. This test uses
    // a valid isolated DB so the failure is recorded cleanly.
    const tmpDir = join(tmpdir(), `htmlcut-test-${Date.now()}`);
    await mkdir(tmpDir, { recursive: true });
    const isolatedDbPath = join(tmpDir, 'test.db');
    const inputPath = join(tmpDir, 'test.html');
    await writeFile(inputPath, '<div>Content</div>', 'utf8');

    try {
        await execFileAsync(CLI_PATH, [
            '-i', inputPath, '-s', '<span>', '-e', '</span>', '--track',
        ], {
            env: { ...process.env, NODE_NO_WARNINGS: '1', HTMLCUT_DB_PATH: isolatedDbPath },
        });
        assert.fail('Should have failed — no <span> in input');
    } catch (err) {
        assert.equal(err.code, 1);
        assert.match(err.stderr || '', /Start pattern not found/);

        // The failure must be recorded in history
        const { stdout } = await execFileAsync(CLI_PATH, ['--history'], {
            env: { ...process.env, NODE_NO_WARNINGS: '1', HTMLCUT_DB_PATH: isolatedDbPath },
        });
        const history = JSON.parse(stdout);
        assert.ok(history.failed?.length >= 1, 'Expected at least one failed extraction in history');
    } finally {
        await rm(tmpDir, { recursive: true, force: true });
    }
});

test('CLI: --history uses default DB path when HTMLCUT_DB_PATH is unset', async () => {
    // Exercises the storage.js fallback: join(homedir(), '.htmlcut_history.db').
    // Override HOME so the default DB lands in a temp dir rather than the real home dir.
    const tmpDir = join(tmpdir(), `htmlcut-test-${Date.now()}`);
    await mkdir(tmpDir, { recursive: true });

    const env = { ...process.env, NODE_NO_WARNINGS: '1', HOME: tmpDir };
    delete env.HTMLCUT_DB_PATH;

    try {
        const { stdout, stderr } = await execFileAsync(CLI_PATH, ['--history'], { env });
        assert.equal(stderr, '');
        const history = JSON.parse(stdout); // must be valid JSON
        assert.equal(typeof history, 'object');
    } finally {
        await rm(tmpDir, { recursive: true, force: true });
    }
});

test('CLI: Stdin size guard rejects input exceeding 50MB', async () => {
    // Pump 1 MB chunks into stdin until the child exits after the size limit fires.
    // Never allocates more than 1 MB at a time — no 51 MB string in the test process.
    await new Promise((resolve, reject) => {
        const child = spawn(process.execPath, [
            '--no-warnings=ExperimentalWarning', CLI_PATH,
            '-i', '-', '-s', '<a>', '-e', '</a>',
        ]);

        let stderr = '';
        child.stderr.on('data', d => { stderr += d; });

        // EPIPE is expected: the child exits after the size limit fires and closes
        // its stdin reader while the pump may still be mid-write. Swallow it.
        child.stdin.on('error', err => {
            if (err.code !== 'EPIPE') { reject(err); }
        });

        const CHUNK = Buffer.alloc(1024 * 1024, 65); // reusable 1 MB buffer ('A')
        const pump = () => {
            if (child.exitCode !== null) { return; }
            const ok = child.stdin.write(CHUNK);
            if (ok) {
                setImmediate(pump);
            } else {
                child.stdin.once('drain', pump);
            }
        };
        pump();

        child.on('close', code => {
            if (code === 1 && /Input exceeds 50MB limit/.test(stderr)) {
                resolve();
            } else {
                reject(new Error(`Expected size limit error. code=${code} stderr=${stderr}`));
            }
        });
        child.on('error', reject);
    });
});

test('CLI: logExtraction failure in success path does not cause exit 1', async () => {
    // Extraction succeeds and files are written. HTMLCUT_DB_PATH points to a directory
    // that does not exist, so DatabaseSync will throw. The try/catch around the
    // success-path logExtraction call must absorb the error — exit code must be 0.
    const tmpDir = join(tmpdir(), `htmlcut-test-${Date.now()}`);
    await mkdir(tmpDir, { recursive: true });
    const inputPath = join(tmpDir, 'test.html');
    await writeFile(inputPath, '<h1>Hello</h1>', 'utf8');
    const outputPath = join(tmpDir, 'out');

    try {
        const { stdout, stderr } = await execFileAsync(CLI_PATH, [
            '-i', inputPath, '-s', '<h1>', '-e', '</h1>', '-o', outputPath, '--track',
        ], {
            env: { ...process.env, NODE_NO_WARNINGS: '1', HTMLCUT_DB_PATH: '/nonexistent/dir/db.sqlite' },
        });
        assert.match(stderr, /Successfully extracted 1 fragment\b/);
        assert.equal(stdout, '');
    } finally {
        await rm(tmpDir, { recursive: true, force: true });
    }
});

// Warning suppression is tested in full unit-isolation in test/suppress.test.js

test('CLI: Reads the entire response body successfully to completion in global mode', async () => {
    // Tests the closing generator cleanup in cli.js line 121
    const bodyParts = ['<p>First</p>', '<p>Second</p>'];
    const server = http.createServer((req, res) => {
        res.writeHead(200, { 'Content-Type': 'text/html' });
        res.write(bodyParts[0]);
        setTimeout(() => res.end(bodyParts[1]), 10);
    });
    await new Promise(resolve => server.listen(0, resolve));
    const port = server.address().port;
    const url = `http://localhost:${port}`;

    const tmpDir = join(tmpdir(), `htmlcut-test-${Date.now()}`);
    await mkdir(tmpDir, { recursive: true });

    try {
        const { stderr } = await execFileAsync(CLI_PATH, [
            '-i', url, '-s', '<p>', '-e', '</p>', '-o', join(tmpDir, 'out'), '-g'
        ], { env: { ...process.env, NODE_NO_WARNINGS: '1' } });
        assert.match(stderr, /Successfully extracted 2 fragments\b/);
    } finally {
        server.close();
        await rm(tmpDir, { recursive: true, force: true });
    }
});

test('CLI: Failed argument parsing with --track logs 0 duration correctly', async () => {
    const tmpDir = join(tmpdir(), `htmlcut-test-${Date.now()}`);
    await mkdir(tmpDir, { recursive: true });
    const isolatedDbPath = join(tmpDir, 'test.db');

    try {
        await execFileAsync(CLI_PATH, ['-s', '<a>', '-t'], {
            env: { ...process.env, NODE_NO_WARNINGS: '1', HTMLCUT_DB_PATH: isolatedDbPath }
        });
        assert.fail('Should fail on missing args');
    } catch (err) {
        assert.match(err.stderr || '', /Missing required arguments/);
        const { stdout } = await execFileAsync(CLI_PATH, ['--history'], {
            env: { ...process.env, NODE_NO_WARNINGS: '1', HTMLCUT_DB_PATH: isolatedDbPath }
        });
        const history = JSON.parse(stdout);
        assert.ok(history.failed.length >= 1, 'Expected at least one failed extraction log');
        assert.equal(typeof history.failed[0].duration_ms, 'number', 'durationMs must be a number');
        assert.equal(history.failed[0].duration_ms, 0, 'Failed-before-start log must record 0ms duration');
    } finally {
        await rm(tmpDir, { recursive: true, force: true });
    }
});

test('CLI: Expands relative links in both HTML and TXT outputs based on input source', async () => {
    const tmpDir = join(tmpdir(), `htmlcut-test-${Date.now()}`);
    await mkdir(tmpDir, { recursive: true });

    try {
        // Use a fake HTTP endpoint to get predictable "http://..." resolution
        const body = '<article><a href="about.html">About</a> <img src="img.png" alt="Logo"></article>';
        const server = http.createServer((req, res) => {
            res.writeHead(200, { 'Content-Type': 'text/html' });
            res.end(body);
        });
        await new Promise(resolve => server.listen(0, resolve));
        const port = server.address().port;
        const url = `http://localhost:${port}/docs/index.html`;

        const outputBase = join(tmpDir, 'out');

        await execFileAsync(CLI_PATH, [
            '-i', url, '-s', '<article>', '-e', '</article>', '-o', outputBase
        ], { env: { ...process.env, NODE_NO_WARNINGS: '1' } });

        const files = await readdir(tmpDir);
        const htmlFile = files.find(f => f.includes('htmlcut') && f.endsWith('.html'));
        const txtFile = files.find(f => f.includes('htmlcut') && f.endsWith('.txt'));

        const htmlContent = await readFile(join(tmpDir, htmlFile), 'utf8');
        const txtContent = await readFile(join(tmpDir, txtFile), 'utf8');

        server.close();

        const expectedHref = `http://localhost:${port}/docs/about.html`;
        const expectedSrc = `http://localhost:${port}/docs/img.png`;

        assert.ok(htmlContent.includes(`<a href="${expectedHref}">`), `HTML missing expanded link, got: ${htmlContent}`);
        assert.ok(htmlContent.includes(`<img src="${expectedSrc}"`), `HTML missing expanded src, got: ${htmlContent}`);

        assert.ok(txtContent.includes(`About [${expectedHref}]`), `TXT missing expanded link, got: ${txtContent}`);
        assert.ok(txtContent.includes(`[Logo]`), `TXT should have image alt text, got: ${txtContent}`);
    } finally {
        await rm(tmpDir, { recursive: true, force: true });
    }
});

test('CLI: --base string overrides local file context and resolves links absolutely to arbitrary URL', async () => {
    const tmpDir = join(tmpdir(), `htmlcut-test-${Date.now()}`);
    await mkdir(tmpDir, { recursive: true });

    try {
        const body = '<p><a href="/articles/new">New</a></p>';
        const inputPath = join(tmpDir, 'input.html');
        await writeFile(inputPath, body, 'utf8');

        const outputBase = join(tmpDir, 'out');

        await execFileAsync(CLI_PATH, [
            '-i', inputPath, '-b', 'https://override.example.com/', '-s', '<p>', '-e', '</p>', '-o', outputBase
        ], { env: { ...process.env, NODE_NO_WARNINGS: '1' } });

        const files = await readdir(tmpDir);
        const txtFile = files.find(f => f.includes('htmlcut') && f.endsWith('.txt'));
        const htmlFile = files.find(f => f.includes('htmlcut') && f.endsWith('.html'));

        const txtContent = await readFile(join(tmpDir, txtFile), 'utf8');
        const htmlContent = await readFile(join(tmpDir, htmlFile), 'utf8');

        assert.ok(txtContent.includes('New [https://override.example.com/articles/new]'), `TXT output missing resolved link overridden by --base, got: ${txtContent}`);
        assert.ok(htmlContent.includes('<a href="https://override.example.com/articles/new">'), `HTML output missing resolved link overridden by --base, got: ${htmlContent}`);
    } finally {
        await rm(tmpDir, { recursive: true, force: true });
    }
});

test('CLI: --stdout streams text directly to stdout and suppresses file creation', async () => {
    const tmpDir = join(tmpdir(), `htmlcut-test-${Date.now()}`);
    await mkdir(tmpDir, { recursive: true });

    try {
        const body = '<p>Streamed Output</p>';
        const inputPath = join(tmpDir, 'input.html');
        await writeFile(inputPath, body, 'utf8');

        const { stdout } = await execFileAsync(CLI_PATH, [
            '-i', inputPath, '-s', '<p>', '-e', '</p>', '--stdout'
        ], { env: { ...process.env, NODE_NO_WARNINGS: '1' } });

        assert.ok(stdout.includes('Streamed Output'), `Stdout missing payload, got: ${stdout}`);

        // Assert no output files were written to disk (only input.html should exist)
        const files = await readdir(tmpDir);
        const generated = files.filter(f => f !== 'input.html' && (f.endsWith('.txt') || f.endsWith('.html')));
        assert.equal(generated.length, 0, `Expected 0 disk output files when using --stdout, found: ${generated.join(', ')}`);
    } finally {
        await rm(tmpDir, { recursive: true, force: true });
    }
});

test('CLI: --json emits a valid JSON array of {html, text} objects to stdout', async () => {
    const tmpDir = join(tmpdir(), `htmlcut-test-${Date.now()}`);
    await mkdir(tmpDir, { recursive: true });

    try {
        const body = '<article>Item 1</article><article>Item 2</article>';
        const inputPath = join(tmpDir, 'input.html');
        await writeFile(inputPath, body, 'utf8');

        const { stdout } = await execFileAsync(CLI_PATH, [
            '-i', inputPath, '-s', '<article>', '-e', '</article>', '-g', '--json'
        ], { env: { ...process.env, NODE_NO_WARNINGS: '1' } });

        // Extract the JSON part (the diagnostic log line precedes the JSON when not using --quiet)
        const jsonMatch = stdout.match(/\[\s*\{[\s\S]*\}\s*\]/);
        assert.ok(jsonMatch, `Output did not contain a JSON array:\n${stdout}`);

        const data = JSON.parse(jsonMatch[0]);
        assert.equal(data.length, 2);
        assert.equal(data[0].text, 'Item 1');
        assert.equal(data[1].text, 'Item 2');
        // html field contains the extracted fragment (start tag included by extractor)
        assert.ok(data[0].html.includes('Item 1'), `First item html should include text, got: ${data[0].html}`);
    } finally {
        await rm(tmpDir, { recursive: true, force: true });
    }
});

test('CLI: --quiet suppresses diagnostic log so stdout contains only the payload', async () => {
    const tmpDir = join(tmpdir(), `htmlcut-test-${Date.now()}`);
    await mkdir(tmpDir, { recursive: true });

    try {
        const body = '<p>Pristine</p>';
        const inputPath = join(tmpDir, 'input.html');
        await writeFile(inputPath, body, 'utf8');

        const { stdout } = await execFileAsync(CLI_PATH, [
            '-i', inputPath, '-s', '<p>', '-e', '</p>', '--stdout', '-q'
        ], { env: { ...process.env, NODE_NO_WARNINGS: '1' } });

        // Only the payload should appear — no "✓ Successfully extracted…" prefix
        assert.equal(stdout, 'Pristine\n', `Stdout must be strictly the payload when --quiet is active, got:\n${stdout}`);
    } finally {
        await rm(tmpDir, { recursive: true, force: true });
    }
});
