import fs from "fs/promises";
import { spawn } from "child_process";
import { tmpdir } from "os";

const spawnCapture = async (cmd: string, args: string[]) => {
  const child = spawn(cmd, args);

  let stdout = "";
  let stderr = "";

  child.stdout.on("data", (data) => {
    stdout += data.toString();
  });

  child.stderr.on("data", (data) => {
    stderr += data.toString();
  });

  const [status, signal] = await new Promise<
    [number | null, NodeJS.Signals | null]
  >((resolve) => {
    child.on("close", (code, sig) => {
      resolve([code ?? -1, sig]);
    });
  });

  return { stdout, stderr, status, signal };
};

const compile = async (filename: string, outputFilename: string) =>
  await spawnCapture(process.argv0, ["compile", filename, outputFilename]);

const run = async (filename: string) =>
  await spawnCapture(process.argv0, [filename]);

describe("llrt compile", async () => {
  const tmpDir = await fs.mkdtemp(`${tmpdir()}/llrt-test-compile`);

  it("can compile and run empty", async () => {
    const tmpOutput = `${tmpDir}/empty.lrt`;

    const compileResult = await compile("fixtures/empty.js", tmpOutput);

    assert.strictEqual(compileResult.stderr, "");
    assert.strictEqual(compileResult.signal, undefined);

    const runResult = await run(tmpOutput);

    assert.strictEqual(runResult.stdout, "");
    assert.strictEqual(runResult.stderr, "");
    assert.strictEqual(runResult.status, 0);
  });

  it("can compile and run console.log", async () => {
    const tmpOutput = `${tmpDir}/console.log.lrt`;

    const compileResult = await compile("fixtures/hello.js", tmpOutput);

    assert.strictEqual(compileResult.stderr, "");
    assert.strictEqual(compileResult.signal, undefined);

    const runResult = await run(tmpOutput);

    assert.strictEqual(runResult.stdout, "hello world!\n");
    assert.strictEqual(runResult.stderr, "");
    assert.strictEqual(runResult.status, 0);
  });

  it("can compile and run throws", async () => {
    const tmpOutput = `${tmpDir}/throws.lrt`;

    const compileResult = await compile("fixtures/throw.js", tmpOutput);

    assert.strictEqual(compileResult.stderr, "");
    assert.strictEqual(compileResult.signal, undefined);

    const runResult = await run(tmpOutput);

    assert.strictEqual(runResult.stdout, "");
    assert.strictEqual(runResult.stderr, "Error: 42\n");
    assert.strictEqual(runResult.status, 1);
  });

  afterAll(async () => {
    await fs.rmdir(tmpDir, { recursive: true });
  });
});
