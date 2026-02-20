import defaultImport from "node:os";
import legacyImport from "os";

it("node:os should be the same as os", () => {
  expect(defaultImport).toStrictEqual(legacyImport);
});
