import dgram from "dgram";
import defaultImport from "node:dgram";

describe("dgram module", () => {
  it("node:dgram should be the same as dgram", () => {
    expect(defaultImport).toStrictEqual(dgram);
  });

  it("should export createSocket function", () => {
    expect(typeof dgram.createSocket).toBe("function");
  });

  it("should export Socket class", () => {
    expect(typeof dgram.Socket).toBe("function");
  });

  it("should create a UDP4 socket", () => {
    const socket = dgram.createSocket("udp4");
    expect(socket).toBeDefined();
  });

  it("should create a UDP6 socket", () => {
    const socket = dgram.createSocket("udp6");
    expect(socket).toBeDefined();
  });

  it("should create a socket with options object", () => {
    const socket = dgram.createSocket({ type: "udp4" });
    expect(socket).toBeDefined();
  });

  it("should bind and close a socket", async () => {
    const socket = dgram.createSocket("udp4");

    await new Promise((resolve) => {
      socket.bind(() => {
        resolve();
      });
    });

    const address = socket.address();
    expect(address).toBeDefined();
    expect(address.port).toBeGreaterThan(0);
    expect(address.family).toBe("IPv4");

    await new Promise((resolve) => {
      socket.close(() => {
        resolve();
      });
    });
  });

  it("should send and receive messages", async () => {
    const server = dgram.createSocket("udp4");
    const client = dgram.createSocket("udp4");

    const testMessage = "Hello, UDP!";

    await new Promise((resolve, reject) => {
      let messageReceived = false;

      server.on("message", (msg, rinfo) => {
        try {
          expect(msg.toString()).toBe(testMessage);
          expect(rinfo.address).toBeDefined();
          expect(rinfo.port).toBeGreaterThan(0);
          expect(rinfo.family).toBe("IPv4");
          messageReceived = true;
        } catch (err) {
          reject(err);
        }
      });

      server.on("listening", () => {
        const address = server.address();

        client.send(testMessage, address.port, "127.0.0.1", (err) => {
          if (err) {
            reject(err);
          }
        });

        // Wait a bit for message to arrive
        setTimeout(() => {
          try {
            expect(messageReceived).toBe(true);
            client.close();
            server.close();
            resolve();
          } catch (err) {
            reject(err);
          }
        }, 100);
      });

      server.on("error", (err) => {
        reject(err);
      });

      server.bind(0); // Bind to random port
    });
  }, 5000);

  it("should support unref and ref methods", () => {
    const socket = dgram.createSocket("udp4");
    expect(socket.unref()).toBe(socket);
    expect(socket.ref()).toBe(socket);
  });

  it("should emit listening event", async () => {
    const socket = dgram.createSocket("udp4");

    await new Promise((resolve) => {
      socket.on("listening", () => {
        const address = socket.address();
        expect(address).toBeDefined();
        socket.close();
        resolve();
      });

      socket.bind(0);
    });
  });

  it("should emit close event", async () => {
    const socket = dgram.createSocket("udp4");

    await new Promise((resolve) => {
      socket.on("close", () => {
        resolve();
      });

      socket.bind(() => {
        socket.close();
      });
    });
  });
});
