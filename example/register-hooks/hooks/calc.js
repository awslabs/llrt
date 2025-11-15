import { registerHooks } from "node:module";

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
