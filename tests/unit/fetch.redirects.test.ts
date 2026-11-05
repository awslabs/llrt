import net from "node:net";
import { spawnCapture } from "./test-utils";

type SeenRequest = {
  method: string;
  path: string;
  body: string;
  authorization: string | null;
  contentType: string | null;
};
let server: net.Server;
let otherServer: net.Server;
let origin: string;
let otherOrigin: string;
const requests: SeenRequest[] = [];

function createServer() {
  return net.createServer((socket) => {
    let data = "";
    let replied = false;
    socket.on("error", () => {});
    socket.on("data", (chunk) => {
      data += chunk.toString();
      const end = data.indexOf("\r\n\r\n");
      if (end < 0 || replied) return;
      const length = Number(
        data.slice(0, end).match(/\r\ncontent-length: (\d+)/i)?.[1] ?? 0
      );
      if (data.length < end + 4 + length) return;
      replied = true;
      const [method, path] = data.split("\r\n", 1)[0].split(" ");
      const request: SeenRequest = {
        method,
        path,
        body: data.slice(end + 4, end + 4 + length),
        authorization:
          data.slice(0, end).match(/\r\nauthorization: ([^\r\n]*)/i)?.[1] ??
          null,
        contentType:
          data.slice(0, end).match(/\r\ncontent-type: ([^\r\n]*)/i)?.[1] ??
          null,
      };
      requests.push(request);
      const url = new URL(path, "http://localhost");
      let status = 200;
      let location: string | null = null;
      if (url.pathname === "/a/start" && !url.searchParams.has("done")) {
        status = Number(url.searchParams.get("status") ?? 302);
        location = url.searchParams.get("to");
      } else if (url.pathname === "/chain/start") {
        status = 302;
        location = "../b/step";
      } else if (url.pathname === "/b/step") {
        status = 302;
        location = "../c/final";
      } else if (url.pathname === "/loop") {
        status = 302;
        location = "./loop";
      } else if (
        url.pathname === "/once" &&
        requests.filter((r) => r.path === path).length === 1
      ) {
        status = 302;
        location = url.searchParams.get("to");
      }
      const body = status === 304 ? "" : JSON.stringify(request);
      socket.write(
        [
          `HTTP/1.1 ${status} Test`,
          "Connection: close",
          "Content-Type: application/json",
          `Content-Length: ${body.length}`,
          ...(location === null ? [] : [`Location: ${location}`]),
          "",
          body,
        ].join("\r\n")
      );
      socket.end();
    });
  });
}

beforeAll(async () => {
  server = createServer();
  otherServer = createServer();
  await Promise.all(
    [server, otherServer].map(
      (s) => new Promise<void>((resolve) => s.listen(0, "127.0.0.1", resolve))
    )
  );
  origin = `http://127.0.0.1:${(server.address() as net.AddressInfo).port}`;
  otherOrigin = `http://127.0.0.1:${(otherServer.address() as net.AddressInfo).port}`;
});

afterAll(async () => {
  await Promise.all(
    [server, otherServer].map(
      (s) => new Promise<void>((resolve) => s.close(() => resolve()))
    )
  );
});

const start = (to: string, status = 302) =>
  `${origin}/a/start?to=${encodeURIComponent(to)}&status=${status}`;

