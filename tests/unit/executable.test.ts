import { spawn } from "child_process";
import {
  accessSync,
  constants,
  mkdtempSync,
  rmSync,
  statSync,
  writeFileSync,
} from "fs";
import { platform, tmpdir } from "os";
import { join } from "path";

const IS_WINDOWS = platform() === "win32";

describe("executable compilation", () => {
  let tmpDir: string;

  beforeEach(() => {
    tmpDir = mkdtempSync(join(tmpdir(), "llrt-test-compile"));
  });

  afterEach(async () => {
    rmSync(tmpDir, { force: true, recursive: true });
  });

  it("should compile JavaScript to a self-contained executable", async function () {
    // Create a temporary test script
    const testScript = `
      console.log("LLRT executable test");
      console.log("Args:", process.argv);
      process.exit(42);
    `;

    const scriptPath = join(tmpDir, "exe_test.js");
    const exePath = join(tmpDir, "exe_test");

    writeFileSync(scriptPath, testScript, "utf8");

    const compileResult = await new Promise<number>((resolve, reject) => {
      // Use cargo run to compile the script (more reliable in test environment)
      spawn("cargo", [
        "run",
        "--",
        "compile",
        scriptPath,
        exePath,
        "--executable",
      ])
        .on("error", (err: Error) => reject(err))
        .on("close", (code: number | null) => resolve(code ?? -1));
    });

    if (compileResult !== 0) {
      throw new Error(`Compilation failed with exit code ${compileResult}`);
    }

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

    // Run the executable
    const execResult = await new Promise<number>((resolve, reject) =>
      spawn(exePath)
        .on("error", (err: Error) => reject(err))
        .on("close", (code: number | null) => resolve(code ?? -1))
    );

    // Verify exit code and output
    if (execResult !== 42) {
      throw new Error(`Expected exit code 42, got ${execResult}`);
    }
  }, 60000);
});
