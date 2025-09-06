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

const compile = async (
  filename: string,
  outputFilename: string,
  executable = false
) => {
  const args = ["compile", filename, outputFilename];
  if (executable) {
    args.push("--executable");
  }
  return await spawnCapture(process.argv0, args);
};

const run = async (filename: string) =>
  await spawnCapture(process.argv0, [filename]);

if (false) {
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

    it("can create a self-contained executable", async () => {
      const tmpExe = `${tmpDir}/hello_exe`;

      // Create a self-contained executable
      const compileResult = await compile("fixtures/hello.js", tmpExe, true);

      expect(compileResult.stderr).toEqual("");
      expect(compileResult.signal).toEqual(undefined);

      // Check that the file exists and is executable
      const stat = await fs.stat(tmpExe);
      expect(stat.isFile()).toBe(true);

      // 0o111 is the executable bits (uga+x)
      expect(!!(stat.mode & 0o111)).toBe(true);
    });

    afterAll(async () => {
      await fs.rmdir(tmpDir, { recursive: true });
    });
  });
}
