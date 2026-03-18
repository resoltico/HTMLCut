import test from 'node:test';
import assert from 'node:assert/strict';
import { access, readFile } from 'node:fs/promises';
import pkg from '../package.json' with { type: 'json' };

test('package: htmlcut bin points at the stable wrapper entrypoint', async () => {
    assert.equal(pkg.bin.htmlcut, './bin/htmlcut.js');
    await access(new URL('../bin/htmlcut.js', import.meta.url));
});

test('package: wrapper entrypoint delegates to the CLI module', async () => {
    const wrapper = await readFile(new URL('../bin/htmlcut.js', import.meta.url), 'utf8');
    assert.match(wrapper, /^#!\/usr\/bin\/env node/m);
    assert.match(wrapper, /import '\.\.\/src\/cli\.js';/);
});
