import * as esbuild from "esbuild";
import fs from "node:fs/promises";
import { createRequire } from "node:module";
import path from "node:path";

const require = createRequire(import.meta.url);

process.env.NODE_PATH = ".";

const TMP_DIR = `.tmp-llrt-aws-sdk`;
const SRC_DIR = path.join("llrt_core", "src", "modules", "js");
const TESTS_DIR = "tests";
const TESTS_SUB_DIR = process.env.TEST_SUB_DIR || "unit";
const OUT_DIR = "bundle/js";
const SHIMS = new Map();
const SDK_BUNDLE_MODE = process.env.SDK_BUNDLE_MODE || "NONE"; // "FULL" or "STD" or "NONE"

async function readFilesRecursive(dir, filePredicate) {
  const dirents = await fs.readdir(dir, { withFileTypes: true });
  const files = await Promise.all(
    dirents.map((dirent) => {
      const filePath = path.join(dir, dirent.name);

      if (dirent.isDirectory()) {
        return readFilesRecursive(filePath, filePredicate);
      } else {
        return filePredicate(filePath) ? filePath : [];
      }
    })
  );
  return Array.prototype.concat(...files);
}

const TEST_FILES = await readFilesRecursive(
  path.join(TESTS_DIR, TESTS_SUB_DIR),
  (filePath) =>
    filePath.endsWith(".test.ts") ||
    filePath.endsWith(".spec.ts") ||
    filePath.endsWith(".any.js")
);
const AWS_JSON_SHARED_COMMAND_REGEX =
  /{\s*const\s*headers\s*=\s*sharedHeaders\(("\w+")\);\s*let body;\s*body\s*=\s*JSON.stringify\(_json\(input\)\);\s*return buildHttpRpcRequest\(context,\s*headers,\s*"\/",\s*undefined,\s*body\);\s*}/gm;
const AWS_JSON_SHARED_COMMAND_REGEX2 =
  /{\s*const\s*headers\s*=\s*sharedHeaders\(("\w+")\);\s*let body;\s*body\s*=\s*JSON.stringify\((\w+)\(input,\s*context\)\);\s*return buildHttpRpcRequest\(context,\s*headers,\s*"\/",\s*undefined,\s*body\);\s*}/gm;
const MINIFY_JS = process.env.JS_MINIFY !== "0";
const SDK_UTILS_PACKAGE = "sdk-utils";
const ENTRYPOINTS = [
  "stream",
  "stream/promises",
  "@llrt/test/index",
  "@llrt/test/worker",
];

const ES_BUILD_OPTIONS = {
  splitting: MINIFY_JS,
  minify: MINIFY_JS,
  sourcemap: false,
  target: "es2023",
  outdir: OUT_DIR,
  bundle: true,
  logLevel: "info",
  platform: "browser",
  format: "esm",
  external: [
    "assert",
    "node:assert",
    "async_hooks",
    "node:async_hooks",
    "buffer",
    "node:buffer",
    "child_process",
    "node:child_process",
    "console",
    "node:console",
    "crypto",
    "node:crypto",
    "dgram",
    "node:dgram",
    "dns",
    "node:dns",
    "events",
    "node:events",
    "fs",
    "node:fs",
    "module",
    "node:module",
    "net",
    "node:net",
    "os",
    "node:os",
    "path",
    "node:path",
    "perf_hooks",
    "node:perf_hooks",
    "process",
    "node:process",
    "stream",
    "node:stream",
    "string_decoder",
    "node:string_decoder",
    "timers",
    "node:timers",
    "tty",
    "node:tty",
    "url",
    "node:url",
    "util",
    "node:util",
    "zlib",
    "node:zlib",
    "llrt:codec",
    "llrt:timezone",
    "llrt:qjs",
    "llrt:util",
    "llrt:xml",
    "@aws-crypto",
  ],
};

const SDK_DATA = await parseSdkData();

