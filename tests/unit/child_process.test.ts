import { spawn } from "child_process";
import { platform } from "os";
const IS_WIN = platform() === "win32";

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
    const command = "sleep 4; echo 123";
    const child = spawn(command, { shell: true });

    child.on("exit", (code, signal) => {
      try {
        if (!IS_WIN) {
          expect(code).toEqual(0);
        }

        expect(signal).toEqual(IS_WIN ? "SIGKILL" : "SIGINT");
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
