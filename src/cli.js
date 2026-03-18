#!/usr/bin/env node

import { parseArgs } from 'node:util';
import pkg from '../package.json' with { type: 'json' };
import { ensureHtmlCutError, usageError } from './errors.js';
import { extractMatches, DEFAULT_REGEX_FLAGS } from './extractor.js';
import { renderHelp } from './help.js';
import { renderHtmlAsText, rewriteHtmlUrls } from './html.js';
import { buildReport, formatPayload, getBundlePaths, writeBundle } from './output.js';
import { DEFAULT_FETCH_TIMEOUT_MS, DEFAULT_MAX_BYTES, parseByteSize, readSource } from './source.js';

const { version } = pkg;

const options = {
    from: { type: 'string', short: 'f' },
    to: { type: 'string', short: 't' },
    pattern: { type: 'string', short: 'p', default: 'literal' },
    flags: { type: 'string' },
    all: { type: 'boolean', short: 'a', default: false },
    capture: { type: 'string', short: 'c', default: 'inner' },
    format: { type: 'string', short: 'F', default: 'text' },
    bundle: { type: 'string', short: 'o' },
    'base-url': { type: 'string', short: 'b' },
    'max-bytes': { type: 'string', default: String(DEFAULT_MAX_BYTES) },
    'fetch-timeout-ms': { type: 'string', default: String(DEFAULT_FETCH_TIMEOUT_MS) },
    verbose: { type: 'boolean', short: 'v', default: false },
    version: { type: 'boolean', short: 'V', default: false },
    help: { type: 'boolean', short: 'h', default: false },
};

function requireChoice(value, allowedValues, label) {
    if (!allowedValues.includes(value)) {
        throw usageError(`${label} must be one of: ${allowedValues.join(', ')}`);
    }
    return value;
}

function parsePositiveInteger(value, label) {
    const parsed = Number(value);
    if (!Number.isInteger(parsed) || parsed <= 0) {
        throw usageError(`${label} must be a positive integer`);
    }
    return parsed;
}

function validateBaseUrl(value) {
    if (!value) {
        return null;
    }

    try {
        const parsed = new URL(value);
        if (!/^https?:$/i.test(parsed.protocol)) {
            throw usageError('--base-url must use http or https');
        }
        return parsed.href;
    } catch (error) {
        if (error instanceof Error && error.name === 'TypeError') {
            throw usageError(`Invalid --base-url: ${value}`, error);
        }
        throw error;
    }
}

try {
    let values;
    let positionals;

    try {
        ({ values, positionals } = parseArgs({
            options,
            allowPositionals: true,
            strict: true,
        }));
    } catch (error) {
        throw usageError(error instanceof Error ? error.message : String(error), error);
    }

    if (values.help) {
        process.stdout.write(renderHelp(version));
        process.exit(0);
    }

    if (values.version) {
        process.stdout.write(`${version}\n`);
        process.exit(0);
    }

    if (positionals.length === 0) {
        throw usageError('Missing input. Pass a URL, file path, or - for stdin.');
    }

    if (positionals.length > 1) {
        throw usageError(`Unexpected extra arguments: ${positionals.slice(1).join(' ')}`);
    }

    if (!values.from || !values.to) {
        throw usageError('Both --from and --to are required.');
    }

    const input = positionals[0];
    const mode = requireChoice(values.pattern, ['literal', 'regex'], '--pattern');
    const capture = requireChoice(values.capture, ['inner', 'outer'], '--capture');
    const format = requireChoice(values.format, ['text', 'html', 'json', 'none'], '--format');
    const baseUrl = validateBaseUrl(values['base-url']);
    const maxBytes = parseByteSize(values['max-bytes']);
    const fetchTimeoutMs = parsePositiveInteger(values['fetch-timeout-ms'], '--fetch-timeout-ms');

    if (mode !== 'regex' && values.flags) {
        throw usageError('--flags can only be used with --pattern regex');
    }

    const startedAt = performance.now();
    const source = await readSource(input, { maxBytes, fetchTimeoutMs });
    const rewriteBaseUrl = baseUrl || source.baseUrl;
    const extracted = extractMatches(source.text, {
        from: values.from,
        to: values.to,
        mode,
        flags: values.flags || DEFAULT_REGEX_FLAGS,
        all: values.all,
        capture,
    });

    const matches = extracted.map(match => {
        const html = rewriteHtmlUrls(match.html, rewriteBaseUrl);
        return {
            index: match.index,
            range: match.range,
            innerRange: match.innerRange,
            outerRange: match.outerRange,
            html,
            text: renderHtmlAsText(html),
        };
    });

    const bundle = values.bundle ? getBundlePaths(values.bundle) : null;
    const durationMs = Math.round(performance.now() - startedAt);
    const report = buildReport({
        version,
        source,
        baseUrl: rewriteBaseUrl,
        pattern: {
            from: values.from,
            to: values.to,
            mode,
            flags: values.flags || DEFAULT_REGEX_FLAGS,
            capture,
            all: values.all,
        },
        durationMs,
        matches,
        bundle,
    });

    if (report.bundle) {
        await writeBundle(report);
    }

    if (format !== 'none') {
        process.stdout.write(`${formatPayload(report, format)}\n`);
    }

    if (values.verbose) {
        const fragmentsLabel = matches.length === 1 ? 'fragment' : 'fragments';
        process.stderr.write(`htmlcut: matched ${matches.length} ${fragmentsLabel} in ${durationMs}ms\n`);
        if (report.bundle) {
            process.stderr.write(`htmlcut: wrote bundle to ${report.bundle.dir}\n`);
        }
    }
} catch (error) {
    const htmlCutError = ensureHtmlCutError(error);
    const causeMessage = htmlCutError.exitCode !== 2
        && htmlCutError.cause instanceof Error
        && htmlCutError.cause.message !== htmlCutError.message
        ? `\n${htmlCutError.cause.message}`
        : '';
    process.stderr.write(`htmlcut: ${htmlCutError.message}${causeMessage}\n`);
    process.exit(htmlCutError.exitCode);
}
