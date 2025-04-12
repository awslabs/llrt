import { spawn, execFile } from "child_process";
import { platform } from "os";
const IS_WINDOWS = platform() === "win32";

describe("child_process.spawn", () => {
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
});

describe("child_process.execFile", () => {
  it("should execute a file and return stdout", (done) => {
    execFile("echo", ["Hello, World!"], (error, stdout, stderr) => {
      console.log({ error, stdout, stderr });
      try {
        expect(error).toBeNull();
        expect(stdout.trim()).toEqual("Hello, World!");
        expect(stderr).toEqual("");
        done();
      } catch (err) {
        done(err);
      }
    });
  });

  it("should execute in a different directory", (done) => {
    execFile("pwd", { cwd: "./tests" }, (error, stdout) => {
      try {
        expect(error).toBeNull();
        const dir = stdout.trim().split("/").at(-1);
        expect(dir).toEqual("tests");
        done();
      } catch (err) {
        done(err);
      }
    });
  });

  it("should handle errors from the executed file", (done) => {
    if (process.env._VIRTUAL_ENV) {
      return done(); // Skip in environments that allow non-existent commands
    }

    execFile("nonexistent-command", (error) => {
      try {
        expect(error).toBeTruthy();
        done();
      } catch (err) {
        done(err);
      }
    });
  });

  it("should return stderr output if there is an error", (done) => {
    execFile("ls", ["nonexistent-dir"], (error, stdout, stderr) => {
      try {
        expect(error).toBeTruthy();
        expect(stderr).toBeTruthy();
        done();
      } catch (err) {
        done(err);
      }
    });
  });

  it("should execute with shell option", (done) => {
    execFile("echo", ["Shell Execution"], { shell: true }, (error, stdout) => {
      try {
        expect(error).toBeNull();
        expect(stdout.trim()).toEqual("Shell Execution");
        done();
      } catch (err) {
        done(err);
      }
    });
  });

  it("should handle large stdout output", (done) => {
    const largeOutput = Array.from({ length: 10000 }, () => "line").join("\n");
    execFile("printf", [largeOutput], (error, stdout) => {
      try {
        expect(error).toBeNull();
        expect(stdout.length).toBeGreaterThan(1000);
        done();
      } catch (err) {
        done(err);
      }
    });
  });

  it("should handle execution with env vars", (done) => {
    const env = { ...process.env, CUSTOM_VAR: "hello" };
    const script = IS_WINDOWS ? "echo %CUSTOM_VAR%" : "echo $CUSTOM_VAR";
    execFile(script, { shell: true, env }, (error, stdout) => {
      try {
        expect(error).toBeNull();
        expect(stdout.trim()).toEqual("hello");
        done();
      } catch (err) {
        done(err);
      }
    });
  });
});
