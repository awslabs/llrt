import { createServer } from "node:http";

const server = createServer((req, res) => {
  res.writeHead(200, { "Content-Type": "text/plain" });
  res.end("Hello Compatible Server");
});

server.listen(3000, () => {
  console.log("Server running at http://localhost:3000/");
});
