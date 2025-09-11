import defaultImport from "node:net";
import legacyImport from "net";

it("node:net should be the same as net", () => {
  expect(defaultImport).toStrictEqual(legacyImport);
});
