import {
  access,
  constants,
  mkdtemp,
  readFile,
  rm,
  stat,
  writeFile,
} from "node:fs/promises";
import { platform, tmpdir } from "node:os";
import { join } from "node:path";
import { spawnCapture } from "./test-utils";

const TIMEOUT = 30000;
const IS_WINDOWS = platform() === "win32";

const compile = async (
  filename: string,
  outputFilename: string
): Promise<void> => {
  const args = ["compile", filename, outputFilename, "--executable"];
  const { code: compileResult } = await spawnCapture(process.argv0, args);
  if (compileResult !== 0) {
    throw new Error(`Compilation failed with exit code ${compileResult}`);
  }
};

const run = async (
  filename: string,
  args: string[] = []
): Promise<{ code: number; output: string }> => {
  const { code, stdout, stderr } = await spawnCapture(filename, args);
  return { code, output: stdout + stderr };
};

describe("executable compilation", () => {
  let tmpDir: string;
  let exePath: string;

  const createTestScript = async (content: string): Promise<string> => {
    const scriptPath = join(tmpDir, "exe_test.js");
    await writeFile(scriptPath, content, "utf8");
    return scriptPath;
  };

  beforeEach(async () => {
    tmpDir = await mkdtemp(join(tmpdir(), "llrt-test-compile"));
    exePath = join(tmpDir, "exe_test");
  });

  afterEach(async () => {
    //await rm(tmpDir, { force: true, recursive: true });
  });

  it(
    "should compile JavaScript to a self-contained executable",
    async () => {
      const scriptPath = await createTestScript(
        "console.log('test'); process.exit(0);"
      );

      await compile(scriptPath, exePath);

      const stats = await stat(exePath);

      if (!stats.isFile()) {
        throw new Error("Created file is not a regular file");
      }
      if (stats.size <= 0) {
        throw new Error("Created file is empty");
      }

      if (!IS_WINDOWS) {
        await access(exePath, constants.X_OK);
      }
      const { code: execResult, output } = await run(exePath);
      if (output.trim() !== "test") {
        throw new Error(`Expected output 'test', got '${output}'`);
      }
      if (execResult !== 0) {
        throw new Error(`Expected exit code 0, got ${execResult}`);
      }
    },
    TIMEOUT
  );

  it(
    "should result in the correct exit code",
    async () => {
      const scriptPath = await createTestScript(
        "console.log('exiting'); process.exit(42);"
      );
      await compile(scriptPath, exePath);

      const { code: execResult, output } = await run(exePath);
      if (output.trim() !== "exiting") {
        throw new Error(`Expected output 'exiting', got '${output}'`);
      }
      if (execResult !== 42) {
        throw new Error(`Expected exit code 42, got ${execResult}`);
      }
    },
    TIMEOUT
  );

  it(
    "should result in the correct exit code passed via argument",
    async () => {
      const scriptPath = await createTestScript(
        "console.log('arg:', process.argv[1]); process.exit(process.argv[1]);"
      );
      await compile(scriptPath, exePath);

      const { code: execResult, output } = await run(exePath, ["1"]);
      if (output.trim() !== "arg: 1") {
        throw new Error(`Expected output 'arg: 1', got '${output}'`);
      }
      if (execResult !== 1) {
        throw new Error(`Expected exit code 1, got ${execResult}`);
      }
    },
    TIMEOUT
  );

  it(
    "should log output from the executable",
    async () => {
      const testOutput = "test content";
      const scriptPath = await createTestScript(`
        console.log('${testOutput}');
      `);

      await compile(scriptPath, exePath);

      const { code: execResult, output } = await run(exePath);
      if (execResult !== 0) {
        throw new Error(`Expected exit code 0, got ${execResult}`);
      }

      if (output.trim() !== testOutput) {
        throw new Error(
          `Expected output to be '${testOutput}', but got '${output}'`
        );
      }
    },
    TIMEOUT
  );
});
