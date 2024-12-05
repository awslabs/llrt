import net from "net";
import { spawn } from "child_process";
import { platform } from "os";
const IS_WINDOWS = platform() === "win32";

let server: net.Server;
let url: string;

beforeAll((done) => {
  server = net.createServer((socket) => {
    socket.on("error", () => {}); //ignore errors as abort signals might cancel the socket
    socket.on("data", () => {
      socket.write(
        "HTTP/1.1 200 OK\r\nContent-Type: text/html\r\n\r\n<html></html>"
      );
      socket.end();
    });
  });

  server.listen(() => {
    const { address, port } = server.address()! as any as net.AddressInfo;
    url = `http://${IS_WINDOWS ? "localhost" : address}:${port}`;
    done();
  });
});

afterAll(() => {
  server?.close();
});

describe("fetch", () => {
  it("should fetch a website", async () => {
    const res = await fetch(url);

    expect(res.status).toEqual(200);
    expect(
      res.headers.get("content-type")?.startsWith("text/html")
    ).toBeTruthy();
  });

  it("should fetch a website with url and options", async () => {
    const options = {
      method: "GET",
      url,
    };

    const request = new Request(url);

    const res = await fetch(request, options);
    expect(res.status).toEqual(200);
    expect(
      res.headers.get("content-type")?.startsWith("text/html")
    ).toBeTruthy();
  });

  it("should fetch a website with different resource options", async () => {
    let res = await fetch(new Request(url));
    expect(res.status).toEqual(200);
    expect(
      res.headers.get("content-type")?.startsWith("text/html")
    ).toBeTruthy();

    res = await fetch(new URL(url));
    expect(res.status).toEqual(200);
    expect(
      res.headers.get("content-type")?.startsWith("text/html")
    ).toBeTruthy();

    res = await fetch("", { url } as any);
    expect(res.status).toEqual(200);
    expect(
      res.headers.get("content-type")?.startsWith("text/html")
    ).toBeTruthy();
  });

  it("should fetch a website in parallel", async () => {
    await Promise.all(new Array(10).fill(0).map(() => fetch(url)));
  });

  it.skip("is not allowed to fetch", (done) => {
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
      expect(stderr.trim()).toEqual(`Error: URL denied: ${deniedUrl.hostname}`);
      expect(stdout.trim()).toEqual("OK");
      done();
    });
    proc.on("error", done);
  });

  it.skip("is only allowed to fetch", (done) => {
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
      expect(stderr.trim()).toEqual(
        `Error: URL not allowed: ${deniedUrl.hostname}`
      );
      expect(stdout.trim()).toEqual("OK");
      done();
    });
    proc.on("error", done);
  });

  it("should be abortable using signals", async () => {
    const abortController = new AbortController();
    const res = fetch(url, { signal: abortController.signal });
    abortController.abort();
    try {
      await res;
    } catch (err: any) {
      expect(err.name).toBe("AbortError");
    }
  });
  it("should be abortable using request signal", async () => {
    const abortController = new AbortController();
    const req = new Request(url, { signal: abortController.signal });
    abortController.abort("aborted");
    try {
      await fetch(req);
    } catch (err: any) {
      expect(abortController.signal.reason).toBe("aborted");
    }
  });
  it("should be processing data-url", async () => {
    const s = "hello";
    const base64 = Buffer.from(s).toString("base64");
    const dataURIPrefix = "data:application/octet-stream;base64,";
    const url = dataURIPrefix + base64;
    const resp = await fetch(url);
    const buf = await resp.arrayBuffer();
    const str = Buffer.from(buf).toString("ascii");
    expect(str).toEqual(s);
  });
});
