import defaultImport from "node:child_process";
import legacyImport from "child_process";

import { platform } from "node:os";
import process from "node:process";
const IS_WINDOWS = platform() === "win32";

it("node:child_process should be the same as child_process", () => {
  expect(defaultImport).toStrictEqual(legacyImport);
});

const { spawn } = defaultImport;

describe("spawn", () => {
  it("should spawn a child process", (done) => {
    // Use cross-platform commands
    const command = IS_WINDOWS ? "cmd" : "ls";
    const args = IS_WINDOWS ? ["/c", "dir"] : ["-l"];
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
    // Use cross-platform commands for getting current directory
    const child = IS_WINDOWS
      ? spawn("cmd", ["/c", "cd"], { cwd: "./tests" })
      : spawn("pwd", { cwd: "./tests" });
    let output = "";
    child.stdout.on("data", (data) => {
      output += data.toString();
    });

    child.on("close", (code) => {
      try {
        // Split by either / or \ for cross-platform compatibility
        const dir = output.trim().split(/[/\\]/).at(-1);
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
    // Use cross-platform approach: Windows uses 'findstr .*' to echo all input, Unix uses 'cat'
    const command = IS_WINDOWS ? "findstr" : "cat";
    const args = IS_WINDOWS ? [".*"] : [];
    const input = "Hello, world!";
    const child = spawn(command, args);

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
    // Use cross-platform long-running command: Windows uses 'ping -n 999 localhost', Unix uses 'sleep 999'
    const command = IS_WINDOWS ? "ping -n 999 localhost" : "sleep 999";
    const child = spawn(command);

    child.on("exit", (code, signal) => {
      try {
        if (IS_WINDOWS) {
          // Windows terminates with code 1 when killed
          expect(code).toEqual(1);
          expect(signal).toBeNull();
        } else {
          expect(code).toEqual(0);
          expect(signal).toEqual("SIGKILL");
        }
        done();
      } catch (error) {
        done(error);
      }
    });

    setTimeout(() => {
      child.kill("SIGKILL"); //SIGINT does not forward to children on Linux
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
    // Use cross-platform long-running command
    const sleepCmd = IS_WINDOWS
      ? "spawn('ping', ['-n', '999', 'localhost']"
      : "spawn('sleep', ['999']";

    const parentProc = spawn(process.argv0, [
      "-e",
      `
        import {spawn} from "child_process";
        const child = ${sleepCmd}, {
          detached: true,
          stdio: 'ignore'
        });
        console.log(child.pid.toString());
      `,
    ]);

    let detachedPidString = "";
    parentProc.stdout.on("data", (data) => {
      detachedPidString += data.toString();
      // Kill parent once we have the PID - parent would otherwise wait for detached child
      parentProc.kill();
    });

    parentProc.on("error", (err) => {
      done(err);
    });

    parentProc.on("close", () => {
      try {
        const detachedPid = parseInt(detachedPidString.trim());
        expect(detachedPid).toBeGreaterThan(0);
        // Verify detached process survived parent termination
        const exists = process.kill(detachedPid, 0);
        expect(exists).toBe(true);
        // Clean up the detached process
        process.kill(detachedPid, "SIGKILL");
        done();
      } catch (error) {
        done(error);
      }
    });
  });
});
