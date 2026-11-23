import defaultImport from "node:net";
import legacyImport from "net";
import type { Server, Socket } from "node:net";
import stream from "node:stream";

const net = defaultImport;

it("node:net should be the same as net", () => {
  expect(defaultImport).toStrictEqual(legacyImport);
});

const listen = (server: Server) =>
  new Promise<number>((resolve) =>
    server.listen(0, "127.0.0.1", () => resolve((server.address() as any).port))
  );
const closeOf = (emitter: any) =>
  new Promise<unknown>((resolve) => emitter.once("close", resolve));
const closeServer = (server: Server) =>
  new Promise<void>((resolve) => server.close(() => resolve()));
const delay = (ms: number) => new Promise((resolve) => setTimeout(resolve, ms));

describe("net.Socket as a stream.Duplex", () => {
  it("should be an instance of stream.Duplex", () => {
    const socket = new net.Socket();
    expect(socket).toBeInstanceOf(stream.Duplex);
    expect(Object.getPrototypeOf(net.Socket)).toBe(stream.Duplex);
    socket.destroy();
  });

  it("should carry data through pipe() and pipeline()", async () => {
    const server = net.createServer((socket) => socket.pipe(socket));
    const port = await listen(server);
    const client = net.connect(port, "127.0.0.1");
    const chunks: Buffer[] = [];
    await new Promise<void>((resolve, reject) =>
      stream.pipeline(
        stream.Readable.from(["a", "b", "c"]),
        client,
        new stream.Writable({
          write(chunk, _encoding, callback) {
            chunks.push(chunk);
            callback();
          },
        }),
        (err) => (err ? reject(err) : resolve())
      )
    );
    expect(Buffer.concat(chunks).toString()).toEqual("abc");
    await closeServer(server);
  });

  it("should stop reading while paused and full, then deliver everything on resume", async () => {
    const total = 4 * 1024 * 1024;
    const server = net.createServer((socket) =>
      socket.end(Buffer.alloc(total, 1))
    );
    const port = await listen(server);
    const client = net.connect(port, "127.0.0.1");
    const closed = closeOf(client);
    let received = 0;
    await new Promise<void>((resolve) => {
      const onFirst = (chunk: Buffer) => {
        received += chunk.length;
        client.off("data", onFirst);
        client.pause();
        resolve();
      };
      client.on("data", onFirst);
    });
    await delay(100);
    expect(client.readableHighWaterMark).toEqual(64 * 1024);
    expect(client.readableLength).toBeGreaterThan(0);
    expect(client.readableLength).toBeLessThanOrEqual(
      client.readableHighWaterMark + 64 * 1024
    );
    client.on("data", (chunk) => (received += chunk.length));
    client.resume();
    await closed;
    expect(received).toEqual(total);
    await closeServer(server);
  });

  it("should decode data with setEncoding(), across chunks", async () => {
    const bytes = Buffer.from("héllo wörld");
    const server = net.createServer((socket) => {
      socket.write(bytes.subarray(0, 2));
      setTimeout(() => socket.end(bytes.subarray(2)), 20);
    });
    const port = await listen(server);
    const client = net.connect(port, "127.0.0.1");
    const closed = closeOf(client);
    client.setEncoding("utf8");
    const chunks: unknown[] = [];
    client.on("data", (chunk) => chunks.push(chunk));
    await closed;
    expect(chunks.every((chunk) => typeof chunk === "string")).toBe(true);
    expect(chunks.join("")).toEqual("héllo wörld");
    await closeServer(server);
  });

  it("should support async iteration", async () => {
    const server = net.createServer((socket) => {
      socket.write("a");
      setTimeout(() => socket.end("b"), 20);
    });
    const port = await listen(server);
    const client = net.connect(port, "127.0.0.1");
    const closed = closeOf(client);
    const chunks: string[] = [];
    for await (const chunk of client) {
      chunks.push(chunk.toString());
    }
    await closed;
    expect(chunks.join("")).toEqual("ab");
    await closeServer(server);
  });

  it("should return null from read() until data arrives, and emit 'end' only once the data is read", async () => {
    const server = net.createServer((socket) => socket.end("data"));
    const port = await listen(server);
    const client = net.connect(port, "127.0.0.1");
    const closed = closeOf(client);
    let ended = false;
    client.on("end", () => (ended = true));
    expect(client.read()).toBeNull();
    await new Promise((resolve) => client.once("readable", resolve));
    await delay(50);
    expect(ended).toBe(false);
    expect(client.read().toString()).toEqual("data");
    await closed;
    expect(ended).toBe(true);
    await closeServer(server);
  });

  it("should deliver data to a once('data') listener (regression guard)", async () => {
    const server = net.createServer((socket) => {
      socket.write("hello");
      setTimeout(() => socket.end(), 20);
    });
    const port = await listen(server);
    const client = net.connect(port, "127.0.0.1");
    const closed = closeOf(client);
    const data = await new Promise<Buffer>((resolve) =>
      client.once("data", resolve)
    );
    expect(data.toString()).toEqual("hello");
    expect(client.listenerCount("data")).toEqual(0);
    await closed;
    await closeServer(server);
  });
});

