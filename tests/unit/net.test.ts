import defaultImport from "node:net";
import legacyImport from "net";
import type { Server, Socket } from "node:net";

const net = defaultImport;

it("node:net should be the same as net", () => {
  expect(defaultImport).toStrictEqual(legacyImport);
});

describe("server.close()", () => {
  const listen = (server: Server) =>
    new Promise<number>((resolve) =>
      server.listen(0, "127.0.0.1", () =>
        resolve((server.address() as any).port)
      )
    );
  const delay = (ms: number) =>
    new Promise((resolve) => setTimeout(resolve, ms));
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
    // Let LLRT's listen task, which owns the listening socket, see the close.
    await delay(50);
    const second = await tryConnect(port);
    const beforeEnd = [...events];
    client.end();
    await Promise.all([closed, socketClosed!]);
    expect(second).toEqual("error");
    expect(beforeEnd).toEqual([]);
    // As in Node, the server closes when its last connection is done, before
    // that socket emits 'close'.
    expect(events.slice(0, 2).sort()).toEqual(["socket:end", "socket:finish"]);
    expect(events.slice(2)).toEqual(["server:close", "socket:close"]);
  });
});

// Regression guards: these pass before and after the close paths became
// event-driven, and pin the order of the close events.
describe("close ordering (regression guards)", () => {
  const listen = (server: Server) =>
    new Promise<number>((resolve) =>
      server.listen(0, "127.0.0.1", () =>
        resolve((server.address() as any).port)
      )
    );
  const closeOf = (emitter: any) =>
    new Promise((resolve) => emitter.once("close", resolve));

  it("should emit a socket's 'close' after both of its sides are done", async () => {
    const events: string[] = [];
    const record = (name: string, socket: Socket) => {
      for (const event of ["end", "finish", "close"]) {
        socket.on(event, () => events.push(`${name}:${event}`));
      }
    };
    // The exchange can finish before an await resumes, so each socket's
    // 'close' is awaited from the moment the socket exists.
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
    // Both connections were done before the server closed. Its order against
    // the last socket's own 'close' is left to the server.close() test.
    expect(count(beforeClose, "socket:end")).toEqual(2);
    expect(count(beforeClose, "socket:finish")).toEqual(2);
    expect(count(events, "socket:close")).toEqual(2);
    expect(count(events, "server:close")).toEqual(1);
  });
});
