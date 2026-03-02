/**
 * @fileoverview Unit tests for src/warnings.js:
 *   - isSuppressedWarning predicate (all branches)
 *   - installWarningFilter() — patches process.emit and is idempotent
 * Tests run in-process so the coverage tool can instrument every branch.
 */
import test from 'node:test';
import assert from 'node:assert/strict';
import { isSuppressedWarning, installWarningFilter } from '../src/warnings.js';

// ── isSuppressedWarning: TRUE path ────────────────────────────────────────

test('isSuppressedWarning: returns true for SQLite ExperimentalWarning', () => {
    const w = { name: 'ExperimentalWarning', message: 'SQLite is an experimental feature' };
    assert.equal(isSuppressedWarning('warning', w), true);
});

// ── isSuppressedWarning: each FALSE branch in isolation ───────────────────

test('isSuppressedWarning: returns false for non-warning event name', () => {
    const w = { name: 'ExperimentalWarning', message: 'SQLite is an experimental feature' };
    assert.equal(isSuppressedWarning('uncaughtException', w), false);
});

test('isSuppressedWarning: returns false when data is null', () => {
    assert.equal(isSuppressedWarning('warning', null), false);
});

test('isSuppressedWarning: returns false when data is a primitive', () => {
    assert.equal(isSuppressedWarning('warning', 'some string'), false);
});

test('isSuppressedWarning: returns false when data.name is not ExperimentalWarning', () => {
    const w = { name: 'DeprecationWarning', message: 'SQLite is an experimental feature' };
    assert.equal(isSuppressedWarning('warning', w), false);
});

test('isSuppressedWarning: returns false when message does not include SQLite', () => {
    const w = { name: 'ExperimentalWarning', message: 'WebSockets is an experimental feature' };
    assert.equal(isSuppressedWarning('warning', w), false);
});

test('isSuppressedWarning: returns false when data.message is not a string', () => {
    const NON_STRING_VALUE = 42;
    const w = { name: 'ExperimentalWarning', message: NON_STRING_VALUE };
    assert.equal(isSuppressedWarning('warning', w), false);
});


// ── installWarningFilter: install + filter test ───────────────────────────

test('installWarningFilter: silences SQLite ExperimentalWarning via process.emit', () => {
    // Install the filter (idempotent — safe to call even if already installed)
    installWarningFilter();

    let warningFired = false;
    const listener = () => { warningFired = true; };
    process.on('warning', listener);

    // Emit a synthetic SQLite ExperimentalWarning — the patch must swallow it.
    process.emit('warning', Object.assign(new Error('SQLite is an experimental feature'), {
        name: 'ExperimentalWarning',
    }));

    process.off('warning', listener);
    assert.equal(warningFired, false,
        'SQLite ExperimentalWarning should have been suppressed by installWarningFilter');
});

test('installWarningFilter: passes through non-SQLite warnings unchanged', () => {
    installWarningFilter(); // idempotent

    let warningFired = false;
    const listener = () => { warningFired = true; };
    process.on('warning', listener);

    process.emit('warning', Object.assign(new Error('Fetch API is experimental'), {
        name: 'ExperimentalWarning',
    }));

    process.off('warning', listener);
    assert.equal(warningFired, true,
        'Non-SQLite warning should still be emitted normally');
});

test('installWarningFilter: is idempotent — calling twice does not double-wrap', () => {
    // Call multiple times — should not throw or double-wrap process.emit
    installWarningFilter();
    installWarningFilter();
    installWarningFilter();

    let count = 0;
    const listener = () => { count++; };
    process.on('warning', listener);

    // This non-SQLite warning should reach the listener exactly once
    process.emit('warning', Object.assign(new Error('Other experimental thing'), {
        name: 'ExperimentalWarning',
    }));

    process.off('warning', listener);
    assert.equal(count, 1, 'Warning should fire exactly once — not double/triple counted');
});
