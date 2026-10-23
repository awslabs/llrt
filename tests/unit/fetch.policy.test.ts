import net from "node:net";
import { spawnCapture } from "./test-utils";

let server: net.Server;
let origin: string;
const requests: string[] = [];

beforeAll(async () => {
  server = net.createServer((socket) => {
    let data = "";
    let replied = false;
    socket.on("error", () => {});
    socket.on("data", (chunk) => {
      data += chunk.toString();
      if (replied || !data.includes("\r\n\r\n")) return;
      replied = true;
      const path = data.split("\r\n", 1)[0].split(" ")[1];
      requests.push(path);
      const body = JSON.stringify({ path });
      socket.write(
        [
          "HTTP/1.1 200 OK",
          "Connection: close",
          "Content-Type: application/json",
          `Content-Length: ${body.length}`,
          "",
          body,
        ].join("\r\n")
      );
      socket.end();
    });
  });
  await new Promise<void>((resolve) => server.listen(0, "127.0.0.1", resolve));
  origin = `http://127.0.0.1:${(server.address() as net.AddressInfo).port}`;
});

afterAll(() => new Promise<void>((resolve) => server.close(() => resolve())));

async function policyFetch(url: string, policy: Record<string, string>) {
  const { LLRT_LOG, LLRT_NET_ALLOW, LLRT_NET_DENY, ...env } = process.env;
  return spawnCapture(
    process.argv[0],
    [
      "-e",
      `fetch(${JSON.stringify(url)}).then(r => r.json()).then(r => console.log(r.path), e => console.log(e.message))`,
    ],
    { env: { ...env, ...policy } }
  );
}

describe("fetch network policy", () => {
  for (const explicitPort of [false, true]) {
    const url = `https://127.0.0.1${explicitPort ? ":443" : ""}/https-policy`;
    const host = `127.0.0.1${explicitPort ? ":443" : ""}`;
    for (const entry of ["127.0.0.1", "127.0.0.1:443"]) {
      it(`denies HTTPS ${host} with ${entry}`, async () => {
        const result = await policyFetch(url, { LLRT_NET_DENY: entry });
        expect(result.code).toBe(0);
        expect(result.stdout.trim()).toBe(`URL denied: ${host}`);
      });
      it(`allows HTTPS ${host} through the allow check with ${entry}`, async () => {
        // Reaching deny proves allow accepted the target, without a TLS listener.
        const result = await policyFetch(url, {
          LLRT_NET_ALLOW: entry,
          LLRT_NET_DENY: "127.0.0.1:443",
        });
        expect(result.code).toBe(0);
        expect(result.stdout.trim()).toBe(`URL denied: ${host}`);
      });
    }
    it(`rejects HTTPS ${host} with a port-80-only allow rule`, async () => {
      const result = await policyFetch(url, { LLRT_NET_ALLOW: "127.0.0.1:80" });
      expect(result.code).toBe(0);
      expect(result.stdout.trim()).toBe(`URL not allowed: ${host}`);
    });
  }

  for (const hostname of ["LOCALHOST", "127.1", "0x7f000001"]) {
    for (const restriction of ["LLRT_NET_ALLOW", "LLRT_NET_DENY"]) {
      it(`matches ${restriction} for ${hostname}`, async () => {
        const path = `/host-policy-${hostname}-${restriction}`;
        const policyOrigin = origin.replace("127.0.0.1", hostname);
        // Keep the connection address portable: Windows does not resolve all
        // legacy IPv4 spellings, even when the policy correctly accepts them.
        const requestOrigin = hostname === "LOCALHOST" ? policyOrigin : origin;
        const result = await policyFetch(requestOrigin + path, {
          [restriction]: policyOrigin,
        });
        expect(result.code).toBe(0);
        if (restriction === "LLRT_NET_ALLOW") {
          expect(result.stdout.trim()).toBe(path);
          expect(requests.includes(path)).toBe(true);
        } else {
          expect(result.stdout.trim()).toBe(
            `URL denied: ${requestOrigin.slice(7)}`
          );
          expect(requests.includes(path)).toBe(false);
        }
      });
    }
  }

  for (const hostname of ["127.1", "0x7f000001"]) {
    it(`matches a noncanonical request host ${hostname} before connecting`, async () => {
      const url = origin.replace("127.0.0.1", hostname) + "/legacy-ip-policy";
      const result = await policyFetch(url, {
        LLRT_NET_ALLOW: origin,
        LLRT_NET_DENY: origin,
      });
      expect(result.code).toBe(0);
      // Deny runs after allow, proving both matched without relying on DNS.
      expect(result.stdout.trim()).toBe(
        `URL denied: ${new URL(origin).host.replace("127.0.0.1", hostname)}`
      );
    });
  }

  for (const allowed of [false, true]) {
    it(`preserves parser-failure rules with ${allowed ? "allow and deny" : "deny only"}`, async () => {
      const result = await policyFetch("http://service.123:49151/", {
        LLRT_NET_DENY: "SERVICE.123:49151",
        ...(allowed ? { LLRT_NET_ALLOW: "service.123:49151" } : {}),
      });
      expect(result.code).toBe(0);
      expect(result.stdout.trim()).toBe("URL denied: service.123:49151");
    });
  }
});
