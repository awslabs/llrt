// Tests for the executable compilation feature
// This verifies that we can create a self-contained executable that bundles
// LLRT with compiled JavaScript bytecode

import { spawn, ChildProcessWithoutNullStreams } from "child_process";
import * as fs from "fs";
import * as path from "path";
import { test, describe, it } from "vitest";
import { platform } from "os";

const IS_WINDOWS = platform() === "win32";

describe("executable compilation", () => {
  it("should compile JavaScript to a self-contained executable", async () => {
    // Create a temporary test script
    const testScript = `
      console.log("LLRT executable test");
      console.log("Args:", process.argv);
      process.exit(42);
    `;

    const tmpDir: string = path.join(process.cwd(), "tmp");
    try {
      fs.mkdirSync(tmpDir, { recursive: true });
    } catch (e) {
      // Directory might already exist
    }

    const scriptPath: string = path.join(tmpDir, "exe_test.js");
    const exePath: string = path.join(tmpDir, "exe_test");

    try {
      // Clean up existing files if they exist
      if (fs.existsSync(exePath)) {
        fs.unlinkSync(exePath);
      }
      
      fs.writeFileSync(scriptPath, testScript);

      // Use cargo run to compile the script (more reliable in test environment)
      const compileCommand = IS_WINDOWS 
        ? `cargo run -- compile "${scriptPath}" "${exePath}" --executable`
        : `cargo run -- compile ${scriptPath} ${exePath} --executable`;
      
      console.log(`Running compile command: ${compileCommand}`);
      
      // Execute the compilation
      const compileResult = await new Promise<number>((resolve, reject) => {
        const compile = spawn("sh", ["-c", compileCommand]);
        
        let stdout = "";
        let stderr = "";
        
        compile.stdout.on("data", (data: Buffer) => {
          stdout += data.toString();
        });
        
        compile.stderr.on("data", (data: Buffer) => {
          stderr += data.toString();
        });
        
        compile.on("error", (err: Error) => {
          console.error("Compile error:", err.message);
          reject(err);
        });
        
        compile.on("close", (code: number | null) => {
          console.log("Compile stdout:", stdout);
          console.log("Compile stderr:", stderr);
          console.log("Compile exit code:", code);
          resolve(code || 0);
        });
      });
      
      // Basic assertions without expect
      if (compileResult !== 0) {
        throw new Error(`Compilation failed with exit code ${compileResult}`);
      }
      
      if (!fs.existsSync(exePath)) {
        throw new Error("Executable file was not created");
      }
      
      // Check executable permissions on non-Windows platforms
      if (!IS_WINDOWS) {
        const stats = fs.statSync(exePath);
        // Just verify it's a file and is not empty
        if (!stats.isFile()) {
          throw new Error("Created file is not a regular file");
        }
        if (stats.size <= 0) {
          throw new Error("Created file is empty");
        }
      }
      
      // Run the executable
      const execResult = await new Promise<{code: number | null, stdout: string}>((resolve, reject) => {
        const exe = spawn(exePath);
        let stdout = "";
        let stderr = "";
        
        exe.stdout.on("data", (data: Buffer) => {
          stdout += data.toString();
        });
        
        exe.stderr.on("data", (data: Buffer) => {
          stderr += data.toString();
        });
        
        exe.on("error", (err: Error) => {
          console.error("Execute error:", err.message);
          reject(err);
        });
        
        exe.on("close", (code: number | null) => {
          console.log("Execute stdout:", stdout);
          console.log("Execute stderr:", stderr);
          console.log("Execute exit code:", code);
          resolve({code, stdout});
        });
      });
      
      // Verify exit code and output
      if (execResult.code !== 42) {
        throw new Error(`Expected exit code 42, got ${execResult.code}`);
      }
      
      if (!execResult.stdout.includes("LLRT executable test")) {
        throw new Error(`Expected output to contain "LLRT executable test", got: ${execResult.stdout}`);
      }
      
      console.log("✅ Executable test passed successfully!");
    } finally {
      // Clean up
      try {
        if (fs.existsSync(scriptPath)) fs.unlinkSync(scriptPath);
        if (fs.existsSync(exePath)) fs.unlinkSync(exePath);
        fs.rmdirSync(tmpDir, { recursive: true });
      } catch (e) {
        console.warn("Cleanup failed:", e);
      }
    }
  }, { timeout: 60000 }); // Increase timeout to 60 seconds
}); 