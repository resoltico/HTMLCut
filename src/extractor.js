/**
 * @fileoverview Streaming extraction engine: scans an AsyncIterable of byte/string
 * chunks for literal or RegExp 'v' start/end delimiters, yielding matched fragments
 * without buffering the entire payload in memory.
 */

const DEFAULT_STREAM_TIMEOUT_MS = 15000;

/** Maximum sliding-window character count before the regex-path trims it. */
const STREAM_WINDOW_MAX = 32768;
/** Number of characters to retain after trimming the regex-path window. */
const STREAM_WINDOW_KEEP = 16384;

/**
 * @typedef {Object} ExtractOptions
 * @property {boolean} [isRegex=false] Parse patterns as RegExp 'v' expressions.
 * @property {boolean} [isGlobal=false] Extract all matches instead of just the first.
 * @property {number} [timeoutMs] Per-chunk processing timeout in ms (default 15 s).
 */

/**
 * Extracts content from an AsyncIterable stream without buffering the entire
 * payload into memory. Yields one string fragment per match.
 *
 * Uses the Web Encoding API (TextDecoder) with streaming enabled to robustly
 * decode chunks (Buffer, Uint8Array) even if multi-byte UTF-8 sequences are
 * split across chunk boundaries.
 *
 * Note: zero-width regex patterns (e.g. \b) in global mode may produce fewer
 * matches than a full-string search because the sliding window must advance
 * at least one character past each zero-width match boundary.
 *
 * @param {Iterable<string|Buffer|Uint8Array>|AsyncIterable<string|Buffer|Uint8Array>} iterable The source stream.
 * @param {string} startPattern The starting literal or RegExp pattern.
 * @param {string} endPattern The terminating literal or RegExp pattern.
 * @param {ExtractOptions} [opts]
 * @returns {AsyncGenerator<string>} An async generator yielding extracted string fragments.
 */
export async function* extractStream(iterable, startPattern, endPattern, {
    isRegex = false,
    isGlobal = false,
    timeoutMs = DEFAULT_STREAM_TIMEOUT_MS,
} = {}) {
    let buf = '';
    let contentStart = -1;
    let matched = false;

    let startRegex = null;
    let endRegex = null;
    if (isRegex) {
        try {
            startRegex = new RegExp(startPattern, 'gv');
            endRegex = new RegExp(endPattern, 'gv');
        } catch (err) {
            const message = err instanceof Error ? err.message : String(err);
            throw new Error(`Invalid RegExp: ${message}`, { cause: err });
        }
    }

    // Inner generator: decodes byte/string chunks to text strings and flushes the
    // TextDecoder after the final chunk so incomplete multi-byte sequences at the
    // end of the stream are resolved. Yielding the flush as a regular chunk means
    // it passes through the same matching loop as every other chunk — ensuring no
    // match is missed due to bytes held back across the stream boundary.
    const decoder = new TextDecoder('utf-8');
    async function* decodeChunks() {
        for await (const chunk of iterable) {
            yield typeof chunk === 'string' ? chunk : decoder.decode(chunk, { stream: true });
        }
        const tail = decoder.decode();
        if (tail) { yield tail; }
    }

    for await (const textChunk of decodeChunks()) {
        const chunkStart = performance.now();
        buf += textChunk;

        while (true) {
            if (performance.now() - chunkStart > timeoutMs) {
                throw new Error('Extraction timeout: pattern too complex or input too large');
            }

            if (contentStart === -1) {
                if (isRegex) {
                    startRegex.lastIndex = 0;
                    const startMatch = startRegex.exec(buf);
                    if (!startMatch) {
                        // Trim oversized window to cap memory; keep a trailing overlap so
                        // a start pattern split across chunk boundaries is not lost.
                        if (buf.length > STREAM_WINDOW_MAX) {
                            buf = buf.slice(-STREAM_WINDOW_KEEP);
                        }
                        break;
                    }
                    contentStart = startMatch.index + startMatch[0].length;
                } else {
                    const startIndex = buf.indexOf(startPattern);
                    if (startIndex === -1) {
                        // Keep a trailing overlap of (pattern.length - 1) chars so a
                        // pattern split across chunks is not lost.
                        // Guard: slice(-0) === slice(0) returns the full string in JS,
                        // so a 1-char pattern requires an explicit empty-string assignment.
                        const overlap = startPattern.length - 1;
                        if (buf.length > startPattern.length) {
                            buf = overlap > 0 ? buf.slice(-overlap) : '';
                        }
                        break;
                    }
                    contentStart = startIndex + startPattern.length;
                }
            }

            if (isRegex) {
                endRegex.lastIndex = contentStart;
                const endMatch = endRegex.exec(buf);
                if (!endMatch) {
                    break;
                }
                yield buf.slice(contentStart, endMatch.index);
                const endTail = endMatch.index + endMatch[0].length;
                // Guard: advance at least 1 char for zero-width end matches to prevent
                // an infinite loop where the window never shrinks.
                buf = buf.slice(endTail || 1);
                contentStart = -1;
                if (!isGlobal) { return; }
                matched = true;
            } else {
                const endIndex = buf.indexOf(endPattern, contentStart);
                if (endIndex === -1) {
                    break;
                }
                yield buf.slice(contentStart, endIndex);
                buf = buf.slice(endIndex + endPattern.length);
                contentStart = -1;
                if (!isGlobal) { return; }
                matched = true;
            }
        }
    }

    if (!matched) {
        if (contentStart === -1) {
            throw new Error(`Start pattern not found: ${startPattern}`);
        } else {
            throw new Error(`End pattern not found: ${endPattern}`);
        }
    }
}
