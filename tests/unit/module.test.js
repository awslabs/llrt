import defaultImport from "node:module";
import legacyImport from "module";

it("node:module should be the same as module", () => {
  expect(defaultImport).toStrictEqual(legacyImport);
});
