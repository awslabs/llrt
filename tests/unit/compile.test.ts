import fs from "fs/promises";
import { spawn } from "child_process";
import { tmpdir, platform } from "os";
const IS_WIN = platform() === "win32";

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

if (!IS_WIN) {
  describe("llrt compile", async () => {
    const tmpDir = await fs.mkdtemp(`${tmpdir()}/llrt-test-compile`);

    it("can compile and run empty", async () => {
      const tmpOutput = `${tmpDir}/empty.lrt`;

      const compileResult = await compile("fixtures/empty.js", tmpOutput);

      expect(compileResult.stderr).toEqual("");
      expect(compileResult.signal).toEqual(undefined);

      const runResult = await run(tmpOutput);

      expect(runResult.stdout).toEqual("");
      expect(runResult.stderr).toEqual("");
      expect(runResult.status).toEqual(0);
    });

    it("can compile and run console.log", async () => {
      const tmpOutput = `${tmpDir}/console.log.lrt`;

      const compileResult = await compile("fixtures/hello.js", tmpOutput);

      expect(compileResult.stderr).toEqual("");
      expect(compileResult.signal).toEqual(undefined);

      const runResult = await run(tmpOutput);

      expect(runResult.stdout).toEqual("hello world!\n");
      expect(runResult.stderr).toEqual("");
      expect(runResult.status).toEqual(0);
    });

    it("can compile and run throws", async () => {
      const tmpOutput = `${tmpDir}/throws.lrt`;

      const compileResult = await compile("fixtures/throw.js", tmpOutput);

      expect(compileResult.stderr).toEqual("");
      expect(compileResult.signal).toEqual(undefined);

      const runResult = await run(tmpOutput);

      expect(runResult.stdout).toEqual("");
      expect(runResult.stderr).toEqual("42\n");
      expect(runResult.status).toEqual(1);
    });

    afterAll(async () => {
      await fs.rmdir(tmpDir, { recursive: true });
    });
  });
}
