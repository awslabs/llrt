import { spawn } from "child_process";
import {
  accessSync,
  constants,
  mkdtempSync,
  readFileSync,
  rmSync,
  statSync,
  writeFileSync,
} from "fs";
import { platform, tmpdir } from "os";
import { join } from "path";

const TIMEOUT = 30000;
const IS_WINDOWS = platform() === "win32";

const spawnCapture = async (cmd: string, args: string[] = []) =>
  await new Promise<number>((resolve, reject) =>
    spawn(cmd, args)
      .on("close", (code) => resolve(code ?? -1))
      .on("error", (err) => reject(err))
  );

const compile = async (filename: string, outputFilename: string) => {
  const args = ["compile", filename, outputFilename, "--executable"];
  const compileResult = await spawnCapture(process.argv0, args);
  if (compileResult !== 0) {
    throw new Error(`Compilation failed with exit code ${compileResult}`);
  }
};

const run = async (filename: string, args: string[] = []) =>
  await spawnCapture(filename, args);

describe("executable compilation", () => {
  let tmpDir: string;
  let exePath: string;
  const createTestScript = (content: string) => {
    const scriptPath = join(tmpDir, "exe_test.js");
    writeFileSync(scriptPath, content, "utf8");
    return scriptPath;
  };

  beforeEach(() => {
    tmpDir = mkdtempSync(join(tmpdir(), "llrt-test-compile"));
    exePath = join(tmpDir, "exe_test");
  });

  afterEach(async () => {
    rmSync(tmpDir, { force: true, recursive: true });
  });

  it(
    "should compile JavaScript to a self-contained executable",
    async () => {
      const scriptPath = createTestScript("process.exit(0);");

      await compile(scriptPath, exePath);

      const stats = statSync(exePath);
      if (!stats.isFile()) {
        throw new Error("Created file is not a regular file");
      } else if (stats.size <= 0) {
        throw new Error("Created file is empty");
      }

      // Check executable permissions on non-Windows platforms
      if (!IS_WINDOWS) {
        accessSync(exePath, constants.X_OK);
      }

      const execResult = await run(exePath);
      if (execResult !== 0) {
        throw new Error(`Expected exit code 0, got ${execResult}`);
      }
    },
    TIMEOUT
  );

  it(
    "should result in the correct exit code",
    async () => {
      const scriptPath = createTestScript("process.exit(42);");

      await compile(scriptPath, exePath);

      const execResult = await run(exePath);
      if (execResult !== 42) {
        throw new Error(`Expected exit code 42, got ${execResult}`);
      }
    },
    TIMEOUT
  );

  it(
    "should result in the correct exit code passed via argument",
    async () => {
      const scriptPath = createTestScript("process.exit(process.argv[1]);");

      await compile(scriptPath, exePath);

      const execResult = await run(exePath, ["1"]);
      if (execResult !== 1) {
        throw new Error(`Expected exit code 1, got ${execResult}`);
      }
    },
    TIMEOUT
  );

  it(
    "should write a file as part of the executable",
    async () => {
      const testFile = "test.txt";
      const expectation = "content";
      const scriptPath = createTestScript(`
        import {writeFileSync} from 'fs';
        import {join} from 'path';

        writeFileSync(join(process.argv[1], '${testFile}'), '${expectation}', 'utf8');
      `);

      await compile(scriptPath, exePath);

      const execResult = await run(exePath, [tmpDir]);
      if (execResult !== 0) {
        throw new Error(`Expected exit code 1, got ${execResult}`);
      }

      const content = readFileSync(join(tmpDir, testFile), "utf8");
      if (content !== expectation) {
        throw new Error(
          `Expected content of the test file to be '${expectation}', but got '${content}'`
        );
      }
    },
    TIMEOUT
  );
});
