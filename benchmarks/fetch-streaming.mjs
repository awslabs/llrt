const ITERATIONS = 3;
const KB = 1024, MB = 1024 * KB, GB = 1024 * MB;
const CHUNK_SIZES = [4 * KB, 16 * KB, 64 * KB, 256 * KB, 1 * MB, 4 * MB];
const PAYLOAD_SIZES = [64 * KB, 256 * KB, 1 * MB, 4 * MB, 16 * MB, 64 * MB, 256 * MB, 1 * GB];

if (process.env.BENCH_PORT) {
  const { BENCH_PORT: port, BENCH_CHUNK: chunk, BENCH_PAYLOAD: payload } = process.env;
  const url = `http://localhost:${port}?chunkSize=${chunk}&totalSize=${payload}`;


  let total = 0;
  for (let i = 0; i < ITERATIONS; i++) {
    const start = performance.now();
    const reader = (await fetch(url)).body.getReader();
    while (!(await reader.read()).done);
    total += performance.now() - start;
  }
  const avgMs = total / ITERATIONS;
  console.log(JSON.stringify({ avgMs, mbps: payload / MB / (avgMs / 1000) }));
} else {
  const { createServer } = await import("node:http");
  const { spawn } = await import("node:child_process");

  const RUNTIMES = [
    ["LLRT", "llrt"],
    ["Node.js", "node"],
    ["Bun", "bun"],
  ];

  const server = createServer((req, res) => {
    const url = new URL(req.url, "http://localhost");
    const chunkSize = +url.searchParams.get("chunkSize") || KB;
    const totalSize = +url.searchParams.get("totalSize") || MB;
    res.writeHead(200, { "Content-Type": "application/octet-stream" });
    const chunk = Buffer.alloc(chunkSize, "x");
    let sent = 0;
    const send = () => {
      while (sent < totalSize) {
        const toSend = totalSize - sent < chunkSize ? Buffer.alloc(totalSize - sent, "x") : chunk;
        if (!res.write(toSend)) return (sent += toSend.length), res.once("drain", send);
        sent += toSend.length;
      }
      res.end();
    };
    send();
  });

  const runBenchmark = (cmd, port, chunk, payload) =>
    new Promise((resolve) => {
      const env = { ...process.env, BENCH_PORT: String(port), BENCH_CHUNK: String(chunk), BENCH_PAYLOAD: String(payload) };
      const proc = spawn(cmd, [import.meta.filename], { env, stdio: ["ignore", "pipe", "pipe"] });
      let out = "";
      proc.stdout.on("data", (d) => (out += d));
      proc.on("close", () => { try { resolve(JSON.parse(out.trim())); } catch { resolve({ avgMs: Infinity, mbps: 0 }); } });
    });

  const fmt = (n) => n >= GB ? `${n / GB}GB` : n >= MB ? `${n / MB}MB` : `${n / KB}KB`;

  server.listen(0, async () => {
    const port = server.address().port;
    const rows = [];
    const wins = Object.fromEntries(RUNTIMES.map(([n]) => [n, 0]));

    const total = PAYLOAD_SIZES.length * CHUNK_SIZES.length;
    let current = 0;

    for (const payload of PAYLOAD_SIZES) {
      for (const chunk of CHUNK_SIZES) {
        current++;
        process.stdout.write(`\r[${current}/${total}] Benchmarking payload=${fmt(payload)} chunk=${fmt(chunk)}...`);
        const results = await Promise.all(RUNTIMES.map(async ([name, cmd]) => [name, await runBenchmark(cmd, port, chunk, payload)]));
        const best = results.reduce((a, b) => b[1].avgMs < a[1].avgMs ? b : a);
        wins[best[0]]++;
        const row = { Payload: fmt(payload), Chunk: fmt(chunk) };
        for (const [name, r] of results) row[name] = `${r.avgMs.toFixed(2)}ms ${r.mbps.toFixed(0)}MB/s`;
        row.Winner = best[0];
        rows.push(row);
      }
    }
    process.stdout.write("\r" + " ".repeat(80) + "\r");

    console.log("\nFETCH STREAMING BENCHMARK - getReader().read() loop\n");
    console.table(rows);
    console.log("\nWins:", Object.entries(wins).map(([k, v]) => `${k}: ${v}`).join(" | "));
    server.close();
  });
}
