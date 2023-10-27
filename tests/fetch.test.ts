import net from "net";
import { spawn } from "child_process";

let server: net.Server;
let url: string;

beforeAll((done) => {
  server = net.createServer((socket) => {
    socket.on("data", () => {
      socket.write(
        "HTTP/1.1 200 OK\r\nContent-Type: text/html\r\n\r\n<html></html>"
      );
      socket.end();
    });
  });

  server.listen(() => {
    let addressInfo = server.address()! as any as net.AddressInfo;
    url = `http://${addressInfo.address}:${addressInfo.port}`;
    done();
  });
});

afterAll(() => {
  server?.close();
});

describe("fetch", () => {
  it("should fetch a website", async () => {
    const res = await fetch(url);

    assert.equal(res.status, 200);
    assert.ok(res.headers.get("content-type")?.startsWith("text/html"));
  });

  it("should fetch a website in parallel", async () => {
    await Promise.all(new Array(10).fill(0).map(() => fetch(url)));
  });

  it("is not allowed to fetch", (done) => {
    let deniedUrl = new URL("https://www.amazon.com");

    let proc = spawn(
      process.argv[0],
      [
        "-e",
        `fetch("${deniedUrl}").catch(console.error).then(() => fetch("${url}")).then(() => console.log("OK"))`,
      ],
      {
        env: {
          LLRT_NET_DENY: "amazon.com",
        },
      }
    );
    let stdout = "";
    let stderr = "";
    proc.stderr.on("data", (data) => {
      stderr += data.toString();
    });
    proc.stdout.on("data", (data) => {
      stdout += data.toString();
    });
    proc.on("close", () => {
      assert.equal(stderr.trim(), `Error: URL denied: ${deniedUrl.hostname}`);
      assert.equal(stdout.trim(), "OK");
      done();
    });
    proc.on("error", done);
  });

  it("is only allowed to fetch", (done) => {
    let deniedUrl = new URL("https://www.amazon.com");

    let proc = spawn(
      process.argv[0],
      [
        "-e",
        `fetch("${deniedUrl}").catch(console.error).then(() => fetch("${url}")).then(() => console.log("OK"))`,
      ],
      {
        env: {
          LLRT_NET_ALLOW: url.toString(),
        },
      }
    );
    let stdout = "";
    let stderr = "";
    proc.stderr.on("data", (data) => {
      stderr += data.toString();
    });
    proc.stdout.on("data", (data) => {
      stdout += data.toString();
    });
    proc.on("close", () => {
      assert.equal(
        stderr.trim(),
        `Error: URL not allowed: ${deniedUrl.hostname}`
      );
      assert.equal(stdout.trim(), "OK");
      done();
    });
    proc.on("error", done);
  });
});