const ADDITIONAL_PACKAGES = [
  "@aws-sdk/core",
  "@aws-sdk/core/account-id-endpoint",
  "@aws-sdk/core/client",
  "@aws-sdk/core/protocols",
  "@aws-sdk/core/util",
  "@aws-sdk/credential-providers",
  "@aws-sdk/s3-presigned-post",
  "@aws-sdk/s3-request-presigner",
  "@aws-sdk/util-dynamodb",
  "@smithy/core",
  "@smithy/core/cbor",
  "@smithy/core/checksum",
  "@smithy/core/client",
  "@smithy/core/config",
  "@smithy/core/endpoints",
  "@smithy/core/event-streams",
  "@smithy/core/protocols",
  "@smithy/core/retry",
  "@smithy/core/schema",
  "@smithy/core/serde",
  "@smithy/core/transport",
  "@smithy/fetch-http-handler",
  "@smithy/is-array-buffer",
  "@smithy/middleware-compression",
  "@smithy/signature-v4",
  "@smithy/signature-v4a",
  "@smithy/types",
  "@smithy/util-hex-encoding",
  "@smithy/util-utf8",
];

const REPLACEMENT_PACKAGES = {
  "@aws-crypto/sha1-browser": "shims/@aws-crypto/sha1-browser.js",
  "@aws-crypto/sha256-browser": "shims/@aws-crypto/sha256-browser.js",
  "@aws-crypto/crc32": "shims/@aws-crypto/crc32.js",
  "@aws-crypto/crc32c": "shims/@aws-crypto/crc32c.js",
  "@smithy/abort-controller": "shims/@smithy/abort-controller.js",
};

const SERVICE_ENDPOINTS_BY_PACKAGE = {};
const CLIENTS_BY_SDK = {};
const SDKS_BY_SDK_PACKAGES = {};
const SDK_PACKAGES = [...ADDITIONAL_PACKAGES];

Object.keys(SDK_DATA).forEach((sdk) => {
  const [clientName, serviceEndpoints, fullSdkOnly] = SDK_DATA[sdk] || [];
  if (SDK_BUNDLE_MODE == "FULL" || (SDK_BUNDLE_MODE == "STD" && !fullSdkOnly)) {
    const sdkPackage = `@aws-sdk/${sdk}`;
    SDK_PACKAGES.push(sdkPackage);
    SDKS_BY_SDK_PACKAGES[sdkPackage] = sdk;
    SERVICE_ENDPOINTS_BY_PACKAGE[sdk] = serviceEndpoints;
    CLIENTS_BY_SDK[sdk] = clientName;
  }
});

async function parseSdkData() {
  const cfgData = await fs.readFile("sdk.cfg");
  const cfgLines = cfgData.toString().split("\n");

  const sdkData = {};

  for (let line of cfgLines) {
    line = line.trim();
    if (line.startsWith("#") || line == "") {
      continue;
    }

    // Parse the line
    const parts = line.split(",");

    //get and remove the item at 0
    const packageName = parts.shift();
    const clientName = parts.shift();

    //get and remove the last item
    const fullSdkOnly = parts.pop() == 1;

    const endpoints = parts;

    // Log or store parsed information
    sdkData[packageName] = [clientName, endpoints, fullSdkOnly];
  }
  return sdkData;
}

function resolveDefaultsModeConfigWrapper(config) {
  if (!config.credentials) {
    config.credentials = {
      accessKeyId: process.env.AWS_ACCESS_KEY_ID,
      secretAccessKey: process.env.AWS_SECRET_ACCESS_KEY,
      sessionToken: process.env.AWS_SESSION_TOKEN,
    };
  }
  if (!config.region) {
    config.region = process.env.AWS_REGION;
  }
  return resolveDefaultsModeConfig(config);
}

const awsJsonSharedCommand = (name, input, context, request) => {
  const headers = sharedHeaders(name);
  const body = JSON.stringify(request ? request(input, context) : _json(input));
  return buildHttpRpcRequest(context, headers, "/", undefined, body);
};

function partitionDnsSuffix(region) {
  if (region.startsWith("cn-")) return ["aws-cn", "amazonaws.com.cn"];
  if (region.startsWith("us-gov-")) return ["aws-us-gov", "amazonaws.com"];
  if (region.startsWith("us-iso-")) return ["aws-iso", "c2s.ic.gov"];
  if (region.startsWith("us-isob-")) return ["aws-iso-b", "sc2s.sgov.gov"];
  return ["aws", "amazonaws.com"];
}

