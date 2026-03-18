import test from 'node:test';
import assert from 'node:assert/strict';
import { access, readFile, writeFile } from 'node:fs/promises';
import { join } from 'node:path';
import { runCli, runCliWithStdin, withServer, withTempDir } from './helpers.js';

test('CLI: --help prints the v2 contract', async () => {
    const { stdout, stderr } = await runCli(['--help']);
    assert.match(stdout, /htmlcut <input> --from <pattern> --to <pattern>/);
    assert.match(stdout, /--bundle <dir>/);
    assert.equal(stderr, '');
});

test('CLI: --version prints the package version', async () => {
    const { stdout, stderr } = await runCli(['--version']);
    assert.equal(stdout, '2.0.0\n');
    assert.equal(stderr, '');
});

test('CLI: missing input fails with usage exit code', async () => {
    await assert.rejects(
        () => runCli([]),
        error => {
            assert.equal(error.code, 2);
            assert.match(error.stderr, /Missing input/);
            return true;
        }
    );
});

test('CLI: invalid options fail as usage errors instead of internal errors', async () => {
    await assert.rejects(
        () => runCli(['--wat']),
        error => {
            assert.equal(error.code, 2);
            assert.equal(
                error.stderr,
                "htmlcut: Unknown option '--wat'. To specify a positional argument starting with a '-', place it at the end of the command after '--', as in '-- \"--wat\"\n"
            );
            return true;
        }
    );
});

test('CLI: default output is plain text to stdout with no success chatter', async () => {
    await withTempDir(async dir => {
        const inputPath = join(dir, 'input.html');
        await writeFile(inputPath, '<article><p>Hello <strong>world</strong></p></article>', 'utf8');

        const { stdout, stderr } = await runCli([
            inputPath,
            '--from', '<article>',
            '--to', '</article>',
        ]);

        assert.equal(stdout, 'Hello world\n');
        assert.equal(stderr, '');
    });
});

test('CLI: extra positionals and invalid flag combinations are rejected cleanly', async () => {
    await withTempDir(async dir => {
        const inputPath = join(dir, 'input.html');
        await writeFile(inputPath, '<p>Hello</p>', 'utf8');

        await assert.rejects(
            () => runCli([
                inputPath,
                'extra',
                '--from', '<p>',
                '--to', '</p>',
            ]),
            error => {
                assert.equal(error.code, 2);
                assert.match(error.stderr, /Unexpected extra arguments/);
                return true;
            }
        );

        await assert.rejects(
            () => runCli([
                inputPath,
                '--from', '<p>',
                '--to', '</p>',
                '--flags', 'i',
            ]),
            error => {
                assert.equal(error.code, 2);
                assert.match(error.stderr, /--flags can only be used with --pattern regex/);
                return true;
            }
        );
    });
});

test('CLI: regex mode supports custom flags', async () => {
    await withTempDir(async dir => {
        const inputPath = join(dir, 'input.html');
        await writeFile(inputPath, '<DIV>Caps Win</DIV>', 'utf8');

        const { stdout } = await runCli([
            inputPath,
            '--from', '<div>',
            '--to', '</div>',
            '--pattern', 'regex',
            '--flags', 'i',
        ]);

        assert.equal(stdout, 'Caps Win\n');
    });
});

test('CLI: invalid base URLs and invalid timeout values are rejected', async () => {
    await withTempDir(async dir => {
        const inputPath = join(dir, 'input.html');
        await writeFile(inputPath, '<p>Hello</p>', 'utf8');

        await assert.rejects(
            () => runCli([
                inputPath,
                '--from', '<p>',
                '--to', '</p>',
                '--base-url', 'not a url',
            ]),
            error => {
                assert.equal(error.code, 2);
                assert.equal(error.stderr, 'htmlcut: Invalid --base-url: not a url\n');
                return true;
            }
        );

        await assert.rejects(
            () => runCli([
                inputPath,
                '--from', '<p>',
                '--to', '</p>',
                '--base-url', 'ftp://example.com',
            ]),
            error => {
                assert.equal(error.code, 2);
                assert.equal(error.stderr, 'htmlcut: --base-url must use http or https\n');
                return true;
            }
        );

        await assert.rejects(
            () => runCli([
                inputPath,
                '--from', '<p>',
                '--to', '</p>',
                '--fetch-timeout-ms', '0',
            ]),
            error => {
                assert.equal(error.code, 2);
                assert.match(error.stderr, /must be a positive integer/);
                return true;
            }
        );
    });
});

