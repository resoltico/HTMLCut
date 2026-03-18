import { mkdir, writeFile } from 'node:fs/promises';
import { resolve } from 'node:path';
import { EXIT_CODES, HtmlCutError } from './errors.js';
import { deriveDocumentTitle, wrapHtmlDocument } from './html.js';

function renderTextPayload(matches) {
    return matches.map(match => match.text).join('\n\n');
}

function renderHtmlPayload(matches) {
    return matches.map(match => match.html).join('\n\n');
}

function renderBundleText(matches) {
    if (matches.length === 1) {
        return matches[0].text;
    }

    return matches.map(match => {
        const heading = `Match ${match.index}`;
        return `${heading}\n${'='.repeat(heading.length)}\n${match.text}`;
    }).join('\n\n');
}

export function getBundlePaths(bundleDir) {
    const dir = resolve(bundleDir);
    return {
        dir,
        html: resolve(dir, 'selection.html'),
        text: resolve(dir, 'selection.txt'),
        report: resolve(dir, 'report.json'),
    };
}

export function buildReport({
    version,
    source,
    baseUrl,
    pattern,
    durationMs,
    matches,
    bundle = null,
}) {
    const documentTitle = deriveDocumentTitle({
        matches,
        input: source.input,
        baseUrl,
    });

    return {
        tool: 'htmlcut',
        version,
        input: {
            kind: source.kind,
            value: source.input,
        },
        baseUrl,
        documentTitle,
        pattern: {
            from: pattern.from,
            to: pattern.to,
            mode: pattern.mode,
            flags: pattern.mode === 'regex' ? pattern.flags : null,
            capture: pattern.capture,
            all: pattern.all,
        },
        stats: {
            bytesRead: source.bytesRead,
            durationMs,
            matchCount: matches.length,
        },
        matches,
        bundle,
    };
}

export function formatPayload(report, format) {
    switch (format) {
    case 'text':
        return renderTextPayload(report.matches);

    case 'html':
        return renderHtmlPayload(report.matches);

    case 'json':
        return JSON.stringify(report, null, 2);

    case 'none':
        return '';

    default:
        throw new HtmlCutError(`Unknown output format: ${format}`, {
            exitCode: EXIT_CODES.USAGE,
            code: 'UNKNOWN_FORMAT',
        });
    }
}

export async function writeBundle(report) {
    if (!report.bundle) {
        return;
    }

    const title = report.documentTitle || deriveDocumentTitle({
        matches: report.matches,
        input: report.input.value,
        baseUrl: report.baseUrl,
    });

    try {
        await mkdir(report.bundle.dir, { recursive: true });
        await Promise.all([
            writeFile(report.bundle.html, wrapHtmlDocument({ matches: report.matches, title }), 'utf8'),
            writeFile(report.bundle.text, renderBundleText(report.matches), 'utf8'),
            writeFile(report.bundle.report, `${JSON.stringify(report, null, 2)}\n`, 'utf8'),
        ]);
    } catch (error) {
        throw new HtmlCutError(`Could not write bundle to ${report.bundle.dir}`, {
            exitCode: EXIT_CODES.OUTPUT,
            code: 'BUNDLE_WRITE_FAILED',
            cause: error,
        });
    }
}
