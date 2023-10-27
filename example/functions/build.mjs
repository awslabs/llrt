import esbuild from "esbuild";
import fs from "fs/promises";

const OUTDIR = "build";
const OUT_JS_INDEX = `${OUTDIR}/index.mjs`;

await fs.rm(OUTDIR, { recursive: true, force: true });
await fs.mkdir(OUTDIR);
await fs.copyFile("src/react/index.html", `${OUTDIR}/index.html`);

const devMode = process.argv.slice(2)[0] == "--dev";

await esbuild.build({
  entryPoints: {
    index: "src/ssr.ts",
    app: "src/react/index.tsx",
  },
  logLevel: "info",
  ...(!devMode && {
    platform: "node",
  }),
  external: ["@aws-sdk"],
  target: "es2020",
  format: devMode ? "cjs" : "esm",
  define: {
    "process.env.NODE_ENV": JSON.stringify("production"),
  },
  loader: {
    ".svg": "file",
  },
  bundle: true,
  outdir: OUTDIR,
});

await fs.rename(`${OUTDIR}/index.js`, OUT_JS_INDEX);
await fs.readFile(OUT_JS_INDEX).then((data) => {
  const indexSource = `import { createRequire } from "module";\nconst require = createRequire(import.meta.url);\n${data.toString()}`;
  return fs.writeFile(OUT_JS_INDEX, indexSource);
});