function defaultEndpointResolver(endpointParams, context = {}) {
  const {
    Region: region,
    Endpoint: customEndpoint,
    Bucket: bucket,
    UseFIPS: useFips,
    UseDualStack: useDualStack,
    Accelerate: accelerate,
    ForcePathStyle: forcePathStyle,
  } = endpointParams;

  const isS3 = serviceName === "s3";
  const authSchemes = isS3
    ? [
        {
          disableDoubleEncoding: true,
          name: "sigv4",
          signingName,
          signingRegion: region,
        },
      ]
    : undefined;
  const properties = authSchemes ? { authSchemes } : {};

  if (customEndpoint) {
    const url = new URL(customEndpoint);
    if (isS3 && bucket && !forcePathStyle) {
      const path = url.pathname === "/" ? "" : url.pathname;
      url.href = `${url.protocol}//${url.host}/${bucket}${path}${url.search}`;
    }
    return { url, properties, headers: {} };
  }

  if (!region) {
    throw new Error("@llrt/endpoint-resolver: Region is missing");
  }

  const [, dnsSuffix] = partitionDnsSuffix(region);

  if (isS3 && typeof bucket === "string" && bucket.startsWith("arn:")) {
    const resource = bucket.split("accesspoint/")[1];
    if (resource && resource.endsWith(".mrap") && !resource.includes("/")) {
      const alias = resource.slice(0, -".mrap".length);
      return {
        url: new URL(`https://${alias}.mrap.accesspoint.s3-global.${dnsSuffix}/`),
        properties: {
          authSchemes: [
            {
              disableDoubleEncoding: true,
              name: "sigv4a",
              signingName,
              signingRegionSet: ["*"],
            },
          ],
        },
        headers: {},
      };
    }
    throw new Error(
      `@llrt/endpoint-resolver: unsupported S3 ARN bucket "${bucket}". ` +
        "Only multi-region access point (.mrap) ARNs are supported."
    );
  }

  if (isS3 && accelerate) {
    throw new Error(
      "@llrt/endpoint-resolver: S3 transfer acceleration is not supported."
    );
  }

  let host = serviceName;
  if (useFips) host += "-fips";
  if (useDualStack) host += ".dualstack";
  host += `.${region}.${dnsSuffix}`;

  const url = new URL(`https://${host}/`);

  if (isS3 && bucket) {
    url.href = `https://${host}/${bucket}`;
  }

  return { url, properties, headers: {} };
}

const WRAPPERS = [
  {
    name: "resolveDefaultsModeConfig",
    filter: /resolveDefaultsModeConfig(\.browser|\.native)?\.js$/,
    wrapper: resolveDefaultsModeConfigWrapper,
  },
];

function executeClientCommand(command, optionsOrCb, cb) {
  if (typeof optionsOrCb === "function") {
    this.send(command, optionsOrCb);
  } else if (typeof cb === "function") {
    if (typeof optionsOrCb !== "object")
      throw new Error(`Expect http options but get ${typeof optionsOrCb}`);
    this.send(command, optionsOrCb || {}, cb);
  } else {
    return this.send(command, optionsOrCb);
  }
}

const ENDPOINT_CACHE_KEY_LOOKUP = {
  Bucket: "b",
  ForcePathStyle: "f",
  UseArnRegion: "n",
  DisableMultiRegionAccessPoints: "m",
  Accelerate: "a",
  UseGlobalEndpoint: "g",
  UseFIPS: "i",
  Endpoint: "e",
  Region: "r",
  UseDualStack: "d",
};
const ENDPOINT_CACHE_KEY_LOOKUP_NAME = Object.keys({
  ENDPOINT_CACHE_KEY_LOOKUP,
})[0];

function calculateEndpointCacheKey(obj) {
  let str = "";
  for (const key in obj) {
    if (obj[key] === true) {
      str += ENDPOINT_CACHE_KEY_LOOKUP[key];
    } else if (typeof obj[key] === "string") {
      str += obj[key];
    }
  }
  return str;
}

