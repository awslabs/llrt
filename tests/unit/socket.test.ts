import net from "net";

describe("createServer and connect", () => {
  it("should create a server and connect to it", (done) => {
    const server = net.createServer();
    server.listen(() => {
      const client = net.connect((server.address() as any).port, () => {
        client.end(() => {
          server.close(done);
        });
      });
    });
  });

  it("should handle data transfer between server and client", (done) => {
    const message = "Hello from client";
    const server = net.createServer((socket) => {
      socket.on("data", (data) => {
        expect(data.toString()).toEqual(message)
        socket.write(data);
      });
    });
    server.listen(() => {
      const client = net.connect((server.address() as any).port, () => {
        client.write(message);
        client.on("data", (data) => {
          expect(data.toString()).toEqual(message)
          client.end(() => {
            server.close(done);
          });
        });
      });
    });
  });

  it("should handle data from server first", (done) => {
    const message = "Hello from client";
    const server = net.createServer((socket) => {
      socket.write(message);
    });
    server.listen(() => {
      const client = net.connect((server.address() as any).port, () => {
        client.on("data", (data) => {
          expect(data.toString()).toEqual(message)
          client.end(() => {
            server.close(done);
          });
        });
      });
    });
  });
});

describe("error handling", () => {
  it("should handle connection error", (done) => {
    const nonExistentPort = 9999;
    const client = net
      .connect(nonExistentPort, "localhost")
      .on("error", (error) => {
        expect(error).toBeInstanceOf(Error);
        client.end();
        done(); // Test passes if an error event is emitted
      });
  });

  it("should handle server destroy", (done) => {
    const server = net.createServer((socket) => {
      socket.on("data", () => {
        socket.destroy();
      });
    });

    server.listen(() => {
      const client = net.connect((server.address() as any).port, () => {
        client.write("hello");
      });
      client.on("close", () => {
        client.end();
        server.close(done);
      });
    });
  });

  it("should handle client destroy", (done) => {
    const server = net.createServer((socket) => {
      setTimeout(() => {
        socket.write("hello", (err) => {
          expect(err).toBeTruthy();
          server.close();
          done();
        });
      }, 100);
    });

    server.listen(() => {
      const client = net.connect((server.address() as any).port, () => {
        client.destroy();
      });
    });
  });
});
