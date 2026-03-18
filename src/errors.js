const INTERNAL_EXIT_CODE = 1;
const USAGE_EXIT_CODE = 2;
const SOURCE_EXIT_CODE = 3;
const EXTRACTION_EXIT_CODE = 4;
const OUTPUT_EXIT_CODE = 5;

export const EXIT_CODES = Object.freeze({
    INTERNAL: INTERNAL_EXIT_CODE,
    USAGE: USAGE_EXIT_CODE,
    SOURCE: SOURCE_EXIT_CODE,
    EXTRACTION: EXTRACTION_EXIT_CODE,
    OUTPUT: OUTPUT_EXIT_CODE,
});

export class HtmlCutError extends Error {
    constructor(message, {
        exitCode = EXIT_CODES.INTERNAL,
        code = 'INTERNAL',
        cause,
    } = {}) {
        super(message, { cause });
        this.name = 'HtmlCutError';
        this.exitCode = exitCode;
        this.code = code;
    }
}

export function usageError(message, cause) {
    return new HtmlCutError(message, {
        exitCode: EXIT_CODES.USAGE,
        code: 'USAGE',
        cause,
    });
}

export function ensureHtmlCutError(error) {
    if (error instanceof HtmlCutError) {
        return error;
    }

    const message = error instanceof Error ? error.message : String(error);
    return new HtmlCutError(message, { cause: error });
}
