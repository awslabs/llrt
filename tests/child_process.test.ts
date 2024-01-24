import { spawn } from "child_process";

describe("child_process.spawn", () => {
  it("should spawn a child process", (done) => {
    const command = "ls";
    const args = ["-l"];
    const child = spawn(command, args);
    child.on("exit", (code) => {
      try {
        assert.strictEqual(code, 0);
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
        assert.strictEqual(output.trim(), args[0]);
        assert.strictEqual(code, 0);
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
        assert.strictEqual(code, 0);
        assert.strictEqual(output.trim(), input);
        done();
      } catch (error) {
        done(error);
      }
    });
  });

  it.only("should handle errors from the child process", (done) => {
    const command = "nonexistent-command";
    const child = spawn(command);
    child.stderr.on("data", (data) => {
      console.log("STDERR DATA:", data.toString());
    });
    child.stdout.on("data", (data) => {
      console.log("STDOUT DATA:", data.toString());
    });
    child.on("error", (err) => {
      try {
        assert.ok(err);
        done();
      } catch (error) {
        done(error);
      }
    });
  });

  it("should handle child process termination", (done) => {
    const command = "sleep 1; echo 123";
    const child = spawn(command, { shell: true });

    child.on("exit", (code, signal) => {
      try {
        assert.strictEqual(code, 0);
        assert.strictEqual(signal, "SIGINT");
        done();
      } catch (error) {
        done(error);
      }
    });

    setTimeout(() => {
      child.kill("SIGINT");
    }, 5);
  });

  it("should handle child process stdio inherit", (done) => {
    const child = spawn("echo", ["123"], { stdio: "inherit" });
    child.on("exit", (code) => {
      try {
        assert.strictEqual(code, 0);
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
        assert.strictEqual(code, 0);
        done();
      } catch (error) {
        done(error);
      }
    });
  });
});
