import net from "node:net";
import { spawnCapture } from "./test-utils";

let server: net.Server;
let port: number;
let connections = 0;

beforeAll(async () => {
  server = net.createServer((socket) => {
    connections += 1;
    socket.on("error", () => {});
    socket.end();
  });
  await new Promise<void>((resolve) => server.listen(0, "127.0.0.1", resolve));
  port = (server.address() as net.AddressInfo).port;
});

afterAll(() => new Promise<void>((resolve) => server.close(() => resolve())));

function cleanEnv() {
  const { LLRT_LOG, LLRT_NET_ALLOW, LLRT_NET_DENY, ...env } = process.env;
  return env;
}

async function runScript(source: string, policy: Record<string, string>) {
  return spawnCapture(process.argv[0], ["-e", source], {
    env: { ...cleanEnv(), ...policy },
  });
}

function tcpScript(host: string, target: number) {
  return `
const net = require("node:net");
try {
  const socket = net.connect({ host: ${JSON.stringify(host)}, port: ${target} });
  socket.on("connect", () => { console.log("CONNECTED"); socket.destroy(); });
  socket.on("error", (e) => console.log("ERROR " + e.message));
} catch (e) {
  console.log("ERROR " + e.message);
}
`;
}

function udpScript(address: string, target: number) {
  return `
const dgram = require("node:dgram");
const socket = dgram.createSocket("udp4");
try {
  socket.send("ping", ${target}, ${JSON.stringify(address)}, (err) => {
    console.log(err ? "ERROR " + err.message : "SENT");
    socket.close();
  });
} catch (e) {
  console.log("ERROR " + e.message);
}
`;
}

describe("net network policy", () => {
  it("denies a bare-host entry for TCP sockets", async () => {
    const result = await runScript(tcpScript("127.0.0.1", port), {
      LLRT_NET_DENY: "127.0.0.1",
    });
    expect(result.stdout.trim()).toBe(
      `ERROR Network address denied: 127.0.0.1:${port}`
    );
    expect(connections).toBe(0);
  });

  it("denies a port entry for TCP sockets", async () => {
    const result = await runScript(tcpScript("127.0.0.1", port), {
      LLRT_NET_DENY: `127.0.0.1:${port}`,
    });
    expect(result.stdout.trim()).toBe(
      `ERROR Network address denied: 127.0.0.1:${port}`
    );
  });

  it("allows a bare-host entry for TCP sockets", async () => {
    const result = await runScript(tcpScript("127.0.0.1", port), {
      LLRT_NET_ALLOW: "127.0.0.1",
    });
    expect(result.stdout.trim()).toBe("CONNECTED");
  });

  it("allows a matching port entry for TCP sockets", async () => {
    const result = await runScript(tcpScript("127.0.0.1", port), {
      LLRT_NET_ALLOW: `127.0.0.1:${port}`,
    });
    expect(result.stdout.trim()).toBe("CONNECTED");
  });

  it("denies a different port through the allow list", async () => {
    const before = connections;
    const result = await runScript(tcpScript("127.0.0.1", port), {
      LLRT_NET_ALLOW: `127.0.0.1:${port + 1}`,
    });
    expect(result.stdout.trim()).toBe(
      `ERROR Network address not allowed: 127.0.0.1:${port}`
    );
    expect(connections).toBe(before);
  });

  for (const alias of ["127.1", "0x7f000001", "2130706433"]) {
    it(`canonicalizes the ${alias} entry`, async () => {
      const before = connections;
      const result = await runScript(tcpScript("127.0.0.1", port), {
        LLRT_NET_DENY: alias,
      });
      expect(result.stdout.trim()).toBe(
        `ERROR Network address denied: 127.0.0.1:${port}`
      );
      expect(connections).toBe(before);
    });
  }

  it("matches a lowercase host entry case-insensitively", async () => {
    const before = connections;
    const result = await runScript(tcpScript("LOCALHOST", port), {
      LLRT_NET_DENY: "localhost",
    });
    expect(result.stdout.trim()).toBe(
      `ERROR Network address denied: LOCALHOST:${port}`
    );
    expect(connections).toBe(before);
  });

  it("accepts a bare IPv6 entry and matches it", async () => {
    const result = await runScript(tcpScript("::1", 9), {
      LLRT_NET_DENY: "::1",
    });
    expect(result.code).toBe(0);
    expect(result.stdout.trim()).toBe("ERROR Network address denied: [::1]:9");
  });

  it("accepts a bracketed IPv6 entry with a port and matches it", async () => {
    const result = await runScript(tcpScript("::1", 9), {
      LLRT_NET_DENY: "[::1]:9",
    });
    expect(result.code).toBe(0);
    expect(result.stdout.trim()).toBe("ERROR Network address denied: [::1]:9");
  });

  it("accepts a unix socket path entry without aborting start-up", async () => {
    const result = await runScript(`console.log("BOOTED")`, {
      LLRT_NET_DENY: "/var/db/app.sock",
    });
    expect(result.code).toBe(0);
    expect(result.stdout.trim()).toBe("BOOTED");
  });

  it("reports invalid entries at start-up", async () => {
    const result = await runScript(`console.log("BOOTED")`, {
      LLRT_NET_DENY: "example.com:not-a-port",
    });
    expect(result.code).not.toBe(0);
    expect(result.stderr).toContain(
      'LLRT_NET_DENY: invalid network policy entry "example.com:not-a-port"'
    );
  });
});

describe("dgram network policy", () => {
  it("denies a bare-host entry for UDP sends", async () => {
    const result = await runScript(udpScript("127.0.0.1", port), {
      LLRT_NET_DENY: "127.0.0.1",
    });
    expect(result.stdout.trim()).toBe(
      `ERROR Network address denied: 127.0.0.1:${port}`
    );
  });

  it("denies a port entry for UDP sends", async () => {
    const result = await runScript(udpScript("127.0.0.1", port), {
      LLRT_NET_DENY: `127.0.0.1:${port}`,
    });
    expect(result.stdout.trim()).toBe(
      `ERROR Network address denied: 127.0.0.1:${port}`
    );
  });

  it("allows a bare-host entry for UDP sends", async () => {
    const result = await runScript(udpScript("127.0.0.1", port), {
      LLRT_NET_ALLOW: "127.0.0.1",
    });
    expect(result.stdout.trim()).toBe("SENT");
  });

  it("denies a different port through the allow list", async () => {
    const result = await runScript(udpScript("127.0.0.1", port), {
      LLRT_NET_ALLOW: `127.0.0.1:${port + 1}`,
    });
    expect(result.stdout.trim()).toBe(
      `ERROR Network address not allowed: 127.0.0.1:${port}`
    );
  });
});
