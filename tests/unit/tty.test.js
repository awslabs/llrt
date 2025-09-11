import defaultImport from "node:tty";
import legacyImport from "tty";

it("node:tty should be the same as tty", () => {
  expect(defaultImport).toStrictEqual(legacyImport);
});
