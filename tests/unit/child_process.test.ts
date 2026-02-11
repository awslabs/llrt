import defaultImport from "node:child_process";
import legacyImport from "child_process";

import { platform } from "node:os";
import process from "node:process";
const IS_WINDOWS = platform() === "win32";

it("node:child_process should be the same as child_process", () => {
  expect(defaultImport).toStrictEqual(legacyImport);
});

const { spawn, exec, execFile } = defaultImport;

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
    const child = IS_WINDOWS
      ? spawn("ping", ["-n", "999", "localhost"])
      : spawn("sleep", ["999"]);

    child.on("exit", (code, signal) => {
      try {
        if (IS_WINDOWS) {
          // LLRT on Windows returns exit code 1 when killed
          expect(code).toEqual(1);
        } else {
          expect(code).toBeNull();
        }
        expect(signal).toEqual("SIGKILL");
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

  // Skip on Windows - detached process behavior differs significantly
  (IS_WINDOWS ? it.skip : it)(
    "should handle detached child process termination",
    (done) => {
      const sleepCmd = "spawn('sleep', ['999']";

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
    }
  );
});

describe("exec", () => {
  it("should execute a shell command and return stdout", (done) => {
    exec("echo Hello", (error, stdout, stderr) => {
      try {
        expect(error).toBeNull();
        expect(stdout.trim()).toEqual("Hello");
        expect(stderr).toEqual("");
        done();
      } catch (err) {
        done(err);
      }
    });
  });

  it("should execute a command with options", (done) => {
    const cwd = IS_WINDOWS ? process.cwd() : "/tmp";
    const pwdCmd = IS_WINDOWS ? "cd" : "pwd";
    exec(pwdCmd, { cwd }, (error, stdout, stderr) => {
      try {
        expect(error).toBeNull();
        if (IS_WINDOWS) {
          expect(stdout.trim().toLowerCase()).toContain(
            process.cwd().toLowerCase()
          );
        } else {
          expect(stdout.trim()).toContain("/tmp");
        }
        expect(stderr).toEqual("");
        done();
      } catch (err) {
        done(err);
      }
    });
  });

  it("should capture stderr on command failure", (done) => {
    if (process.env._VIRTUAL_ENV) {
      // QEMU may handle errors differently
      return done();
    }
    // Use a command that writes to stderr and exits with error
    const cmd = IS_WINDOWS
      ? "cmd /c exit 1"
      : "sh -c 'echo error >&2 && exit 1'";
    exec(cmd, (error, stdout, stderr) => {
      try {
        expect(error).not.toBeNull();
        expect(error!.code).toEqual(1);
        done();
      } catch (err) {
        done(err);
      }
    });
  });

  it("should handle nonexistent commands", (done) => {
    if (process.env._VIRTUAL_ENV) {
      // QEMU may handle errors differently
      return done();
    }
    exec("nonexistent_command_12345", (error, stdout, stderr) => {
      try {
        expect(error).not.toBeNull();
        done();
      } catch (err) {
        done(err);
      }
    });
  });

  it("should respect maxBuffer option", (done) => {
    // Generate output larger than maxBuffer
    const cmd = IS_WINDOWS
      ? 'cmd /c "echo AAAAAAAAAA"'
      : "echo AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA";
    exec(cmd, { maxBuffer: 10 }, (error, stdout, stderr) => {
      try {
        expect(error).not.toBeNull();
        expect(error!.message).toContain("maxBuffer");
        done();
      } catch (err) {
        done(err);
      }
    });
  });
});

describe("execFile", () => {
  it("should execute a file and return stdout", (done) => {
    execFile("echo", ["Hello", "World"], (error, stdout, stderr) => {
      try {
        expect(error).toBeNull();
        expect(stdout.trim()).toEqual("Hello World");
        expect(stderr).toEqual("");
        done();
      } catch (err) {
        done(err);
      }
    });
  });

  it("should execute a file without arguments", (done) => {
    const cmd = IS_WINDOWS ? "cmd" : "pwd";
    const args = IS_WINDOWS ? ["/c", "cd"] : [];
    execFile(cmd, args, (error, stdout, stderr) => {
      try {
        expect(error).toBeNull();
        expect(stdout.length).toBeGreaterThan(0);
        expect(stderr).toEqual("");
        done();
      } catch (err) {
        done(err);
      }
    });
  });

  it("should execute with options", (done) => {
    const cwd = IS_WINDOWS ? process.cwd() : "/tmp";
    const cmd = IS_WINDOWS ? "cmd" : "pwd";
    const args = IS_WINDOWS ? ["/c", "cd"] : [];
    execFile(cmd, args, { cwd }, (error, stdout, stderr) => {
      try {
        expect(error).toBeNull();
        if (IS_WINDOWS) {
          expect(stdout.trim().toLowerCase()).toContain(
            process.cwd().toLowerCase()
          );
        } else {
          expect(stdout.trim()).toContain("/tmp");
        }
        done();
      } catch (err) {
        done(err);
      }
    });
  });

  it("should handle nonexistent executable", (done) => {
    if (process.env._VIRTUAL_ENV) {
      // QEMU may handle errors differently
      return done();
    }
    execFile("nonexistent_command_12345", [], (error, stdout, stderr) => {
      try {
        expect(error).not.toBeNull();
        done();
      } catch (err) {
        done(err);
      }
    });
  });

  it("should handle executable failure with exit code", (done) => {
    if (process.env._VIRTUAL_ENV) {
      // QEMU may handle errors differently
      return done();
    }
    const cmd = IS_WINDOWS ? "cmd" : "sh";
    const args = IS_WINDOWS ? ["/c", "exit 42"] : ["-c", "exit 42"];
    execFile(cmd, args, (error, stdout, stderr) => {
      try {
        expect(error).not.toBeNull();
        expect(error!.code).toEqual(42);
        done();
      } catch (err) {
        done(err);
      }
    });
  });

  it("should respect maxBuffer option", (done) => {
    execFile(
      "echo",
      ["AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA"],
      { maxBuffer: 10 },
      (error, stdout, stderr) => {
        try {
          expect(error).not.toBeNull();
          expect(error!.message).toContain("maxBuffer");
          done();
        } catch (err) {
          done(err);
        }
      }
    );
  });

  it("should execute with shell option", (done) => {
    // When shell is true, execFile should use shell to execute
    const cmd = IS_WINDOWS ? "echo %PATH%" : "echo $PATH";
    execFile(cmd, [], { shell: true }, (error, stdout, stderr) => {
      try {
        expect(error).toBeNull();
        expect(stdout.length).toBeGreaterThan(0);
        done();
      } catch (err) {
        done(err);
      }
    });
  });
});