describe("half-open sockets", () => {
  it("should end the writable side once the readable side ends, without allowHalfOpen", async () => {
    const events: string[] = [];
    let serverClosed: Promise<unknown>;
    let lateWrite: Promise<any>;
    const server = net.createServer((socket) => {
      serverClosed = closeOf(socket);
      socket.on("end", () => events.push("end"));
      socket.on("error", (err: any) => events.push(`error:${err.code}`));
      lateWrite = new Promise((resolve) =>
        socket.on("finish", () => {
          events.push("finish");
          socket.write("late", resolve);
        })
      );
      socket.resume();
    });
    const port = await listen(server);
    const client = net.connect(port, "127.0.0.1", () => client.end());
    const closed = closeOf(client);
    client.resume();
    await closed;
    await serverClosed!;
    expect((await lateWrite!).code).toEqual("EPIPE");
    expect(events).toEqual(["end", "finish", "error:EPIPE"]);
    await closeServer(server);
  });

  it("should keep the writable side open after the readable side ends, with allowHalfOpen", async () => {
    let writableAtEnd: boolean | undefined;
    const server = net.createServer({ allowHalfOpen: true }, (socket) => {
      socket.on("end", () => {
        writableAtEnd = socket.writable;
        setTimeout(() => socket.end("reply"), 20);
      });
      socket.resume();
    });
    const port = await listen(server);
    const client = net.connect(port, "127.0.0.1", () => client.end());
    const closed = closeOf(client);
    let reply = "";
    client.on("data", (chunk) => (reply += chunk));
    await closed;
    expect(writableAtEnd).toBe(true);
    expect(reply).toEqual("reply");
    await closeServer(server);
  });
});

describe("writes", () => {
  it("should call a write's callback once its bytes are written, after 'drain'", async () => {
    const total = 8 * 1024 * 1024;
    let serverSocket: Socket;
    let serverClosed: Promise<unknown>;
    let onConnection: () => void;
    const connection = new Promise<void>((resolve) => {
      onConnection = resolve;
    });
    const server = net.createServer((socket) => {
      serverSocket = socket;
      serverClosed = closeOf(socket);
      socket.pause();
      onConnection();
    });
    const port = await listen(server);
    const client = net.connect(port, "127.0.0.1");
    const closed = closeOf(client);
    await connection;
    const events: string[] = [];
    const drained = new Promise((resolve) => client.once("drain", resolve));
    client.on("drain", () => events.push("drain"));
    const ok = client.write(Buffer.alloc(total, 1), () =>
      events.push("callback")
    );
    expect(ok).toBe(false);
    await delay(100);
    expect(events).toEqual([]);
    serverSocket!.resume();
    await drained;
    expect(events).toEqual(["drain", "callback"]);
    client.end();
    await Promise.all([closed, serverClosed!]);
    await closeServer(server);
  });
});

describe("destroy and errors", () => {
  it("should fail a write on a socket that never connected", async () => {
    const socket = new net.Socket();
    const events: string[] = [];
    socket.on("error", (err: any) => events.push(`error:${err.code}`));
    socket.on("close", () => events.push("close"));
    const closed = closeOf(socket);
    expect(socket.write("x")).toBe(false);
    await closed;
    expect(events).toEqual(["error:ERR_SOCKET_CLOSED", "close"]);
  });

  it("should emit 'error', then 'close' with hadError, on destroy(err)", async () => {
    const server = net.createServer((socket) => socket.resume());
    const port = await listen(server);
    const events: string[] = [];
    const client = net.connect(port, "127.0.0.1", () =>
      client.destroy(new Error("boom"))
    );
    client.on("error", (err) => events.push(`error:${err.message}`));
    await new Promise<void>((resolve) =>
      client.on("close", (hadError) => {
        events.push(`close:${hadError}`);
        resolve();
      })
    );
    expect(client.destroyed).toBe(true);
    expect(events).toEqual(["error:boom", "close:true"]);
    await closeServer(server);
  });

  it("should emit 'error' with ECONNREFUSED, then 'close', on a refused connect", async () => {
    const tmp = net.createServer();
    const port = await listen(tmp);
    await closeServer(tmp);
    const events: string[] = [];
    const client = net.connect(port, "127.0.0.1");
    client.on("error", (err: any) => events.push(`error:${err.code}`));
    await new Promise<void>((resolve) =>
      client.on("close", (hadError) => {
        events.push(`close:${hadError}`);
        resolve();
      })
    );
    expect(events).toEqual(["error:ECONNREFUSED", "close:true"]);
  });
});

