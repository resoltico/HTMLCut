import { execFile, spawn } from 'node:child_process';
import { mkdtemp, rm } from 'node:fs/promises';
import http from 'node:http';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { promisify } from 'node:util';

const execFileAsync = promisify(execFile);

export const CLI_PATH = join(process.cwd(), 'bin', 'htmlcut.js');

export function runCli(args, { env = {} } = {}) {
    return execFileAsync(process.execPath, [CLI_PATH, ...args], {
        env: {
            ...process.env,
            ...env,
        },
    });
}

export function runCliWithStdin(args, stdin, { env = {} } = {}) {
    return new Promise((resolve, reject) => {
        const child = spawn(process.execPath, [CLI_PATH, ...args], {
            env: {
                ...process.env,
                ...env,
            },
        });

        let stdout = '';
        let stderr = '';

        child.stdout.on('data', chunk => {
            stdout += chunk;
        });

        child.stderr.on('data', chunk => {
            stderr += chunk;
        });

        child.on('error', reject);
        child.on('close', code => {
            resolve({ code, stdout, stderr });
        });

        child.stdin.end(stdin);
    });
}

export async function withTempDir(run) {
    const dir = await mkdtemp(join(tmpdir(), 'htmlcut-test-'));
    try {
        return await run(dir);
    } finally {
        await rm(dir, { recursive: true, force: true });
    }
}

export async function withServer(handler, run) {
    const server = http.createServer(handler);
    await new Promise(resolve => server.listen(0, '127.0.0.1', resolve));
    const address = server.address();
    const url = `http://127.0.0.1:${address.port}`;

    try {
        return await run({ server, url });
    } finally {
        await new Promise(resolve => server.close(resolve));
    }
}
