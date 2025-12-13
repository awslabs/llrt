import fs from "node:fs/promises";
import { tmpdir, platform } from "node:os";
import { spawnCapture } from "./test-utils";

const IS_WINDOWS = platform() === "win32";

const compile = async (
  filename: string,
  outputFilename: string,
  executable = false
) => {
  const args = ["compile", filename, outputFilename];
  if (executable) {
    args.push("--executable");
  }
  const { code, stdout, stderr } = await spawnCapture(process.argv0, args);
  return { stdout, stderr, status: code, signal: undefined };
};

const run = async (filename: string) => {
  const { code, stdout, stderr } = await spawnCapture(process.argv0, [
    filename,
  ]);
  return { stdout, stderr, status: code };
};

const runExecutable = async (filename: string) => {
  const { code, stdout, stderr } = await spawnCapture(filename, []);
  return { stdout, stderr, status: code };
};

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
    // On Windows, executables need .exe extension
    const tmpExe = IS_WINDOWS
      ? `${tmpDir}/hello_exe.exe`
      : `${tmpDir}/hello_exe`;

    // Create a self-contained executable
    const compileResult = await compile("fixtures/hello.js", tmpExe, true);

    expect(compileResult.stderr).toEqual("");
    expect(compileResult.signal).toEqual(undefined);

    // Check that the file exists and is executable
    const stat = await fs.stat(tmpExe);
    expect(stat.isFile()).toBe(true);

    if (IS_WINDOWS) {
      // On Windows, verify the file has .exe extension (which makes it executable)
      expect(tmpExe.endsWith(".exe")).toBe(true);
    } else {
      // On Unix, verify executable bits are set (0o111 is uga+x)
      expect(!!(stat.mode & 0o111)).toBe(true);
    }

    // Verify the executable actually runs correctly on both platforms
    const runResult = await runExecutable(tmpExe);
    expect(runResult.stdout).toEqual("hello world!\n");
    expect(runResult.status).toEqual(0);
  });

  afterAll(async () => {
    await fs.rmdir(tmpDir, { recursive: true });
  });
});
