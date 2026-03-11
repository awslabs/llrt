import defaultImport from "node:stream";
import legacyImport from "stream";

it("node:stream should be the same as stream", () => {
  expect(defaultImport).toStrictEqual(legacyImport);
});
