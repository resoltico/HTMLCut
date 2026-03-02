#!/usr/bin/env -S node --no-warnings=ExperimentalWarning

// ---------------------------------------------------------------------------
// Programmatically silence node:sqlite ExperimentalWarning so users who
// invoke the script via `node src/cli.js` (bypassing the shebang flags)
// do not see noisy experimental notices. Must be the very first code run.
// ---------------------------------------------------------------------------
import { installWarningFilter } from './warnings.js';
installWarningFilter();

import { parseArgs } from 'node:util';
import { stat } from 'node:fs/promises';
import { createReadStream } from 'node:fs';
import { extractStream } from './extractor.js';
import { logExtraction, getHistoryGroupedBySuccess } from './storage.js';
import { getOutputBase, writeOutput, toPlainText } from './formatters.js';
import pkg from '../package.json' with { type: 'json' };

const { version } = pkg;

const MAX_FILE_SIZE = 50 * 1024 * 1024; // 50 MB guard for known-size local files
const MAX_FILE_SIZE_MB = MAX_FILE_SIZE / 1024 / 1024;
const FETCH_TIMEOUT_MS = 15000;

const options = {
    input: { type: 'string', short: 'i' },
    start: { type: 'string', short: 's' },
    end: { type: 'string', short: 'e' },
    regex: { type: 'boolean', short: 'r', default: false },
    global: { type: 'boolean', short: 'g', default: false },
    output: { type: 'string', short: 'o', default: 'output' },
    track: { type: 'boolean', short: 't', default: false },
    history: { type: 'boolean', short: 'H', default: false },
    version: { type: 'boolean', short: 'V', default: false },
    help: { type: 'boolean', short: 'h', default: false },
};

// Hoisted so the catch block can (a) log failures with correctly parsed metadata
// rather than falling back to raw process.argv heuristics, and (b) cancel any
// in-flight HTTP response body on error paths.
let inputSource = '';
let startPattern = '';
let endPattern = '';
let shouldTrack = false;
let startTime = 0;
let fetchBody = null;

