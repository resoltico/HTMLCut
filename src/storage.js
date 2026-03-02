/**
 * @fileoverview Native node:sqlite history manager.
 */
import { DatabaseSync } from 'node:sqlite';
import { homedir } from 'node:os';
import { join } from 'node:path';

const SUCCESS = 1;
const FAILURE = 0;
const MAX_HISTORY_SIZE = 1000;
const HISTORY_DISPLAY_LIMIT = 50;

let dbInstance = null;
let stmtInsert = null;
let stmtPrune = null;
let stmtSelect = null;

function ensureDb() {
    if (dbInstance === null) {
        const DB_PATH = process.env.HTMLCUT_DB_PATH || join(homedir(), '.htmlcut_history.db');
        const db = new DatabaseSync(DB_PATH);
        db.exec(`
            CREATE TABLE IF NOT EXISTS runs (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                timestamp TEXT,
                source TEXT,
                start_pattern TEXT,
                end_pattern TEXT,
                success INTEGER,
                duration_ms INTEGER
            )
        `);
        stmtInsert = db.prepare(`
            INSERT INTO runs (timestamp, source, start_pattern, end_pattern, success, duration_ms)
            VALUES (?, ?, ?, ?, ?, ?)
        `);
        stmtPrune = db.prepare(`
            DELETE FROM runs
            WHERE id NOT IN (
                SELECT id FROM runs ORDER BY id DESC LIMIT ${MAX_HISTORY_SIZE}
            )
        `);
        stmtSelect = db.prepare(`
            SELECT id, timestamp, source, start_pattern, end_pattern, success, duration_ms
            FROM runs ORDER BY id DESC LIMIT ${HISTORY_DISPLAY_LIMIT}
        `);
        dbInstance = db;
    }
}

/**
 * @typedef {Object} ExtractionLog
 * @property {string} source
 * @property {string} startPattern
 * @property {string} endPattern
 * @property {boolean} success
 * @property {number} durationMs
 */

/**
 * Logs a single extraction event to the local SQLite DB.
 * @param {ExtractionLog} log The log payload.
 */
export function logExtraction({ source, startPattern, endPattern, success, durationMs }) {
    ensureDb();
    stmtInsert.run(new Date().toISOString(), source, startPattern, endPattern, success ? SUCCESS : FAILURE, durationMs);
    stmtPrune.run();
}

/**
 * Returns the most recent extraction logs grouped by outcome.
 * The `success` field on each row is a boolean.
 * @returns {Partial<Record<"successful" | "failed", unknown[]>>}
 */
export function getHistoryGroupedBySuccess() {
    ensureDb();
    const rows = stmtSelect.all();
    const mapped = rows.map(row => ({ ...row, success: row.success === SUCCESS }));
    return Object.groupBy(mapped, row => row.success ? 'successful' : 'failed');
}