function codeToRegex(fn, includeSignature = false) {
  return new RegExp(
    fn
      .toString()
      .split("\n")
      .reduce((acc, line, index, array) => {
        if (includeSignature || (index > 0 && index < array.length - 1)) {
          acc.push(line.trim());
        }
        return acc;
      }, [])
      .join("\n")
      .replace(/\s+/g, "\\s*")
      .replace(/\(/g, "\\(")
      .replace(/\)/g, "\\)")
      .replace(/\./g, "\\.")
      .replace(/\?,/g, "\\?")
      .replace(/\,/g, ",?")
      .replace(/\$/g, "\\$")
      .replace(/\{/g, "\\s*{")
      .replace(/\}/g, "}\\s*")
      .replace(/\|/g, "\\|"),
    "g"
  );
}

const AWS_SDK_PLUGIN = {
  name: "aws-sdk-plugin",
  setup(build) {
    const tslib = require.resolve("tslib/tslib.es6.js");

    const executeClientCommandRegex = codeToRegex(executeClientCommand);

    build.onResolve({ filter: /^tslib$/ }, () => {
      return { path: tslib };
    });

    //load replace shims
    for (const [filter, contents] of SHIMS) {
      build.onLoad({ filter }, () => ({
        contents,
      }));
    }

    for (const sdk in CLIENTS_BY_SDK) {
      const clientClass = CLIENTS_BY_SDK[sdk];

      build.onLoad(
        { filter: new RegExp(`@aws-sdk\\/${sdk}\\/dist-es/${clientClass}.js`) },
        async ({ path: filePath }) => {
          const source = (await fs.readFile(filePath)).toString();
          const name = path.parse(filePath).name;

          console.log("Optimized:", name);

          let contents = `import { ${executeClientCommand.name} } from "${SDK_UTILS_PACKAGE}"\n`;
          contents += source.replace(
            executeClientCommandRegex,
            `return ${executeClientCommand.name}.call(this, command, optionsOrCb, cb)`
          );

          return {
            contents,
          };
        }
      );
    }

    build.onLoad({ filter: /xml-parser\.browser\.js$/ }, async (args) => {
      const realPath = path.join(path.dirname(args.path), "xml-parser.js");
      const contents = await fs.readFile(realPath, "utf8");
      return { contents, loader: "js" };
    });

    build.onLoad(
      { filter: /protocols\/Aws_json1_1\.js$/ },
      async ({ path: filePath }) => {
        const name = path.parse(filePath).name;

        let source = (await fs.readFile(filePath)).toString();

        const sourceLength = source.length;

        source = source.replace(
          AWS_JSON_SHARED_COMMAND_REGEX,
          (_, name) => `${awsJsonSharedCommand.name}(${name}, input, context)`
        );

        source = source.replace(
          AWS_JSON_SHARED_COMMAND_REGEX2,
          (_, name, request) =>
            `${awsJsonSharedCommand.name}(${name}, input, context, ${request})`
        );

        if (sourceLength === source.length) {
          throw new Error(`Failed to optimize: ${name}`);
        }

        console.log("Optimized:", name);

        source = `const ${
          awsJsonSharedCommand.name
        } = ${awsJsonSharedCommand.toString()}\n\n${source}`;

        return {
          contents: source,
        };
      }
    );

    build.onResolve({ filter: /^sdk-utils$/ }, (args) => ({
      path: args.path,
      namespace: "sdk-utils-ns",
    }));

    build.onLoad({ filter: /.*/, namespace: "sdk-utils-ns" }, (args) => {
      let contents = "";

      contents += `import { Command as $Command } from "@smithy/core/client";\n`;
      contents += `import { getEndpointPlugin } from "@smithy/core/endpoints";\n`;
      contents += `import { getSerdePlugin } from "@smithy/core/serde";\n`;
      contents += `import { SMITHY_CONTEXT_KEY } from "@smithy/types";\n`;
      contents += `export ${executeClientCommand.toString()}\n`;
      contents += `const ${ENDPOINT_CACHE_KEY_LOOKUP_NAME} = ${JSON.stringify(
        ENDPOINT_CACHE_KEY_LOOKUP
      )};\n`;
      contents += `export const cloneModel = (obj) => ({...obj})\n`;
      contents += `export ${calculateEndpointCacheKey.toString()}\n`;

      return {
        contents,
        resolveDir: path.dirname(args.path),
      };
    });

    build.onLoad(
      { filter: /endpoint\/endpointResolver\.js$/ },
      async ({ path: filePath }) => {
        const clientDir = path.resolve(filePath, "../../../").split("/").pop();
        const sdk = clientDir.substring("client-".length);

        const serviceEndpoints = SERVICE_ENDPOINTS_BY_PACKAGE[sdk];
        const serviceName =
          (serviceEndpoints && serviceEndpoints[0]) || sdk;

        let signingName = sdk;
        try {
          const paramsSource = (
            await fs.readFile(path.join(path.dirname(filePath), "EndpointParameters.js"))
          ).toString();
          const m = paramsSource.match(/defaultSigningName:\s*"([^"]*)"/);
          if (m) {
            signingName = m[1];
          }
        } catch {}

        let contents = "";
        contents += `const serviceName = ${JSON.stringify(serviceName)};\n`;
        contents += `const signingName = ${JSON.stringify(signingName)};\n`;
        contents += `export ${partitionDnsSuffix.toString()}\n`;
        contents += `export ${defaultEndpointResolver.toString()}\n`;

        return {
          contents,
        };
      }
    );

    for (const { filter, wrapper, name } of WRAPPERS) {
      build.onLoad({ filter }, async ({ path }) => {
        let source = (await fs.readFile(path)).toString();
        let replaced = false;
        let contents = "";
        source = source.replace(
          RegExp(`export\\s*(const\\s*${name})`),
          (_, replacement) => {
            replaced = true;
            return replacement;
          }
        );
        if (!replaced) {
          contents += source;
        } else {
          const wrapperName = `${name}Wrapper`;
          contents += `${source}\n`;
          contents += `const ${wrapperName} = ${wrapper.toString()}\n`;
          contents += `export {${wrapperName} as ${name}}`;
        }

        return {
          contents,
        };
      });
    }

    build.onLoad({ filter: /package\.json$/ }, async ({ path }) => {
      let packageJson = JSON.parse(await fs.readFile(path));
      let { version } = packageJson;
      const data = {
        version,
      };
      return {
        contents: `export default ${JSON.stringify(data)}`,
      };
    });
  },
};

