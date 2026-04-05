import process from "node:process";

import defaultImport from "node:process";
import legacyImport from "process";
import { spawnCapture } from "./test-utils";
it("node:process should be the same as process", () => {
  expect(defaultImport).toStrictEqual(legacyImport);
});

const {
  env,
  cwd,
  argv0,
  argv,
  platform,
  arch,
  hrtime,
  release,
  version,
  versions,
  exit,
} = defaultImport;

it("should have a process env", () => {
  expect(env).toEqual(process.env);
});

it("should have a process cwd", () => {
  expect(cwd()).toEqual(process.cwd());
});

it("should have a process argv0", () => {
  expect(argv0).toEqual(process.argv0);
});

it("should have a process argv", () => {
  expect(argv).toEqual(process.argv);
});

it("should have a process platform", () => {
  expect(platform).toEqual(process.platform);
});

it("should have a process arch", () => {
  expect(arch).toEqual(process.arch);
});

it("should have a process hrtime", () => {
  expect(hrtime.bigint() > 0).toBeTruthy();
});

it("should have a process release", () => {
  expect(release).toEqual(process.release);
});

it("should have a process version", () => {
  expect(version).toEqual(process.version);
});

it("should have a process versions", () => {
  expect(versions).toEqual(process.versions);
});

it("should have a process exit", () => {
  expect(exit).toEqual(process.exit);
});

// ── exitCode accessor ─────────────────────────────────────────────────────────

it("exitCode round-trip: set then get returns same value", () => {
  process.exitCode = 42;
  expect(process.exitCode).toBe(42);
  process.exitCode = 0; // reset so the process exits cleanly
});

// ── exit() validation ─────────────────────────────────────────────────────────

it("process.exit(1.5) throws RangeError", () => {
  expect(() => process.exit(1.5)).toThrow(RangeError);
});

// ── __exit_listeners not on globalThis ────────────────────────────────────────

it("__exit_listeners is not accessible from JS (Rust-side only)", () => {
  expect("__exit_listeners" in globalThis).toBe(false);
});

// ── process.on / process.off / process.once ───────────────────────────────────
// These tests use spawnCapture so that exit-listener side-effects (writes to
// stderr) can be observed from outside the process being tested.

const LLRT = process.argv[0];

it("process.on: multiple listeners are all called on exit", async () => {
  const { code, stderr } = await spawnCapture(LLRT, [
    "-e",
    `
    let out = '';
    process.on('exit', (code) => { process.stderr.write('A' + code); });
    process.on('exit', (code) => { process.stderr.write('B' + code); });
    `,
  ]);
  expect(code).toBe(0);
  expect(stderr).toContain("A0");
  expect(stderr).toContain("B0");
});

it("process.off: removed listener is not called on exit", async () => {
  const { code, stderr } = await spawnCapture(LLRT, [
    "-e",
    `
    const cb = () => process.stderr.write('SHOULD-NOT-FIRE');
    process.on('exit', cb);
    process.off('exit', cb);
    process.stderr.write('REMOVED');
    `,
  ]);
  expect(code).toBe(0);
  expect(stderr).toContain("REMOVED");
  expect(stderr).not.toContain("SHOULD-NOT-FIRE");
});

it("process.once: callback fires exactly once", async () => {
  const { code, stderr } = await spawnCapture(LLRT, [
    "-e",
    `
    let n = 0;
    process.once('exit', () => { n++; process.stderr.write('ONCE:' + n); });
    `,
  ]);
  expect(code).toBe(0);
  // Only one 'ONCE:' prefix should appear, and n should be 1.
  const matches = stderr.match(/ONCE:/g);
  expect(matches?.length).toBe(1);
  expect(stderr).toContain("ONCE:1");
});

it("process.on: listeners receive the exit code", async () => {
  const { code, stderr } = await spawnCapture(LLRT, [
    "-e",
    `
    process.exitCode = 7;
    process.on('exit', (code) => { process.stderr.write('CODE:' + code); });
    `,
  ]);
  expect(code).toBe(7);
  expect(stderr).toContain("CODE:7");
});
