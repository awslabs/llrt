/**
 * Smoke test for LLRT process stdio and fetch APIs.
 *
 * Run with:
 *   ./target/release/llrt tests/smoke/stdio-fetch.js
 *
 * Exit code 0 = all checks passed.
 * Failed assertions are counted; the script exits non-zero at the end if any failed.
 */

import net from "node:net";

// ── helpers ──────────────────────────────────────────────────────────────────

let passed = 0;
let failed = 0;

function assert(condition, label) {
  if (condition) {
    process.stdout.write("  PASS  " + label + "\n");
    passed++;
  } else {
    process.stderr.write("  FAIL  " + label + "\n");
    failed++;
  }
}

function assertEq(actual, expected, label) {
  if (actual === expected) {
    process.stdout.write("  PASS  " + label + "\n");
    passed++;
  } else {
    process.stderr.write(
      "  FAIL  " +
        label +
        "  (got: " +
        JSON.stringify(actual) +
        ", want: " +
        JSON.stringify(expected) +
        ")\n"
    );
    failed++;
  }
}

// ── section 1: process.stdout / process.stderr ───────────────────────────────

process.stdout.write("=== section: process.stdout / process.stderr ===\n");

assert(typeof process.stdout === "object", "process.stdout is an object");
assert(
  typeof process.stdout.write === "function",
  "process.stdout.write is a function"
);
assert(typeof process.stderr === "object", "process.stderr is an object");
assert(
  typeof process.stderr.write === "function",
  "process.stderr.write is a function"
);
assert(
  typeof process.stdout.columns === "number",
  "process.stdout.columns is a number"
);
assert(
  typeof process.stdout.rows === "number",
  "process.stdout.rows is a number"
);
assert(
  typeof process.stdout.isTTY === "boolean",
  "process.stdout.isTTY is a boolean"
);

// write() should return a boolean (or undefined — both acceptable as per Node.js)
const writeResult = process.stdout.write("");
assert(
  writeResult === true || writeResult === false || writeResult === undefined,
  "process.stdout.write returns boolean-ish"
);

process.stderr.write("  (this line intentionally goes to stderr)\n");

// ── section 2: process.stdin ─────────────────────────────────────────────────

process.stdout.write("=== section: process.stdin ===\n");

assert(typeof process.stdin === "object", "process.stdin is an object");
// fd 0 is stdin; isTTY is false when stdin is not a terminal (e.g. in CI/pipe)
assert(
  typeof process.stdin.isTTY === "boolean",
  "process.stdin.isTTY is a boolean"
);

// ── section 3: process.on('exit') ────────────────────────────────────────────

process.stdout.write("=== section: process.on/off/once ===\n");

// on() — registered below, will fire during natural exit drain
let exitListenerFired = false;
process.on("exit", (code) => {
  exitListenerFired = true;
  // We can only use synchronous APIs inside an exit listener.
  process.stderr.write("  INFO  exit listener fired with code=" + code + "\n");
});

// off() — register then immediately remove; must not fire
let removedFired = false;
const removedCb = () => {
  removedFired = true;
};
process.on("exit", removedCb);
process.off("exit", removedCb);

// once() — should fire exactly once
let onceCount = 0;
process.once("exit", () => {
  onceCount++;
});

// Validate removedFired and onceCount inside a final exit listener so the
// assertions run after all other listeners have executed.
process.on("exit", () => {
  if (removedFired) {
    process.stderr.write(
      "  FAIL  process.off: removed listener still fired on exit\n"
    );
    process.exitCode = 1;
  } else {
    process.stdout.write(
      "  PASS  process.off: removed listener did not fire\n"
    );
  }
  if (onceCount !== 1) {
    process.stderr.write(
      "  FAIL  process.once: expected onceCount=1 but got " + onceCount + "\n"
    );
    process.exitCode = 1;
  } else {
    process.stdout.write("  PASS  process.once: fired exactly once\n");
  }
});

assert(typeof process.on === "function", "process.on is a function");
assert(typeof process.off === "function", "process.off is a function");
assert(typeof process.once === "function", "process.once is a function");
assert(
  typeof process.addListener === "function",
  "process.addListener is a function"
);
assert(
  typeof process.removeListener === "function",
  "process.removeListener is a function"
);

// ── section 4: exitCode accessor ─────────────────────────────────────────────

process.stdout.write("=== section: process.exitCode ===\n");

process.exitCode = 7;
assertEq(process.exitCode, 7, "exitCode round-trips through getter");
process.exitCode = 0; // reset

// ── section 5: process.exit() validation ─────────────────────────────────────

process.stdout.write("=== section: process.exit() validation ===\n");

let threwRangeError = false;
try {
  process.exit(1.5);
} catch (e) {
  threwRangeError = e instanceof RangeError;
}
assert(threwRangeError, "process.exit(1.5) throws RangeError");

// ── section 6: globalThis.__exit_listeners is hidden ─────────────────────────

process.stdout.write("=== section: security — EXIT_LISTENERS hidden ===\n");

assert(
  globalThis.__exit_listeners === undefined,
  "__exit_listeners is not reachable from JS"
);

// ── section 7: fetch against a local HTTP server ─────────────────────────────

process.stdout.write("=== section: fetch ===\n");

const server = net.createServer((socket) => {
  socket.on("error", () => {});
  socket.on("data", () => {
    socket.write(
      "HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\nContent-Length: 5\r\n\r\nhello"
    );
    socket.end();
  });
});

await new Promise((resolve, reject) => {
  server.once("error", reject);
  server.listen(0, "127.0.0.1", resolve);
});

const { port } = server.address();
const url = "http://127.0.0.1:" + port;

const controller = new AbortController();
const fetchTimeout = setTimeout(() => controller.abort(), 5000);

try {
  const res = await fetch(url, { signal: controller.signal });
  clearTimeout(fetchTimeout);
  assertEq(res.status, 200, "fetch: response status is 200");
  const body = await res.text();
  assertEq(body, "hello", "fetch: response body is 'hello'");
  assert(
    res.headers.get("content-type")?.startsWith("text/plain"),
    "fetch: Content-Type starts with text/plain"
  );
} catch (e) {
  clearTimeout(fetchTimeout);
  process.stderr.write("  FAIL  fetch threw: " + e + "\n");
  failed++;
} finally {
  server.close();
}

// ── summary ───────────────────────────────────────────────────────────────────

process.stdout.write(
  "\n=== summary: " + passed + " passed, " + failed + " failed ===\n"
);

if (failed > 0) {
  process.exit(1);
}

// Exit listeners registered above will fire during natural process termination.
// They run after this script completes — their output goes to stderr.
