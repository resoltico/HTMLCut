import { Buffer } from 'node:buffer';
import { createReadStream } from 'node:fs';
import { stat } from 'node:fs/promises';
import { resolve } from 'node:path';
import { EXIT_CODES, HtmlCutError, usageError } from './errors.js';

export const DEFAULT_MAX_BYTES = 50 * 1024 * 1024;
export const DEFAULT_FETCH_TIMEOUT_MS = 15000;
const KIBIBYTE = 1024;
const MEBIBYTE = KIBIBYTE * KIBIBYTE;
const GIBIBYTE = MEBIBYTE * KIBIBYTE;

const BYTE_UNITS = new Map([
    ['b', 1],
    ['kb', KIBIBYTE],
    ['mb', MEBIBYTE],
    ['gb', GIBIBYTE],
]);

function countChunkBytes(chunk) {
    return typeof chunk === 'string' ? Buffer.byteLength(chunk) : chunk.byteLength;
}

async function readTextFromIterable(iterable, { maxBytes, overflowMessage }) {
    const decoder = new TextDecoder();
    const parts = [];
    let bytesRead = 0;

    for await (const chunk of iterable) {
        bytesRead += countChunkBytes(chunk);
        if (bytesRead > maxBytes) {
            throw new HtmlCutError(overflowMessage, {
                exitCode: EXIT_CODES.SOURCE,
                code: 'SOURCE_TOO_LARGE',
            });
        }

        if (typeof chunk === 'string') {
            parts.push(chunk);
        } else {
            parts.push(decoder.decode(chunk, { stream: true }));
        }
    }

    const tail = decoder.decode();
    if (tail) {
        parts.push(tail);
    }

    return {
        text: parts.join(''),
        bytesRead,
    };
}

export function parseByteSize(value) {
    if (typeof value === 'number' && Number.isFinite(value) && value > 0) {
        return Math.floor(value);
    }

    const match = /^(\d+(?:\.\d+)?)\s*(b|kb|mb|gb)?$/i.exec(String(value).trim());
    if (!match) {
        throw usageError(`Invalid byte size: ${value}`);
    }

    const [, amountText, unitText = 'b'] = match;
    const amount = Number(amountText);
    const unit = BYTE_UNITS.get(unitText.toLowerCase());
    const bytes = amount * unit;

    if (!Number.isFinite(bytes) || bytes <= 0) {
        throw usageError(`Invalid byte size: ${value}`);
    }

    return Math.floor(bytes);
}

export function formatByteSize(bytes) {
    if (bytes % GIBIBYTE === 0) {
        return `${bytes / GIBIBYTE} GB`;
    }

    if (bytes % MEBIBYTE === 0) {
        return `${bytes / MEBIBYTE} MB`;
    }

    if (bytes % KIBIBYTE === 0) {
        return `${bytes / KIBIBYTE} KB`;
    }

    return `${bytes} bytes`;
}

export async function readSource(input, {
    maxBytes = DEFAULT_MAX_BYTES,
    fetchTimeoutMs = DEFAULT_FETCH_TIMEOUT_MS,
} = {}) {
    const limitLabel = `${formatByteSize(maxBytes)} limit`;

    if (/^https?:\/\//i.test(input)) {
        let response;
        try {
            response = await fetch(input, { signal: AbortSignal.timeout(fetchTimeoutMs) });
        } catch (error) {
            throw new HtmlCutError(`Could not fetch ${input}`, {
                exitCode: EXIT_CODES.SOURCE,
                code: 'FETCH_FAILED',
                cause: error,
            });
        }

        if (!response.ok) {
            throw new HtmlCutError(`HTTP ${response.status} ${response.statusText}`.trim(), {
                exitCode: EXIT_CODES.SOURCE,
                code: 'HTTP_ERROR',
            });
        }

        const contentLengthHeader = response.headers.get('content-length');
        const contentLength = contentLengthHeader === null ? Number.NaN : Number(contentLengthHeader);
        if (Number.isFinite(contentLength) && contentLength > maxBytes) {
            throw new HtmlCutError(`Response exceeds ${limitLabel}`, {
                exitCode: EXIT_CODES.SOURCE,
                code: 'SOURCE_TOO_LARGE',
            });
        }

        if (!response.body) {
            throw new HtmlCutError('Response body was empty', {
                exitCode: EXIT_CODES.SOURCE,
                code: 'EMPTY_RESPONSE',
            });
        }

        try {
            const { text, bytesRead } = await readTextFromIterable(response.body, {
                maxBytes,
                overflowMessage: `Response exceeds ${limitLabel}`,
            });

            return {
                kind: 'url',
                input,
                text,
                bytesRead,
                baseUrl: input,
            };
        } catch (error) {
            if (error instanceof HtmlCutError) {
                throw error;
            }

            throw new HtmlCutError(`Could not read response body from ${input}`, {
                exitCode: EXIT_CODES.SOURCE,
                code: 'FETCH_READ_FAILED',
                cause: error,
            });
        }
    }

    if (input === '-') {
        try {
            const { text, bytesRead } = await readTextFromIterable(process.stdin, {
                maxBytes,
                overflowMessage: `Stdin exceeds ${limitLabel}`,
            });

            return {
                kind: 'stdin',
                input,
                text,
                bytesRead,
                baseUrl: null,
            };
        } catch (error) {
            if (error instanceof HtmlCutError) {
                throw error;
            }

            throw new HtmlCutError('Could not read stdin', {
                exitCode: EXIT_CODES.SOURCE,
                code: 'STDIN_READ_FAILED',
                cause: error,
            });
        }
    }

    const resolvedInput = resolve(input);

    try {
        const info = await stat(resolvedInput);
        if (info.size > maxBytes) {
            throw new HtmlCutError(`File exceeds ${limitLabel}`, {
                exitCode: EXIT_CODES.SOURCE,
                code: 'SOURCE_TOO_LARGE',
            });
        }
    } catch (error) {
        if (error instanceof HtmlCutError) {
            throw error;
        }

        throw new HtmlCutError(`Could not access file ${resolvedInput}`, {
            exitCode: EXIT_CODES.SOURCE,
            code: 'FILE_ACCESS_FAILED',
            cause: error,
        });
    }

    try {
        const { text, bytesRead } = await readTextFromIterable(createReadStream(resolvedInput), {
            maxBytes,
            overflowMessage: `File exceeds ${limitLabel}`,
        });

        return {
            kind: 'file',
            input: resolvedInput,
            text,
            bytesRead,
            baseUrl: null,
        };
    } catch (error) {
        if (error instanceof HtmlCutError) {
            throw error;
        }

        throw new HtmlCutError(`Could not read file ${resolvedInput}`, {
            exitCode: EXIT_CODES.SOURCE,
            code: 'FILE_READ_FAILED',
            cause: error,
        });
    }
}
