import net from "node:net";
import zlib from "node:zlib";
import { platform } from "node:os";

const PAYLOAD = "llrt decompress ".repeat(256);
const COMPRESSED = zlib.brotliCompressSync(Buffer.from(PAYLOAD));

let server: net.Server;
let origin: string;

beforeAll((done) => {
  server = net.createServer((socket) => {
    let data = "";
    socket.on("error", () => {});
    socket.on("data", (chunk) => {
      data += chunk.toString();
      if (!data.includes("\r\n\r\n")) return;
      const path = data.split(" ", 2)[1];
      if (path === "/br") {
        socket.write(
          "HTTP/1.1 200 OK\r\nContent-Type: application/octet-stream\r\n" +
            `Content-Encoding: br\r\nContent-Length: ${COMPRESSED.length}\r\n\r\n`
        );
        socket.write(COMPRESSED);
      } else {
        const sent =
          data.match(/\r\naccept-encoding: ([^\r\n]*)/i)?.[1] ?? "<none>";
        socket.write(
          "HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\n" +
            `Content-Length: ${sent.length}\r\n\r\n${sent}`
        );
      }
      socket.end();
    });
  });
  server.listen(() => {
    const { address, port } = server.address() as net.AddressInfo;
    const host = platform() === "win32" ? "localhost" : address;
    origin = `http://${host}:${port}`;
    done();
  });
});

afterAll(() => {
  server?.close();
});

const isCompressed = (bytes: ArrayBuffer | Uint8Array) => {
  const view = new Uint8Array(bytes);
  return (
    view.length === COMPRESSED.length &&
    view.every((byte, i) => byte === COMPRESSED[i])
  );
};

describe("fetch decompress option", () => {
  it("decodes Content-Encoding by default", async () => {
    const res = await fetch(`${origin}/br`);
    expect(await res.text()).toBe(PAYLOAD);
  });

  it("returns the body as sent when decompress is false", async () => {
    const res = await fetch(`${origin}/br`, { decompress: false });
    expect(res.headers.get("content-encoding")).toBe("br");
    expect(res.headers.get("content-length")).toBe(`${COMPRESSED.length}`);
    expect(isCompressed(await res.arrayBuffer())).toBe(true);
  });

  it("streams the body as sent when decompress is false", async () => {
    const res = await fetch(`${origin}/br`, { decompress: false });
    const reader = res.body!.getReader();
    const chunks: Uint8Array[] = [];
    for (let r = await reader.read(); !r.done; r = await reader.read()) {
      chunks.push(r.value);
    }
    expect(isCompressed(Buffer.concat(chunks))).toBe(true);
  });

  it("applies decompress from the init when given a Request", async () => {
    // The AWS SDK's fetch handler calls fetch(new Request(...), init).
    const res = await fetch(new Request(`${origin}/br`), {
      decompress: false,
    });
    expect(isCompressed(await res.arrayBuffer())).toBe(true);
  });

  it("ignores decompress set on a Request, as Bun does", async () => {
    const req = new Request(`${origin}/br`, { decompress: false } as any);
    const res = await fetch(req);
    expect(await res.text()).toBe(PAYLOAD);
  });

  it("sends the default Accept-Encoding only when decompressing", async () => {
    const res = await fetch(`${origin}/echo`);
    expect(await res.text()).toBe("zstd, br, gzip, deflate");

    const raw = await fetch(`${origin}/echo`, { decompress: false });
    expect(await raw.text()).toBe("<none>");
  });

  it("keeps an explicit Accept-Encoding when decompress is false", async () => {
    const res = await fetch(`${origin}/echo`, {
      decompress: false,
      headers: { "accept-encoding": "gzip" },
    });
    expect(await res.text()).toBe("gzip");
  });
});
