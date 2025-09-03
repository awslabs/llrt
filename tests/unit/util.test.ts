import defaultImport from "node:util";
import legacyImport from "util";
import * as legacyNamedImport from "util";

import { EventEmitter } from "node:events";

const modules = {
  "node:util": defaultImport,
  util: legacyImport,
  "* as util": legacyNamedImport,
};
for (const module in modules) {
  const { inherits } = modules[module];

  describe(module, () => {
    describe("inherits", () => {
      it("should be inheritable parent classes", () => {
        function MyStream() {
          EventEmitter.call(this);
        }

        inherits(MyStream, EventEmitter);

        const stream = new MyStream();

        expect(stream instanceof EventEmitter).toBeTruthy();
        expect(MyStream.super_).toEqual(EventEmitter);
      });
    });
  });
}
