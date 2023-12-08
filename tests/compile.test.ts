import fs from 'fs/promises';
import { spawn } from 'child_process';
import { tmpdir } from 'os';

const spawnCapture = async (cmd: string, args: string[]) => {
    const child = spawn(cmd, args);

    let error;
    child.on('error', (err) => { error = err });

    let status = -1;
    let signal = undefined;
    child.on('exit', (code, sig) => {
        status = code ?? -1;
        signal = sig; // 'SIGILL' when there is a panic.
    });

    let stdout = '';
    child.stdout.on('data', (data) => {
        stdout += data.toString();
    });

    let stderr = '';
    child.stderr.on('data', (data) => {
        stderr += data.toString();
    });

    await new Promise((resolve) => child.on('exit', resolve));

    return {
        stdout: stdout ?? '',
        stderr: stderr ?? '',
        status,
        signal,
        error
    }
}

const compile = async (filename: string, outputFilename: string) => {
    return await spawnCapture(
        process.argv0,
        ['compile', filename, outputFilename]);
}

const run = async (filename: string) => {
    return await spawnCapture(
        process.argv0,
        [filename]);
}

describe('llrt compile', async () => {
    const tmpDir = await fs.mkdtemp(`${tmpdir}/llrt-test-compile`);
    const cases = [{
        name: 'empty', filename: 'fixtures/empty.js', expected: { stdout: '', stderr: '', status: 0 },
    }, {
        name: 'console.log', filename: 'fixtures/hello.js', expected: { stdout: 'hello world!\n', status: 0 },
    }, {
        name: 'throws', filename: 'fixtures/throw.js', expected: { stdout: '', stderr: 'Error: 42\n', status: 1 },
    }];

    cases.forEach(async c => {
        it(`can compile and run ${c.name}`, async () => {
            const tmpOutput = `${tmpDir}/${c.name}.lrt`;
            // compile.
            {
                const child = await compile(c.filename, tmpOutput);
                if (c.expected.compileError && typeof c.expected.stderr !== 'undefined') {
                    if (c.expected.stderr instanceof RegExp) {
                        assert.match(child.stderr, c.expected.stderr)
                    } else {
                        assert.strictEqual(child.stderr, c.expected.stderr)
                    };
                }
                if (c.expected.signal) {
                    assert.strictEqual(child.signal, c.expected.signal);
                }
                if (c.expected.compileError) {
                    return;
                }
            }

            // run.
            {
                const child = await run(tmpOutput);
                if (typeof c.expected.stdout !== 'undefined') {
                    assert.strictEqual(child.stdout, c.expected.stdout);
                }

                if (typeof c.expected.stderr !== 'undefined') {
                    if (c.expected.stderr instanceof RegExp) {
                        assert.match(child.stderr, c.expected.stderr)
                    } else {
                        assert.strictEqual(child.stderr, c.expected.stderr)
                    };
                }

                if (typeof c.expected.status !== 'undefined') {
                    assert.strictEqual(child.status, c.expected.status);
                }
            }
        })
    });

    afterAll(async () => {
        await fs.rmdir(tmpDir, { recursive: true });
    })
});
