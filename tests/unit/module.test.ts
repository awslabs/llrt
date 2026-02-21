import defaultImport from "node:module";
import legacyImport from "module";

it("node:module should be the same as module", () => {
  expect(defaultImport).toStrictEqual(legacyImport);
});

const { createRequire } = defaultImport;

it("should resolve 'node:module via createRequire()", () => {
  const __require = createRequire(import.meta.url);
  expect(__require("node:module").createRequire).toBeDefined();
});
