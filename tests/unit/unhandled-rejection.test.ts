import { mkdtemp, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { spawnCapture } from "./test-utils";

describe("unhandled promise rejections", () => {
  let tmpDir: string;

  const run = async (source: string) => {
    const scriptPath = join(tmpDir, "script.mjs");
    await writeFile(scriptPath, source, "utf8");
    return spawnCapture(process.argv0, [scriptPath]);
  };

  beforeEach(async () => {
    tmpDir = await mkdtemp(join(tmpdir(), "llrt-test-unhandled-"));
  });

  afterEach(async () => {
    await rm(tmpDir, { recursive: true, force: true });
  });

  it("should print the error and exit with code 1 for an unhandled async throw", async () => {
    const { code, stdout, stderr } = await run(`
      async function main() { throw new Error("async boom"); }
      main();
      console.log("after");
    `);
    expect(code).toEqual(1);
    expect(stdout).toContain("after");
    expect(stderr).toContain("Error: async boom");
  });

  it("should exit with code 1 for an unhandled rejection with a non-error reason", async () => {
    const { code, stderr } = await run(`Promise.reject("plain reason");`);
    expect(code).toEqual(1);
    expect(stderr).toContain("plain reason");
  });

  it("should not report a rejection handled in the same tick", async () => {
    const { code, stderr } = await run(`
      const p = Promise.reject(new Error("handled"));
      p.catch(() => {});
      console.log("done");
    `);
    expect(code).toEqual(0);
    expect(stderr).toEqual("");
  });

  it("should not report a rejection handled after a microtask tick", async () => {
    const { code, stderr } = await run(`
      const p = Promise.reject(new Error("handled later"));
      await Promise.resolve();
      await Promise.resolve();
      p.catch(() => {});
      console.log("done");
    `);
    expect(code).toEqual(0);
    expect(stderr).toEqual("");
  });

  it("should not report a rejection handled in a microtask queued from a native event", async () => {
    const { code, stderr } = await run(`
      import { spawn } from "node:child_process";
      const child = spawn(process.argv0, ["-v"]);
      child.on("exit", () => {
        const p = Promise.reject(new Error("handled in microtask"));
        queueMicrotask(() => p.catch(() => {}));
      });
    `);
    expect(code).toEqual(0);
    expect(stderr).toEqual("");
  });

  it("should not report a rejected promise that is awaited in try/catch", async () => {
    const { code, stdout, stderr } = await run(`
      async function main() { throw new Error("caught"); }
      try {
        await main();
      } catch (e) {
        console.log("caught:", e.message);
      }
    `);
    expect(code).toEqual(0);
    expect(stdout).toContain("caught: caught");
    expect(stderr).toEqual("");
  });

  it("should keep exiting with code 1 for a synchronous throw", async () => {
    const { code, stderr } = await run(`throw new Error("sync boom");`);
    expect(code).toEqual(1);
    expect(stderr).toContain("Error: sync boom");
  });
});
