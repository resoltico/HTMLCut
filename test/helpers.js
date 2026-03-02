/**
 * @fileoverview Shared test helpers for extractor and stream tests.
 */

/** Builds an async iterable from the given parts. */
export async function* chunksOf(...parts) {
    for (const p of parts) { yield p; }
}

/** Fully consumes an async generator, discarding all yielded values. */
export async function drain(gen) {
    for await (const _fragment of gen) { }
}

/** Fully consumes an async generator and returns all yielded values. */
export async function collect(gen) {
    const results = [];
    for await (const frag of gen) { results.push(frag); }
    return results;
}
