import defaultImport from "node:child_process";
import legacyImport from "child_process";

import { platform } from "node:os";
import process from "node:process";
import { parse } from "node:path";
const IS_WINDOWS = platform() === "win32";

it("node:child_process should be the same as child_process", () => {
  expect(defaultImport).toStrictEqual(legacyImport);
});

const { spawn } = defaultImport;

describe("spawn", () => {
  it("should spawn a child process", (done) => {
    const command = "ls";
    const args = ["-l"];
    const child = spawn(command, args);
    child.on("exit", (code) => {
      try {
        expect(code).toEqual(0);
        done();
      } catch (error) {
        done(error);
      }
    });
  });
  it("should spawn in a diffrent directory", (done) => {
    const child = spawn("pwd", {
      cwd: "./tests",
    });
    let output = "";
    child.stdout.on("data", (data) => {
      output += data.toString();
    });

    child.on("close", (code) => {
      try {
        const dir = output.trim().split("/").at(-1);
        expect(dir).toEqual("tests");
        expect(code).toEqual(0);
        done();
      } catch (error) {
        done(error);
      }
    });
  });
  it("should capture output from the child process", (done) => {
    const command = "echo";
    const args = ["Hello, World!"];
    const child = spawn(command, args);
    let output = "";
    child.stdout.on("data", (data) => {
      output += data.toString();
    });

    child.on("close", (code) => {
      try {
        expect(output.trim()).toEqual(args[0]);
        expect(code).toEqual(0);
        done();
      } catch (error) {
        done(error);
      }
    });
  });

  it("should send input to the child process", (done) => {
    const command = "cat";
    const input = "Hello, world!";
    const child = spawn(command);

    child.stdin.write(input);
    child.stdin.end();

    let output = "";
    child.stdout.on("data", (data) => {
      output += data.toString();
    });

    child.on("close", (code) => {
      try {
        expect(code).toEqual(0);
        expect(output.trim()).toEqual(input);
        done();
      } catch (error) {
        done(error);
      }
    });
  });

  it("should handle errors from the child process", (done) => {
    if (process.env._VIRTUAL_ENV) {
      //QEMU spawns nonexistent-command successfully
      return done();
    }
    const command = "nonexistent-command";
    const child = spawn(command);
    child.on("error", (err) => {
      try {
        expect(err).toBeTruthy();
        done();
      } catch (error) {
        done(error);
      }
    });
  });

  it("should handle child process termination", (done) => {
    const command = "sleep 999";
    const child = spawn(command, { shell: true });

    child.on("exit", (code, signal) => {
      try {
        if (!IS_WINDOWS) {
          expect(code).toEqual(0);
        }

        expect(signal).toEqual(IS_WINDOWS ? "SIGKILL" : "SIGINT");
        done();
      } catch (error) {
        done(error);
      }
    });

    setTimeout(() => {
      child.kill("SIGINT");
    }, 50);
  });

  it("should handle child process stdio inherit", (done) => {
    const child = spawn("echo", ["123"], { stdio: "inherit" });
    child.on("exit", (code) => {
      try {
        expect(code).toEqual(0);
        done();
      } catch (error) {
        done(error);
      }
    });
  });
  it("should handle child process stdio ignore", (done) => {
    const child = spawn("echo", ["123"], { stdio: "ignore" });
    child.on("exit", (code) => {
      try {
        expect(code).toEqual(0);
        done();
      } catch (error) {
        done(error);
      }
    });
  });

  it("should have a process exitCode", async () => {
    const testExitCode = async (
      exitCodeValue: number | string,
      expectedCode: number
    ) => {
      const proc = spawn(process.argv0, [
        "-e",
        `process.exitCode = ${exitCodeValue}`,
      ]);
      await new Promise<void>((resolve) => {
        proc.on("exit", (code) => {
          expect(code).toEqual(expectedCode);
          resolve();
        });
      });
    };

    await testExitCode(241212341, 181);
    await testExitCode(-1, 255);
    await testExitCode(1, 1);
    await testExitCode(-1231231231, 1);
    await testExitCode(266, 10);
    await testExitCode("266", 10);
  });

  it("should handle detached child process termination", (done) => {
    const parentProc = spawn(process.argv0, [
      "-e",
      `
        const child = require('child_process').spawn('sleep', ['999'], {
          detached: true,
          stdio: 'ignore'
        });
        console.log(child.pid.toString());
      `,
    ]);

    let detachedPidString = "";
    parentProc.stdout.on("data", (data) => {
      console.log("DATA", data.toString());
      detachedPidString += data.toString();
      parentProc.kill();
    });

    parentProc.on("exit", () => {
      console.log("aaa", detachedPidString);
      try {
        const detachedPid = parseInt(detachedPidString.trim());
        expect(detachedPid).toBeGreaterThan(0);
        expect(process.kill(detachedPid, 0)).toBe(true);
        process.kill(detachedPid);
        done();
      } catch (error) {
        done(error);
      }
    });
  });
});
