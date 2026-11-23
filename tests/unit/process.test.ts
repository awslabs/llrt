import process from "node:process";

import defaultImport from "node:process";
import legacyImport from "process";
import { isatty } from "node:tty";

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
  stdout,
  stderr,
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
  expect(versions.llrt).toEqual(process.version);
  expect(versions.node).toEqual("0.0.0");
});

it("should have a process exit", () => {
  expect(exit).toEqual(process.exit);
});

it("should have a process stdout and stderr", () => {
  expect(stdout).toBe(globalThis.process.stdout);
  expect(stderr).toBe(globalThis.process.stderr);
});

describe("process.stdout and process.stderr", () => {
  const run = (script: string) => spawnCapture(process.argv0, ["-e", script]);

  it("should set isTTY only when the stream is a terminal", () => {
    expect(process.stdout.isTTY).toBe(isatty(1) || undefined);
    expect(process.stderr.isTTY).toBe(isatty(2) || undefined);
  });

  it("should not define isTTY when output is piped", async () => {
    const { stdout } = await run(
      `process.stdout.write(["isTTY" in process.stdout, "isTTY" in process.stderr].join())`
    );
    expect(stdout).toEqual("false,false");
  });

  it("should write strings without a trailing newline", async () => {
    const { code, stdout, stderr } = await run(
      `process.stdout.write("out1"); process.stdout.write("out2\\n"); process.stderr.write("err")`
    );
    expect(code).toEqual(0);
    expect(stdout).toEqual("out1out2\n");
    expect(stderr).toEqual("err");
  });

  it("should keep call order with console.log", async () => {
    const { stdout } = await run(
      `process.stdout.write("a"); console.log("b"); process.stdout.write("c")`
    );
    expect(stdout).toEqual("ab\nc");
  });

  it("should write Buffers, typed arrays and DataViews", async () => {
    const { stdout } = await run(`
      process.stdout.write(Buffer.from("a"));
      process.stdout.write(new Uint8Array([98]));
      process.stdout.write(new Uint16Array([0x6463]));
      process.stdout.write(new DataView(new Uint8Array([0, 101, 0]).buffer, 1, 1));
    `);
    expect(stdout).toEqual("abcde");
  });

  it("should decode strings with the given encoding", async () => {
    const { stdout } = await run(
      `process.stdout.write("6869", "hex"); process.stdout.write("IQ==", "base64"); process.stdout.write("?", "utf8")`
    );
    expect(stdout).toEqual("hi!?");
  });

  it("should return true and call the callback asynchronously with null", async () => {
    const { stdout } = await run(`
      let called = false;
      const ret = process.stdout.write("x", (err) => {
        called = true;
        process.stdout.write(" cb:" + err);
      });
      process.stdout.write(" sync:" + called + " ret:" + ret);
      process.stderr.write("", "utf8", (err) => process.stdout.write(" errcb:" + err));
    `);
    expect(stdout).toEqual("x sync:false ret:true cb:null errcb:null");
  });

  it("should flush output written right before process.exit", async () => {
    const { code, stdout } = await run(
      `process.stdout.write("partial"); process.exit(3)`
    );
    expect(code).toEqual(3);
    expect(stdout).toEqual("partial");
  });

  it("should reject unknown encodings", () => {
    expect(() => process.stdout.write("", "nope" as any)).toThrow(
      "Unknown encoding: nope"
    );
    expect(() => process.stdout.write("", 123 as any)).toThrow(
      "Unknown encoding: 123"
    );
    expect(() => process.stdout.write("", true as any)).toThrow(
      "Unknown encoding: true"
    );
  });

  it("should default to utf8 for any falsy encoding", () => {
    for (const encoding of [undefined, null, "", false, 0, -0, 0n, NaN]) {
      expect(process.stdout.write("", encoding as any)).toBe(true);
    }
  });

  it("should reject badly encoded strings like Buffer.from", () => {
    const thrown = (fn: () => unknown) => {
      try {
        fn();
      } catch (e) {
        return e as Error;
      }
      throw new Error("expected a throw");
    };
    const expected = thrown(() => Buffer.from("abc", "hex"));
    const actual = thrown(() => process.stdout.write("abc", "hex"));
    expect(actual.constructor).toBe(expected.constructor);
    expect(actual.message).toEqual(expected.message);
  });
});

it("should pass three or four extra arguments to a nextTick callback", async () => {
  for (const args of [
    [1, 2, 3],
    [1, 2, 3, 4],
  ]) {
    const received = await new Promise<unknown[]>((resolve) =>
      process.nextTick((...values: unknown[]) => resolve(values), ...args)
    );
    expect(received).toEqual(args);
  }
});
