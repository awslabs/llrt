import defaultImport from "node:assert";
import legacyImport from "assert";
import * as legacyNamedImport from "assert";

import assert from "node:assert";

const modules = {
  "node:assert": defaultImport,
  assert: legacyImport,
  "* as assert": legacyNamedImport,
};
for (const module in modules) {
  const { ok } = modules[module];
  describe(module, () => {
    describe(`ok`, () => {
      it("Should be returned 'undefined' (So it's not an error)", () => {
        expect(ok(true)).toBeUndefined(); //bool
        expect(ok(1)).toBeUndefined(); // numeric
        expect(ok("non-empty string")).toBeUndefined(); // string
        expect(ok([])).toBeUndefined(); // array
        expect(ok({})).toBeUndefined(); // object
        expect(ok(() => {})).toBeUndefined(); // function
        expect(ok(123n)).toBeUndefined(); // bigint
        expect(ok(Symbol())).toBeUndefined(); // symbol
        expect(ok(new Error())).toBeUndefined(); // error
        class AssertTestClass {}
        expect(ok(AssertTestClass)).toBeUndefined(); // constructor
      });

      it("Should be returned exception", () => {
        const errMsg =
          "AssertionError: The expression was evaluated to a falsy value";
        expect(() => ok(false)).toThrow(errMsg);
        expect(() => ok(0)).toThrow(errMsg);
        expect(() => ok("")).toThrow(errMsg);
        expect(() => ok(null)).toThrow(errMsg);
      });

      it("should be returned as original error message", () => {
        const errMsg = "Error: Value must be true";
        expect(() => ok(false, errMsg)).toThrow(errMsg);
      });

      it("should be returned as original error", () => {
        const errMsg = "Error: This is error";
        expect(() => ok(false, Error(errMsg))).toThrow(errMsg);
      });

      it("Should be handled correctly even within functions", () => {
        const errMsg = "Error: Value should be truthy";
        function checkValue(value) {
          ok(value, errMsg);
        }
        expect(checkValue(true)).toBeUndefined();
        expect(() => checkValue(false)).toThrow(errMsg);
      });
    });

    it("Should be returned 'undefined' (So it's not an error)", () => {
      expect(assert(true)).toBeUndefined();
      expect(assert(1)).toBeUndefined();
      expect(assert("non-empty string")).toBeUndefined();
      expect(assert([])).toBeUndefined();
      expect(assert({})).toBeUndefined();
    });
  });
}