describe("server.close()", () => {
  // Resolves with the first of "connect" and "error".
  const tryConnect = (port: number) =>
    new Promise<string>((resolve) => {
      const socket = net.connect(port, "127.0.0.1");
      socket.on("connect", () => {
        socket.destroy();
        resolve("connect");
      });
      socket.on("error", () => resolve("error"));
    });

  it("should refuse new connections while one is still open, and emit 'close' once it is done", async () => {
    const events: string[] = [];
    let socketClosed: Promise<unknown>;
    let onConnection: () => void;
    const connection = new Promise<void>((resolve) => {
      onConnection = resolve;
    });
    const server = net.createServer((socket) => {
      for (const event of ["end", "finish", "close"]) {
        socket.on(event, () => events.push(`socket:${event}`));
      }
      socketClosed = new Promise((resolve) => socket.once("close", resolve));
      onConnection();
    });
    const port = await listen(server);
    const client = net.connect(port, "127.0.0.1");
    await connection;
    const closed = new Promise<void>((resolve) =>
      server.on("close", () => {
        events.push("server:close");
        resolve();
      })
    );
    server.close();
    const second = await tryConnect(port);
    const beforeEnd = [...events];
    client.end();
    await Promise.all([closed, socketClosed!]);
    expect(second).toEqual("error");
    expect(beforeEnd).toEqual([]);
    // The server closes when its last connection is done, before that socket's 'close'.
    expect(events.slice(0, 2).sort()).toEqual(["socket:end", "socket:finish"]);
    expect(events.slice(2)).toEqual(["server:close", "socket:close"]);
  });
});

// Regression guards: these pass on main too, and pin the order of the close events.
describe("close ordering (regression guards)", () => {
  it("should emit a socket's 'close' after both of its sides are done", async () => {
    const events: string[] = [];
    const record = (name: string, socket: Socket) => {
      for (const event of ["end", "finish", "close"]) {
        socket.on(event, () => events.push(`${name}:${event}`));
      }
    };
    // Each 'close' is awaited from the start: the exchange can end before an await resumes.
    const closed: Promise<unknown>[] = [];
    let onConnection: () => void;
    const connection = new Promise<void>((resolve) => {
      onConnection = resolve;
    });
    const server = net.createServer((socket) => {
      record("srv", socket);
      closed.push(closeOf(socket));
      socket.on("data", () => {
        socket.write("pong");
        socket.end();
      });
      onConnection();
    });
    const port = await listen(server);
    const client = net.connect(port, "127.0.0.1", () => client.write("ping"));
    record("cli", client);
    closed.push(closeOf(client));
    client.on("data", () => {});
    await connection;
    await Promise.all(closed);
    await new Promise((resolve) => server.close(resolve));
    for (const side of ["cli", "srv"]) {
      const own = events.filter((event) => event.startsWith(`${side}:`));
      expect(own.at(-1)).toEqual(`${side}:close`);
      expect([...own].sort()).toEqual([
        `${side}:close`,
        `${side}:end`,
        `${side}:finish`,
      ]);
    }
  });

  it("should emit the server's 'close' only once its last connection is done", async () => {
    const events: string[] = [];
    const waiters: (() => void)[] = [];
    const nextSocketClose = () =>
      new Promise<void>((resolve) => waiters.push(resolve));
    const socketsClosed: Promise<unknown>[] = [];
    let onConnections: () => void;
    const connections = new Promise<void>((resolve) => {
      onConnections = resolve;
    });
    const server = net.createServer((socket) => {
      for (const event of ["end", "finish"]) {
        socket.on(event, () => events.push(`socket:${event}`));
      }
      socket.on("close", () => {
        events.push("socket:close");
        waiters.shift()?.();
      });
      socketsClosed.push(closeOf(socket));
      if (socketsClosed.length === 2) onConnections();
    });
    const port = await listen(server);
    const clients = [
      net.connect(port, "127.0.0.1"),
      net.connect(port, "127.0.0.1"),
    ];
    await connections;
    const closed = new Promise<void>((resolve) =>
      server.close(() => {
        events.push("server:close");
        resolve();
      })
    );
    const first = nextSocketClose();
    clients[0].end();
    await first;
    clients[1].end();
    await Promise.all([closed, ...socketsClosed]);
    const count = (list: string[], event: string) =>
      list.filter((item) => item === event).length;
    const beforeClose = events.slice(0, events.indexOf("server:close"));
    // Both connections were done before the server closed.
    expect(count(beforeClose, "socket:end")).toEqual(2);
    expect(count(beforeClose, "socket:finish")).toEqual(2);
    expect(count(events, "socket:close")).toEqual(2);
    expect(count(events, "server:close")).toEqual(1);
  });
});
