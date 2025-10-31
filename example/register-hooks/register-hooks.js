import { registerHooks } from "node:module";
import { readFileSync } from "node:fs";

registerHooks({
  resolve(specifier, context, nextResolve) {
    if (specifier === "fs") {
      return {
        url: "llrt-polyfill:fs",
        shortCircuit: true,
      };
    } else if (specifier.startsWith("internal:")) {
      specifier = specifier.replace("internal:", "");
    }
    return nextResolve(specifier, context);
  },
  load(url, context, nextLoad) {
    if (url === "llrt-polyfill:fs") {
      const code = `
        export * from "internal:fs";
        import fs from "internal:fs";
        export function existsSync(path) {
          try {
            fs.accessSync(path);
            return true;
          } catch {
            return false;
          }
        }
      `;

      return { format: "module", shortCircuit: true, source: code };
    }
    return nextLoad(url, context);
  },
});

registerHooks({
  resolve(specifier, context, nextResolve) {
    if (specifier === "calc") {
      return {
        url: "calc",
        shortCircuit: true,
      };
    }
    return nextResolve(specifier, context);
  },
  load(url, context, nextLoad) {
    if (url === "calc") {
      const code = `
        export function add(p1, p2) {
          return p1 + p2;
        }
      `;

      return { format: "module", shortCircuit: true, source: code };
    }
    return nextLoad(url, context);
  },
});

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
      const code = readFileSync("./http.js");

      return { format: "module", shortCircuit: true, source: code };
    }
    return nextLoad(url, context);
  },
});
