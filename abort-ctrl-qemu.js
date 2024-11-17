import net from "net";

const server = net.createServer((socket) => {
  socket.on("data", () => {
    socket.write(
      "HTTP/1.1 200 OK\r\nContent-Type: text/html\r\n\r\n<html></html>"
    );
    socket.end();
  });
});
server.listen(() => {
  const { port } = server.address();
  const url = `http://localhost:${port}`;

  for (let i = 0; i < 1 * 1000 * 1000; i++) {
    const abortController = new AbortController();
    const res = fetch(url, { signal: abortController.signal });
    abortController.abort();
    if (i % 10000 === 0) {
      console.log(i);
    }
  }
  server.close();
});