describe("fetch redirects", () => {
  for (const [name, location, target] of [
    ["path-relative", "next", "/a/next"],
    ["parent-relative", "../final", "/final"],
    ["root-relative", "/final?value=1", "/final?value=1"],
    ["query-only", "?done=1", "/a/start?done=1"],
    ["dot segments", "./nested/../../final", "/final"],
    ["encoded path", "../a%20b?q=a%2Fb", "/a%20b?q=a%2Fb"],
  ]) {
    it(`resolves ${name} Location against the response URL`, async () => {
      const response = await fetch(start(location));
      expect(response.status).toBe(200);
      expect(response.url).toBe(origin + target);
      expect(response.redirected).toBe(true);
      expect((await response.json()).path).toBe(target);
    });
  }

  it("continues to follow absolute URLs", async () => {
    const response = await fetch(start(origin + "/absolute"));
    expect(response.url).toBe(origin + "/absolute");
    expect((await response.json()).path).toBe("/absolute");
  });

  it("resolves each chained Location against the latest response URL", async () => {
    const response = await fetch(origin + "/chain/start");
    expect(response.url).toBe(origin + "/c/final");
    expect((await response.json()).path).toBe("/c/final");
  });

  for (const location of ["", "#fragment"]) {
    it(`follows a ${location ? "fragment-only" : "empty"} Location without sending a fragment`, async () => {
      const url = `${origin}/once?to=${encodeURIComponent(location)}`;
      const response = await fetch(url);
      expect(response.redirected).toBe(true);
      expect(response.url).toBe(url);
      expect((await response.json()).path).toBe(
        new URL(url).pathname + new URL(url).search
      );
    });
  }

  it("preserves the source response and URL in manual mode", async () => {
    const url = start("../manual-target");
    const response = await fetch(url, { redirect: "manual" });
    expect(response.status).toBe(302);
    expect(response.url).toBe(url);
    expect(response.redirected).toBe(false);
    expect(response.headers.get("location")).toBe("../manual-target");
    await response.text();
    expect(requests.some((r) => r.path === "/manual-target")).toBe(false);
  });

  it("rejects in error mode without requesting the target", async () => {
    await expect(
      fetch(start("../error-target"), { redirect: "error" })
    ).rejects.toThrow("Unexpected redirect");
    expect(requests.some((r) => r.path === "/error-target")).toBe(false);
  });

  it("returns a redirect without Location unchanged", async () => {
    const url = origin + "/a/start";
    const response = await fetch(url);
    expect(response.status).toBe(302);
    expect(response.url).toBe(url);
    expect(response.redirected).toBe(false);
    await response.text();
  });

  for (const [status, method, body] of [
    [301, "GET", ""],
    [302, "GET", ""],
    [303, "GET", ""],
    [307, "POST", "payload"],
    [308, "POST", "payload"],
  ] as const) {
    it(`preserves method/body rules for ${status} with a relative Location`, async () => {
      const response = await fetch(start("../method", status), {
        method: "POST",
        body: "payload",
        headers: { "content-type": "text/plain" },
      });
      const result = await response.json();
      expect(result.method).toBe(method);
      expect(result.body).toBe(body);
      expect(result.contentType).toBe(method === "GET" ? null : "text/plain");
    });
  }

  for (const relative of [false, true]) {
    it(`retains authorization with a mixed-case hostname and ${relative ? "relative" : "absolute"} Location`, async () => {
      const localOrigin = origin.replace("127.0.0.1", "LOCALHOST");
      const location = relative ? "../case-auth" : localOrigin + "/case-auth";
      const url = start(location).replace(origin, localOrigin);
      const response = await fetch(url, {
        headers: { authorization: "Bearer case-test" },
      });
      expect((await response.json()).authorization).toBe("Bearer case-test");
    });
  }

  it("retains same-origin authorization and strips it across a scheme-relative redirect", async () => {
    const headers = { authorization: "Bearer test" };
    const same = await fetch(start("../same-auth"), { headers });
    expect((await same.json()).authorization).toBe("Bearer test");
    const cross = await fetch(
      start(otherOrigin.replace("http:", "") + "/cross-auth"),
      { headers }
    );
    expect(cross.url).toBe(otherOrigin + "/cross-auth");
    expect((await cross.json()).authorization).toBe(null);
  });

  for (const location of [
    "http://[",
    "file:///tmp/not-fetched",
    "data:text/plain,not-fetched",
    "http://127.0.0.1:1/blocked",
  ]) {
    it(`rejects invalid or unsupported redirect target ${location}`, async () => {
      await expect(fetch(start(location))).rejects.toThrow();
    });
  }

  it("allows same-origin relative redirects under LLRT_NET_ALLOW", async () => {
    const url = start("../allowed");
    const { LLRT_LOG, LLRT_NET_ALLOW, LLRT_NET_DENY, ...env } = process.env;
    const result = await spawnCapture(
      process.argv[0],
      [
        "-e",
        `fetch(${JSON.stringify(url)}).then(r => r.json()).then(r => console.log(r.path))`,
      ],
      { env: { ...env, LLRT_NET_ALLOW: origin } }
    );
    expect(result.code).toBe(0);
    expect(result.stdout.trim()).toBe("/allowed");
  });

  it("does not apply destination access restrictions in manual mode", async () => {
    const url = start(otherOrigin.replace("http:", "") + "/manual-denied");
    const { LLRT_LOG, LLRT_NET_ALLOW, LLRT_NET_DENY, ...env } = process.env;
    const result = await spawnCapture(
      process.argv[0],
      [
        "-e",
        `fetch(${JSON.stringify(url)}, {redirect: "manual"}).then(async r => { await r.text(); console.log(r.status, r.url); })`,
      ],
      { env: { ...env, LLRT_NET_DENY: otherOrigin } }
    );
    expect(result.code).toBe(0);
    expect(result.stdout.trim()).toBe(`302 ${url}`);
    expect(requests.some((r) => r.path === "/manual-denied")).toBe(false);
  });

  async function policyFetch(location: string, policy: Record<string, string>) {
    const { LLRT_LOG, LLRT_NET_ALLOW, LLRT_NET_DENY, ...env } = process.env;
    return spawnCapture(
      process.argv[0],
      [
        "-e",
        `fetch(${JSON.stringify(start(location))}).then(r => r.json()).then(r => console.log(r.path), e => console.log(e.message))`,
      ],
      { env: { ...env, ...policy } }
    );
  }

  for (const explicitPort of [false, true]) {
    const target = `https://127.0.0.1${explicitPort ? ":443" : ""}/https-policy`;
    for (const entry of ["127.0.0.1", "127.0.0.1:443"]) {
      it(`denies redirected HTTPS ${target} with ${entry}`, async () => {
        const result = await policyFetch(target, { LLRT_NET_DENY: entry });
        expect(result.code).toBe(0);
        expect(result.stdout.trim()).toBe("URL denied: 127.0.0.1");
      });
      it(`allows redirected HTTPS ${target} through the allow check with ${entry}`, async () => {
        const result = await policyFetch(target, {
          LLRT_NET_ALLOW: `${origin} ${entry}`,
          LLRT_NET_DENY: "127.0.0.1:443",
        });
        expect(result.code).toBe(0);
        // Deny runs after allow, avoiding a TLS listener on privileged port 443.
        expect(result.stdout.trim()).toBe("URL denied: 127.0.0.1");
      });
    }
    it(`rejects redirected HTTPS ${target} with a port-80-only allow rule`, async () => {
      const result = await policyFetch(target, {
        LLRT_NET_ALLOW: `${origin} 127.0.0.1:80`,
      });
      expect(result.code).toBe(0);
      expect(result.stdout.trim()).toBe("URL not allowed: 127.0.0.1");
    });
  }

  for (const hostname of ["LOCALHOST", "127.1", "0x7f000001"]) {
    for (const restriction of ["LLRT_NET_ALLOW", "LLRT_NET_DENY"]) {
      it(`matches ${restriction} for redirected ${hostname}`, async () => {
        const path = `/host-policy-${hostname}-${restriction}`;
        const policyOrigin = otherOrigin.replace("127.0.0.1", hostname);
        const target = policyOrigin + path;
        const result = await policyFetch(target, {
          [restriction]:
            restriction === "LLRT_NET_ALLOW"
              ? `${origin} ${policyOrigin}`
              : policyOrigin,
        });
        expect(result.code).toBe(0);
        if (restriction === "LLRT_NET_ALLOW") {
          expect(result.stdout.trim()).toBe(path);
          expect(requests.some((r) => r.path === path)).toBe(true);
        } else {
          expect(result.stdout.trim()).toBe(
            `URL denied: ${new URL(target).host}`
          );
          expect(requests.some((r) => r.path === path)).toBe(false);
        }
      });
    }
  }

  it("still bounds relative redirect loops", async () => {
    await expect(fetch(origin + "/loop")).rejects.toThrow(
      "Max retries exceeded"
    );
  });

  for (const restriction of ["LLRT_NET_ALLOW", "LLRT_NET_DENY"]) {
    it(`checks ${restriction} after resolving a scheme-relative target`, async () => {
      const path = "/restricted-" + restriction;
      const url = start(otherOrigin.replace("http:", "") + path);
      const { LLRT_LOG, LLRT_NET_ALLOW, LLRT_NET_DENY, ...env } = process.env;
      const result = await spawnCapture(
        process.argv[0],
        [
          "-e",
          `fetch(${JSON.stringify(url)}).then(() => console.log("UNEXPECTED"), e => console.log(e.message))`,
        ],
        {
          env: {
            ...env,
            [restriction]:
              restriction === "LLRT_NET_ALLOW" ? origin : otherOrigin,
          },
        }
      );
      expect(result.code).toBe(0);
      const message =
        restriction === "LLRT_NET_ALLOW" ? "URL not allowed" : "URL denied";
      expect(result.stdout.trim()).toBe(
        `${message}: ${new URL(otherOrigin).host}`
      );
      expect(requests.some((r) => r.path === path)).toBe(false);
    });
  }
});