test('CLI: --capture outer returns delimiters when html format is requested', async () => {
    await withTempDir(async dir => {
        const inputPath = join(dir, 'input.html');
        await writeFile(inputPath, '<main><section>Keep me</section></main>', 'utf8');

        const { stdout } = await runCli([
            inputPath,
            '--from', '<section>',
            '--to', '</section>',
            '--capture', 'outer',
            '--format', 'html',
        ]);

        assert.equal(stdout, '<section>Keep me</section>\n');
    });
});

test('CLI: stdin + --base-url rewrites links and emits structured JSON', async () => {
    const result = await runCliWithStdin([
        '-',
        '--from', '<article>',
        '--to', '</article>',
        '--base-url', 'https://example.com/docs/start.html',
        '--format', 'json',
    ], '<article><a href="../guide.html">Guide</a></article>');

    assert.equal(result.code, 0);
    assert.equal(result.stderr, '');

    const payload = JSON.parse(result.stdout);
    assert.equal(payload.input.kind, 'stdin');
    assert.equal(payload.baseUrl, 'https://example.com/docs/start.html');
    assert.equal(payload.documentTitle, 'example.com');
    assert.equal(payload.matches[0].html, '<a href="https://example.com/guide.html">Guide</a>');
    assert.equal(payload.matches[0].text, 'Guide [https://example.com/guide.html]');
});

test('CLI: --bundle writes deterministic bundle files and reports their paths', async () => {
    await withTempDir(async dir => {
        const inputPath = join(dir, 'input.html');
        const bundleDir = join(dir, 'bundle');
        await writeFile(inputPath, '<p>One</p><p>Two</p>', 'utf8');

        const { stdout, stderr } = await runCli([
            inputPath,
            '--from', '<p>',
            '--to', '</p>',
            '--all',
            '--format', 'json',
            '--bundle', bundleDir,
            '--verbose',
        ]);

        const payload = JSON.parse(stdout);
        assert.equal(payload.stats.matchCount, 2);
        assert.equal(payload.bundle.dir, bundleDir);
        assert.equal(payload.documentTitle, 'input');
        assert.match(stderr, /matched 2 fragments/);
        assert.match(stderr, /wrote bundle/);

        await access(payload.bundle.html);
        await access(payload.bundle.text);
        await access(payload.bundle.report);

        const html = await readFile(payload.bundle.html, 'utf8');
        const text = await readFile(payload.bundle.text, 'utf8');
        const report = JSON.parse(await readFile(payload.bundle.report, 'utf8'));

        assert.match(html, /Match 1/);
        assert.match(html, /<section data-match-index="2">/);
        assert.match(text, /Match 2/);
        assert.equal(report.bundle.report, payload.bundle.report);
    });
});

test('CLI: --format none suppresses stdout while still writing the bundle', async () => {
    await withTempDir(async dir => {
        const inputPath = join(dir, 'input.html');
        const bundleDir = join(dir, 'bundle');
        await writeFile(inputPath, '<p>One</p>', 'utf8');

        const result = await runCli([
            inputPath,
            '--from', '<p>',
            '--to', '</p>',
            '--format', 'none',
            '--bundle', bundleDir,
            '--verbose',
        ]);

        assert.equal(result.stdout, '');
        assert.match(result.stderr, /matched 1 fragment/);
        assert.match(result.stderr, /wrote bundle/);
        await access(join(bundleDir, 'selection.txt'));
        await access(join(bundleDir, 'selection.html'));
        await access(join(bundleDir, 'report.json'));
    });
});

test('CLI: incomplete trailing match in --all mode is a hard extraction failure', async () => {
    await withTempDir(async dir => {
        const inputPath = join(dir, 'broken.html');
        await writeFile(inputPath, '<p>One</p><p>Two', 'utf8');

        await assert.rejects(
            () => runCli([
                inputPath,
                '--from', '<p>',
                '--to', '</p>',
                '--all',
            ]),
            error => {
                assert.equal(error.code, 4);
                assert.match(error.stderr, /End pattern was not found/);
                return true;
            }
        );
    });
});

test('CLI: URL content-length larger than --max-bytes fails before reading the body', async () => {
    await withServer((req, res) => {
        res.writeHead(200, {
            'content-length': String(4096),
            'content-type': 'text/html',
        });
        res.end('<p>ignored</p>');
    }, async ({ url }) => {
        await assert.rejects(
            () => runCli([
                url,
                '--from', '<p>',
                '--to', '</p>',
                '--max-bytes', '1kb',
            ]),
            error => {
                assert.equal(error.code, 3);
                assert.match(error.stderr, /Response exceeds 1 KB limit/);
                return true;
            }
        );
    });
});
