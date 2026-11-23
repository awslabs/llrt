import defaultImport from "node:stream";
import legacyImport from "stream";

const { Readable, Writable } = defaultImport;

it("node:stream should be the same as stream", () => {
  expect(defaultImport).toStrictEqual(legacyImport);
});

it("should start the flow of a Readable on once('data')", async () => {
  const readable = new Readable({ read() {} });
  const data = new Promise<Buffer>((resolve) => readable.once("data", resolve));
  readable.push("a");
  expect((await data).toString()).toEqual("a");
  expect(readable.listenerCount("data")).toEqual(0);
});

it("should emit 'error' when a Writable's write calls back with an error synchronously", async () => {
  const error = new Error("write failed");
  const writable = new Writable({
    write(_chunk, _encoding, callback) {
      callback(error);
    },
  });
  const emitted = new Promise((resolve) => writable.on("error", resolve));
  writable.write("a");
  expect(await emitted).toBe(error);
});
