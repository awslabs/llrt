import net from "node:net";
import { platform } from "node:os";
import { spawnCapture } from "./test-utils";
import { gzipSync, deflateSync, brotliCompressSync } from "node:zlib";

const IS_WINDOWS = platform() === "win32";

let server: net.Server;
let url: string;
let gzipUrl: string;
let deflateUrl: string;
let brotliUrl: string;
let echoUrl: string;

const { LLRT_LOG, ...TEST_ENV } = process.env;

const spawnAndCollectOutput = async (
  deniedUrl: URL,
  env: Record<string, string>
) => {
  const { stdout, stderr } = await spawnCapture(
    process.argv[0],
    [
      "-e",
      `fetch("${deniedUrl}").catch(console.error).then(() => fetch("${url}")).then(() => console.log("OK"))`,
    ],
    { env: { ...TEST_ENV, ...env } }
  );
  return { stdout, stderr };
};

const TEST_TEXT = "Hello, World! This is compressed content for testing.";

beforeAll((done) => {
  server = net.createServer((socket) => {
    socket.on("error", () => {}); //ignore errors as abort signals might cancel the socket
    socket.on("data", (data) => {
      const request = data.toString();
      const getMatch = request.match(/^GET\s+(\S+)/);
      const postMatch = request.match(/^POST\s+(\S+)/);
      const path = getMatch ? getMatch[1] : postMatch ? postMatch[1] : "/";

      // Handle POST /echo - echo back the request body
      if (postMatch && path === "/echo") {
        // Parse Content-Length header
        const contentLengthMatch = request.match(/Content-Length:\s*(\d+)/i);
        const contentLength = contentLengthMatch
          ? parseInt(contentLengthMatch[1], 10)
          : 0;

        // Find the body (after double CRLF)
        const bodyStart = request.indexOf("\r\n\r\n");
        const body =
          bodyStart >= 0
            ? request.slice(bodyStart + 4, bodyStart + 4 + contentLength)
            : "";

        socket.write(
          `HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\nContent-Length: ${body.length}\r\n\r\n${body}`
        );
        socket.end();
        return;
      }

      if (path === "/gzip") {
        const compressed = gzipSync(Buffer.from(TEST_TEXT));
        socket.write(
          `HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\nContent-Encoding: gzip\r\nContent-Length: ${compressed.length}\r\n\r\n`
        );
        socket.write(compressed);
        socket.end();
      } else if (path === "/deflate") {
        const compressed = deflateSync(Buffer.from(TEST_TEXT));
        socket.write(
          `HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\nContent-Encoding: deflate\r\nContent-Length: ${compressed.length}\r\n\r\n`
        );
        socket.write(compressed);
        socket.end();
      } else if (path === "/brotli") {
        const compressed = brotliCompressSync(Buffer.from(TEST_TEXT));
        socket.write(
          `HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\nContent-Encoding: br\r\nContent-Length: ${compressed.length}\r\n\r\n`
        );
        socket.write(compressed);
        socket.end();
      } else {
        socket.write(
          "HTTP/1.1 200 OK\r\nContent-Type: text/html\r\n\r\n<html></html>"
        );
        socket.end();
      }
    });
  });

  server.listen(() => {
    const { address, port } = server.address()! as any as net.AddressInfo;
    const baseUrl = `http://${IS_WINDOWS ? "localhost" : address}:${port}`;
    url = baseUrl;
    gzipUrl = `${baseUrl}/gzip`;
    deflateUrl = `${baseUrl}/deflate`;
    brotliUrl = `${baseUrl}/brotli`;
    echoUrl = `${baseUrl}/echo`;
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

  it("is not allowed to fetch", async () => {
    const deniedUrl = new URL("https://www.amazon.com");
    const { stdout, stderr } = await spawnAndCollectOutput(deniedUrl, {
      LLRT_NET_DENY: "amazon.com",
    });

    expect(stderr.trim()).toEqual(`Error: URL denied: ${deniedUrl.hostname}`);
    expect(stdout.trim()).toEqual("OK");
  });

  it("is only allowed to fetch", async () => {
    const deniedUrl = new URL("https://www.amazon.com");
    const { stdout, stderr } = await spawnAndCollectOutput(deniedUrl, {
      LLRT_NET_ALLOW: url,
    });

    expect(stderr.trim()).toEqual(
      `Error: URL not allowed: ${deniedUrl.hostname}`
    );
    expect(stdout.trim()).toEqual("OK");
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

  describe("compressed response streaming", () => {
    it("should decompress gzip response via text()", async () => {
      const res = await fetch(gzipUrl);
      const text = await res.text();
      expect(text).toEqual(TEST_TEXT);
    });

    it("should decompress deflate response via text()", async () => {
      const res = await fetch(deflateUrl);
      const text = await res.text();
      expect(text).toEqual(TEST_TEXT);
    });

    it("should decompress brotli response via text()", async () => {
      const res = await fetch(brotliUrl);
      const text = await res.text();
      expect(text).toEqual(TEST_TEXT);
    });

    it("should decompress gzip response via body stream", async () => {
      const res = await fetch(gzipUrl);
      const reader = res.body!.getReader();
      const chunks: Uint8Array[] = [];
      while (true) {
        const { done, value } = await reader.read();
        if (done) break;
        chunks.push(value);
      }
      const decoder = new TextDecoder();
      const result = chunks.map((chunk) => decoder.decode(chunk)).join("");
      expect(result).toEqual(TEST_TEXT);
    });

    it("should decompress deflate response via body stream", async () => {
      const res = await fetch(deflateUrl);
      const reader = res.body!.getReader();
      const chunks: Uint8Array[] = [];
      while (true) {
        const { done, value } = await reader.read();
        if (done) break;
        chunks.push(value);
      }
      const decoder = new TextDecoder();
      const result = chunks.map((chunk) => decoder.decode(chunk)).join("");
      expect(result).toEqual(TEST_TEXT);
    });

    it("should decompress brotli response via body stream", async () => {
      const res = await fetch(brotliUrl);
      const reader = res.body!.getReader();
      const chunks: Uint8Array[] = [];
      while (true) {
        const { done, value } = await reader.read();
        if (done) break;
        chunks.push(value);
      }
      const decoder = new TextDecoder();
      const result = chunks.map((chunk) => decoder.decode(chunk)).join("");
      expect(result).toEqual(TEST_TEXT);
    });
  });

  describe("fetch with ReadableStream body", () => {
    it("should send ReadableStream body with fetch", async () => {
      const bodyText = "Hello from ReadableStream!";
      const stream = new ReadableStream({
        start(controller) {
          controller.enqueue(new TextEncoder().encode(bodyText));
          controller.close();
        },
      });

      const res = await fetch(echoUrl, {
        method: "POST",
        body: stream,
        headers: {
          "Content-Type": "text/plain",
        },
      });

      const result = await res.text();
      expect(result).toEqual(bodyText);
    });

    it("should send ReadableStream body with multiple chunks", async () => {
      const chunks = ["chunk1", "chunk2", "chunk3"];
      const stream = new ReadableStream({
        start(controller) {
          for (const chunk of chunks) {
            controller.enqueue(new TextEncoder().encode(chunk));
          }
          controller.close();
        },
      });

      const res = await fetch(echoUrl, {
        method: "POST",
        body: stream,
        headers: {
          "Content-Type": "text/plain",
        },
      });

      const result = await res.text();
      expect(result).toEqual(chunks.join(""));
    });

    it("should send ReadableStream body created from Request", async () => {
      const bodyText = "Request with stream body";
      const stream = new ReadableStream({
        start(controller) {
          controller.enqueue(new TextEncoder().encode(bodyText));
          controller.close();
        },
      });

      const request = new Request(echoUrl, {
        method: "POST",
        body: stream,
        headers: {
          "Content-Type": "text/plain",
        },
      });

      const res = await fetch(request);
      const result = await res.text();
      expect(result).toEqual(bodyText);
    });

    it("should handle async ReadableStream body", async () => {
      const bodyText = "Async stream body";
      const stream = new ReadableStream({
        async start(controller) {
          // Simulate async chunk delivery
          await new Promise((resolve) => setTimeout(resolve, 10));
          controller.enqueue(new TextEncoder().encode(bodyText));
          controller.close();
        },
      });

      const res = await fetch(echoUrl, {
        method: "POST",
        body: stream,
        headers: {
          "Content-Type": "text/plain",
        },
      });

      const result = await res.text();
      expect(result).toEqual(bodyText);
    });
  });
});