try {
    const { values, positionals } = parseArgs({ options, allowPositionals: true });

    if (values.help) {
        console.log(`HTMLCut v${version}
  Usage: htmlcut --input <file|url|-> --start <pattern> --end <pattern> [--regex] [--global] [--output <path>]

  Options:
    -i, --input    Source HTML file, URL (e.g. https://example.com), or - for stdin
                   (also accepts as the first positional argument, without -i)
    -s, --start    Start pattern
    -e, --end      End pattern
    -r, --regex    Treat patterns as RegExp 'v' expressions
    -g, --global   Extract all matching occurrences globally
    -o, --output   Output base path (default: "output")
    -t, --track    Log this extraction to local history
    -H, --history  Show your recent extraction history
    -V, --version  Print version number
    -h, --help     Show this help message
`);
        process.exit(0);
    }

    if (values.version) {
        console.log(version);
        process.exit(0);
    }

    if (values.history) {
        const history = getHistoryGroupedBySuccess();
        console.log(JSON.stringify(history, null, 2));
        process.exit(0);
    }

    inputSource = values.input || positionals[0] || '';
    startPattern = values.start || '';
    endPattern = values.end || '';
    shouldTrack = values.track;

    if (!inputSource || !startPattern || !endPattern) {
        throw new Error('Missing required arguments. Use --help for usage.');
    }

    startTime = performance.now();

    let sourceStream;

    if (/^https?:\/\//i.test(inputSource)) {
        const response = await fetch(inputSource, { signal: AbortSignal.timeout(FETCH_TIMEOUT_MS) });
        if (!response.ok) {
            throw new Error(`HTTP ${response.status}: ${response.statusText}`);
        }

        const contentLength = response.headers.get('content-length');
        if (contentLength && Number(contentLength) > MAX_FILE_SIZE) {
            throw new Error(`Payload exceeds ${MAX_FILE_SIZE_MB}MB limit`);
        }

        if (!response.body) {
            throw new Error('Empty response body');
        }

        // Wrap the byte-streaming body in a size guard as an async generator.
        fetchBody = response.body;
        sourceStream = (async function* () {
            let totalRawBytes = 0;
            for await (const chunk of fetchBody) {
                totalRawBytes += chunk.length;
                if (totalRawBytes > MAX_FILE_SIZE) {
                    throw new Error(`Payload exceeds ${MAX_FILE_SIZE_MB}MB limit`);
                }
                yield chunk;
            }
        }());
    } else if (inputSource === '-') {
        // Eagerly buffer stdin before extraction. A lazy generator wrapping
        // process.stdin is abandoned mid-iteration when extraction finishes early
        // (non-global mode), leaving the libuv read handle open and causing the
        // process to hang. Consuming stdin fully up front eliminates that race.
        const stdinChunks = [];
        let stdinBytes = 0;
        for await (const chunk of process.stdin) {
            stdinBytes += chunk.length;
            if (stdinBytes > MAX_FILE_SIZE) {
                throw new Error(`Input exceeds ${MAX_FILE_SIZE_MB}MB limit`);
            }
            stdinChunks.push(chunk);
        }
        sourceStream = stdinChunks;
    } else {
        // For local files with a known size, gate early so we avoid reading at all.
        const stats = await stat(inputSource);
        if (stats.size > MAX_FILE_SIZE) {
            throw new Error(`File exceeds ${MAX_FILE_SIZE_MB}MB limit`);
        }

        // Wrap createReadStream in a size guard (catches special files like /dev/zero
        // where stat() reports 0 bytes but the stream delivers far more).
        const rs = createReadStream(inputSource);
        sourceStream = (async function* () {
            let totalRawBytes = 0;
            for await (const chunk of rs) {
                totalRawBytes += chunk.length;
                if (totalRawBytes > MAX_FILE_SIZE) {
                    throw new Error(`File exceeds ${MAX_FILE_SIZE_MB}MB limit`);
                }
                yield chunk;
            }
        }());
    }

    // Stream the source through the extractor; accumulate extracted fragments for output.
    const htmlFragments = [];
    const txtFragments = [];

    for await (const fragment of extractStream(sourceStream, startPattern, endPattern, { isRegex: values.regex, isGlobal: values.global })) {
        htmlFragments.push(fragment);
        txtFragments.push(toPlainText(fragment));
    }

    // Release any unconsumed URL response body. In non-global mode the extractor
    // returns after the first match, leaving the connection open. cancel() closes it.
    fetchBody?.cancel().catch(() => {});

    const finalHtml = htmlFragments.join('\n');
    const finalTxt = txtFragments.join('\n');

    const outputBase = getOutputBase(values.output);
    const htmlPath = `${outputBase}.html`;
    const txtPath = `${outputBase}.txt`;

    try {
        await Promise.all([
            writeOutput(finalHtml, htmlPath, inputSource),
            writeOutput(finalTxt, txtPath),
        ]);
    } catch (writeError) {
        throw new Error('One or more output files could not be written', { cause: writeError });
    }

    const durationMs = Math.round(performance.now() - startTime);

    if (shouldTrack) {
        try {
            logExtraction({
                source: inputSource,
                startPattern,
                endPattern,
                success: true,
                durationMs,
            });
        } catch {
            // Silently ignore history logging failures — the extraction succeeded
        }
    }

    const count = htmlFragments.length;
    console.log(`✓ Successfully extracted ${count} ${count === 1 ? 'fragment' : 'fragments'} in ${durationMs}ms`);
    console.log(`  → ${htmlPath}`);
    console.log(`  → ${txtPath}`);

} catch (error) {
    // Release any unconsumed URL response body (e.g. pattern not found, timeout).
    fetchBody?.cancel().catch(() => {});

    const causeMsg = error.cause instanceof Error ? `: ${error.cause.message}` : '';
    console.error(`✗ Error: ${error.message}${causeMsg}`);

    if (shouldTrack) {
        try {
            logExtraction({
                source: inputSource,
                startPattern,
                endPattern,
                success: false,
                durationMs: startTime > 0 ? Math.round(performance.now() - startTime) : 0,
            });
        } catch {
            // Silently ignore logging failures during fatal exceptions
        }
    }

    process.exit(1);
}