function esbuildShimPlugin(shims) {
  return {
    name: "esbuild-shim",
    setup(build) {
      shims.forEach(([filter, value], index) => {
        build.onResolve(
          {
            filter,
          },
          (args) => ({
            path: args.path,
            namespace: `esbuild-shim-${index}-ns`,
          })
        );
        build.onLoad(
          { filter: /.*/, namespace: `esbuild-shim-${index}-ns` },
          () => {
            const contents = value || "export default {}";
            return {
              contents,
            };
          }
        );
      });
    },
  };
}

const requireProcessPlugin = {
  name: "require-process",
  setup(build) {
    build.onResolve({ filter: /^process\/$/ }, () => {
      return { path: "process", external: true };
    });
  },
};

async function rmTmpDir() {
  await fs.rm(TMP_DIR, {
    recursive: true,
    force: true,
  });
}

async function createOutputDirectories() {
  await fs.rm(OUT_DIR, { recursive: true, force: true });
  await fs.mkdir(OUT_DIR, { recursive: true });
  await rmTmpDir();
  await fs.mkdir(TMP_DIR, { recursive: true });
}

async function loadShims() {
  const loadShim = async (filter, filename) => {
    const bytes = await fs.readFile(path.join("shims", filename));
    SHIMS.set(filter, bytes.toString());
  };

  await Promise.all([
    loadShim(/@aws-crypto/, "@aws-crypto/index.js"),
    loadShim(/@smithy\/util-hex-encoding/, "@smithy/util-hex-encoding.js"),
    loadShim(/@smithy\/util-utf8/, "@smithy/util-utf8.js"),
    loadShim(/stringHasher.js/, "string-hasher.js"),
    loadShim(/@smithy\/util-base64/, "@smithy/util-base64.js"),
    loadShim(/mnemonist\/lru-cache\.js/, "mnemonist/lru-cache.js"),
  ]);
}

