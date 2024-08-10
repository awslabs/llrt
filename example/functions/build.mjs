import esbuild from "esbuild";
import fs from "fs/promises";
import path from "path";

const OUTDIR = "build";

await fs.rm(OUTDIR, { recursive: true, force: true });

async function buildReact() {
  const outbase = path.join(OUTDIR, "react");
  const outfile = path.join(outbase, "index.mjs");

  await fs.mkdir(outbase, { recursive: true });
  await fs.copyFile("src/react/index.html", path.join(outbase, "index.html"));

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
    target: "es2023",
    format: devMode ? "cjs" : "esm",
    define: {
      "process.env.NODE_ENV": JSON.stringify("production"),
    },
    loader: {
      ".svg": "file",
    },
    bundle: true,
    outdir: outbase,
  });

  await fs.rename(path.join(outbase, "index.js"), outfile);
  await fs.readFile(outfile).then((data) => {
    const indexSource = `import { createRequire } from "module";\nconst require = createRequire(import.meta.url);\n${data.toString()}`;
    return fs.writeFile(outfile, indexSource);
  });
}

async function buildExternalSdkFunction() {
  const outbase = path.join(OUTDIR, "external");
  const outfile = path.join(outbase, "index.mjs");

  await esbuild.build({
    entryPoints: {
      index: "src/non-included-sdk.mjs",
    },
    logLevel: "info",
    platform: "browser",
    target: "es2023",
    format: "esm",
    bundle: true,
    minify: true,
    sourcemap: false,
    outfile,
    external: [
      "@smithy",
      "@aws-sdk/core",
      "@aws-sdk/util-user-agent-browser",
      "@aws-crypto",
      "bowser",
    ],
  });
}

await buildReact();
await buildExternalSdkFunction();
