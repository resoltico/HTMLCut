/**
 * @fileoverview SQLite ExperimentalWarning suppression helper.
 * Extracted into its own module so it can be unit-tested without importing
 * the full CLI module (which triggers parseArgs at load time).
 */

/**
 * Returns true if the given process 'warning' event is a SQLite
 * ExperimentalWarning that should be silenced.
 * @param {string | symbol} name  The event name passed to process.emit.
 * @param {unknown} data The event data (usually an Error-like object).
 * @returns {boolean}
 */
export function isSuppressedWarning(name, data) {
    if (name !== 'warning' || typeof data !== 'object' || data === null) { return false; }
    const d = /** @type {any} */ (data);
    return d.name === 'ExperimentalWarning' && typeof d.message === 'string' && d.message.includes('SQLite');
}

/**
 * Patches process.emit once to suppress SQLite ExperimentalWarning events.
 * Idempotent: a second call is a no-op because installed guards re-entry.
 */
let installed = false;
export function installWarningFilter() {
    if (installed) { return; }
    installed = true;
    const orig = process.emit.bind(process);
    process.emit = function (name, data, ...rest) {
        if (isSuppressedWarning(name, data)) { return false; }
        return orig(name, data, ...rest);
    };
}
