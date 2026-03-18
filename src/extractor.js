import { EXIT_CODES, HtmlCutError, usageError } from './errors.js';

export const DEFAULT_REGEX_FLAGS = 'u';

function createLiteralFinder(pattern) {
    return (sourceText, fromIndex) => {
        const index = sourceText.indexOf(pattern, fromIndex);
        if (index === -1) {
            return null;
        }

        return {
            index,
            end: index + pattern.length,
        };
    };
}

function normalizeRegexFlags(flags = DEFAULT_REGEX_FLAGS) {
    const uniqueFlags = [...new Set(flags.replaceAll('g', ''))];
    return `${uniqueFlags.join('')}g`;
}

function createRegexFinder(pattern, flags) {
    let regex;
    try {
        regex = new RegExp(pattern, normalizeRegexFlags(flags));
    } catch (error) {
        throw usageError(`Invalid regular expression: ${error instanceof Error ? error.message : String(error)}`, error);
    }

    return (sourceText, fromIndex) => {
        regex.lastIndex = fromIndex;
        const match = regex.exec(sourceText);
        if (!match) {
            return null;
        }

        return {
            index: match.index,
            end: match.index + match[0].length,
        };
    };
}

function createFinder(pattern, { mode, flags }) {
    if (!pattern) {
        throw usageError('Patterns must not be empty');
    }

    if (mode === 'literal') {
        return createLiteralFinder(pattern);
    }

    if (mode === 'regex') {
        return createRegexFinder(pattern, flags);
    }

    throw usageError(`Unknown pattern mode: ${mode}`);
}

export function extractMatches(sourceText, {
    from,
    to,
    mode = 'literal',
    flags = DEFAULT_REGEX_FLAGS,
    all = false,
    capture = 'inner',
} = {}) {
    if (capture !== 'inner' && capture !== 'outer') {
        throw usageError(`Unknown capture mode: ${capture}`);
    }

    const findStart = createFinder(from, { mode, flags });
    const findEnd = createFinder(to, { mode, flags });
    const matches = [];
    let cursor = 0;

    while (cursor <= sourceText.length) {
        const startMatch = findStart(sourceText, cursor);
        if (!startMatch) {
            break;
        }

        const endMatch = findEnd(sourceText, startMatch.end);
        if (!endMatch) {
            throw new HtmlCutError(`End pattern was not found after offset ${startMatch.index}: ${to}`, {
                exitCode: EXIT_CODES.EXTRACTION,
                code: 'END_NOT_FOUND',
            });
        }

        const outerRange = {
            start: startMatch.index,
            end: endMatch.end,
        };
        const innerRange = {
            start: startMatch.end,
            end: endMatch.index,
        };
        const range = capture === 'outer' ? outerRange : innerRange;

        matches.push({
            index: matches.length + 1,
            html: sourceText.slice(range.start, range.end),
            range,
            innerRange,
            outerRange,
        });

        if (!all) {
            return matches;
        }

        cursor = outerRange.end > outerRange.start ? outerRange.end : outerRange.start + 1;
    }

    if (matches.length === 0) {
        throw new HtmlCutError(`Start pattern was not found: ${from}`, {
            exitCode: EXIT_CODES.EXTRACTION,
            code: 'START_NOT_FOUND',
        });
    }

    return matches;
}
