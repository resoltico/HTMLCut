import test from 'node:test';
import assert from 'node:assert/strict';
// ES module imports are hoisted, so this line executes AFTER storage.js is
// parsed but BEFORE any test code runs. It works only because storage.js uses
// lazy ensureDb() initialization — the DB path is read on first call, not at
// module load time. Changing storage.js to eagerly open the DB would break this.
process.env.HTMLCUT_DB_PATH = ':memory:';
import { logExtraction, getHistoryGroupedBySuccess } from '../src/storage.js';

const TEST_DURATION_MS = 50;

test('Storage lazily creates DB and handles empty runs gracefully', () => {
    const history = getHistoryGroupedBySuccess();
    // Native groupBy on empty returns gracefully empty object with null prototype
    assert.deepEqual(history, Object.create(null));
});

test('Storage handles logging and groups by success natively', () => {
    logExtraction({
        source: 'test-source-success',
        startPattern: 'start',
        endPattern: 'end',
        success: true,
        durationMs: TEST_DURATION_MS
    });

    logExtraction({
        source: 'test-source-fail',
        startPattern: 'start',
        endPattern: 'end',
        success: false,
        durationMs: 0
    });

    const history = getHistoryGroupedBySuccess();

    assert.ok(history.successful, 'Expected successful group to exist');
    assert.ok(history.failed, 'Expected failed group to exist');

    const successfulRuns = history.successful.filter(run => run.source === 'test-source-success');
    assert.ok(successfulRuns.length >= 1, 'Expected at least one successful run');
    assert.equal(successfulRuns[0].duration_ms, TEST_DURATION_MS);
    assert.equal(successfulRuns[0].success, true);

    const failedRuns = history.failed.filter(run => run.source === 'test-source-fail');
    assert.ok(failedRuns.length >= 1, 'Expected at least one failed run');
    assert.equal(failedRuns[0].success, false);
});

test('Storage prunes rows beyond 1000 limit on insert', () => {
    // Insert 1005 rows on top of 2 from the previous test = 1007 total.
    // Pruning keeps the 1000 most recent, discarding the 7 oldest (which includes
    // both rows from the previous test). The display query returns 50 rows,
    // all of which must be prune-test rows with no failed rows remaining in view.
    const BATCH = 1005;
    const DISPLAY_LIMIT = 50;

    for (let i = 0; i < BATCH; i++) {
        logExtraction({
            source: `prune-test-${i}`,
            startPattern: 's',
            endPattern: 'e',
            success: true,
            durationMs: i,
        });
    }

    const history = getHistoryGroupedBySuccess();
    const pruneRuns = history.successful.filter(run => run.source.startsWith('prune-test-'));
    assert.equal(
        pruneRuns.length, DISPLAY_LIMIT,
        `Expected exactly ${DISPLAY_LIMIT} prune-test rows in display window, found ${pruneRuns.length}`
    );
    assert.equal(history.failed, undefined, 'Oldest failed row must have been pruned out of view');
});

