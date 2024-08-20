//test top level await
await new Promise((res) => setTimeout(res, 0));
await import("./hello.js");

export const handler = async () => ({
  statusCode: 200,
  body: "Hello world!",
});