async function buildLibrary() {
  const defaultLibEsBuildOption = {
    chunkNames: "llrt-[name]-runtime-[hash]",
    ...ES_BUILD_OPTIONS,
    splitting: false,
    keepNames: true,
    nodePaths: ["."],
  };

  // Build lib
  const entryPoints = {};
  ENTRYPOINTS.forEach((entry) => {
    entryPoints[entry] = path.join(SRC_DIR, entry);
  });
  await esbuild.build({
    ...defaultLibEsBuildOption,
    entryPoints,
    plugins: [requireProcessPlugin],
    sourcemap: false,
  });

  // Build tests
  const testEntryPoints = TEST_FILES.reduce((acc, entry) => {
    const { name, dir } = path.parse(entry);
    const parentDir = path.basename(dir);
    acc[path.join("__tests__", parentDir, name)] = entry;
    return acc;
  }, {});

  await esbuild.build({
    ...defaultLibEsBuildOption,
    entryPoints: testEntryPoints,
    external: [...ES_BUILD_OPTIONS.external, "@aws-sdk", "@smithy"],
    sourcemap: false,
  });
}

async function buildSdks() {
  const sdkEntryList = await Promise.all(
    SDK_PACKAGES.map(async (pkg) => {
      const packagePath = path.join(TMP_DIR, pkg);
      const sdk = SDKS_BY_SDK_PACKAGES[pkg];
      const sdkIndexFile = path.join(packagePath, "index.js");

      await fs.mkdir(packagePath, { recursive: true });

      let sdkContents = `export * from "${pkg}";`;
      await fs.writeFile(sdkIndexFile, sdkContents);

      return [pkg, sdkIndexFile];
    })
  );

  const sdkEntryPoints = Object.fromEntries(sdkEntryList);

  await Promise.all([
    esbuild.build({
      entryPoints: sdkEntryPoints,
      plugins: [
        AWS_SDK_PLUGIN,
        esbuildShimPlugin([[/^bowser$/]]),
        {
          name: "llrt-stream-compat",
          setup(build) {
            build.onResolve(
              { filter: /getAwsChunkedEncodingStream/ },
              (args) => {
                if (args.importer.includes("@smithy")) {
                  return {
                    path: path.resolve(
                      "shims/@smithy/getAwsChunkedEncodingStream.js"
                    ),
                  };
                }
              }
            );
          },
        },
      ],
      alias: {
        "@aws-sdk/util-utf8-browser": "@smithy/util-utf8",
        "@aws-sdk/util-utf8": "@smithy/util-utf8",
        "@aws-sdk/signature-v4-multi-region": path.resolve(
          "shims/@aws-sdk/signature-v4-multi-region.js"
        ),
        "@smithy/md5-js": "crypto",
        "@aws-sdk/credential-providers": path.dirname(
          require.resolve("@aws-sdk/credential-providers/package.json")
        ) + "/dist-es/index.browser.js",
        "fast-xml-parser": "llrt:xml",
        "xml-parser.browser": "xml-parser",
      },
      chunkNames: "llrt-[name]-sdk-[hash]",
      metafile: true,
      ...ES_BUILD_OPTIONS,
    }),
    esbuild.build({
      entryPoints: REPLACEMENT_PACKAGES,
      ...ES_BUILD_OPTIONS,
      sourcemap: false,
    }),
  ]);

  //console.log(await esbuild.analyzeMetafile(result.metafile));
}

console.log("Building...");

await createOutputDirectories();
let error;
try {
  if (SDK_BUNDLE_MODE != "NONE") {
    await loadShims();
  }

  await buildLibrary();

  if (SDK_BUNDLE_MODE != "NONE") {
    await buildSdks();
  }
} catch (e) {
  error = e;
}

await rmTmpDir();

if (error) {
  throw error;
}
