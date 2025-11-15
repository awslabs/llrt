import { registerHooks } from "node:module";
import { readFileSync } from "node:fs";

registerHooks({
  resolve(specifier, context, nextResolve) {
    if (specifier === "http") {
      return {
        url: "http",
        shortCircuit: true,
      };
    }
    return nextResolve(specifier, context);
  },
  load(url, context, nextLoad) {
    if (url === "http") {
      const code = readFileSync("./src/http.js");

      return { format: "module", shortCircuit: true, source: code };
    }
    return nextLoad(url, context);
  },
});
