import * as esbuild from "esbuild";
import fs from "fs/promises";
import { createRequire } from "module";
import path from "path";

const require = createRequire(import.meta.url);

process.env.NODE_PATH = ".";

const TMP_DIR = `.tmp-llrt-aws-sdk`;
const SRC_DIR = path.join("llrt_core", "src", "modules", "js");
const TESTS_DIR = "tests";
const OUT_DIR = "bundle/js";
const SHIMS = new Map();

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
  TESTS_DIR,
  (filePath) => filePath.endsWith(".test.ts") || filePath.endsWith(".spec.ts")
);
const AWS_JSON_SHARED_COMMAND_REGEX =
  /{\s*const\s*headers\s*=\s*sharedHeaders\(("\w+")\);\s*let body;\s*body\s*=\s*JSON.stringify\(_json\(input\)\);\s*return buildHttpRpcRequest\(context,\s*headers,\s*"\/",\s*undefined,\s*body\);\s*}/gm;
const AWS_JSON_SHARED_COMMAND_REGEX2 =
  /{\s*const\s*headers\s*=\s*sharedHeaders\(("\w+")\);\s*let body;\s*body\s*=\s*JSON.stringify\((\w+)\(input,\s*context\)\);\s*return buildHttpRpcRequest\(context,\s*headers,\s*"\/",\s*undefined,\s*body\);\s*}/gm;
const MINIFY_JS = process.env.JS_MINIFY !== "0";
const SDK_UTILS_PACKAGE = "sdk-utils";
const ENTRYPOINTS = ["@llrt/std", "stream", "@llrt/test"];

const ES_BUILD_OPTIONS = {
  splitting: MINIFY_JS,
  minify: MINIFY_JS,
  sourcemap: true,
  target: "es2023",
  outdir: OUT_DIR,
  bundle: true,
  logLevel: "info",
  platform: "browser",
  format: "esm",
  external: [
    "console",
    "node:console",
    "crypto",
    "os",
    "fs",
    "child_process",
    "process",
    "timers",
    "stream",
    "path",
    "events",
    "buffer",
    "net",
    "util",
    "url",
    "zlib",
    "llrt:hex",
    "llrt:uuid",
    "llrt:xml",
    "perf_hooks",
  ],
};

// API Endpoint Definitions
//
// const SDK_DATA = {
//   // Classification
//   ...BUNDLE_TARGET({ "PackageName": ["ClientName", ["ServiceEndpoints", ...]] }),
// }
//
// The meanings of each and how to look them up are as follows.
//
//   1. Classification
//   https://docs.aws.amazon.com/whitepapers/latest/aws-overview/amazon-web-services-cloud-platform.html
//
//   2. BUNDLE_TARGET
//
//     It is like a macro when switching the bundle target according to the environment variable SDK_BUNDLE_MODE.
//
//     FULL : The bundle is eligible when SDK_BUNDLE_MODE is 'FULL'.
//     STD : The bundle is eligible when SDK_BUNDLE_MODE is 'STD' or 'FULL'.
//
//     Note that if SDK_BUNDLE_MODE is 'NONE', the bundle will not be bundled even if BUNDLE_TARGET is set to 'STD' or 'FULL'.
//
//   3. ClientName
//   https://www.npmjs.com/search?q=%40aws-sdk%2Fclient-
//
//    Case) @aws-sdk/client-sts
//    [Import Section](https://www.npmjs.com/package/@aws-sdk/client-sts#getting-started)
//
//    > // ES6+ example
//    > import { STSClient, GetCallerIdentityCommand } from "@aws-sdk/client-sts";
//               ^^^ <- This part except for the "client"
//
//   4. ServiceEndpoints
//   https://docs.aws.amazon.com/general/latest/gr/aws-service-information.html
//
//    Case) @aws-sdk/client-sts
//    [Service endpoints Section](https://docs.aws.amazon.com/general/latest/gr/sts.html#sts_region)
//
//    > | Region Name    | Region    | Endpoint                    | Protocol |
//    > | -------------- | --------- | --------------------------- | -------- |
//    > | US East (Ohio) | us-east-2 | sts.us-east-2.amazonaws.com | HTTPS    |
//                                     ^^^ <- String before "region"
//
//    If multiple endpoints are required, such as @aws-sdk/client-sso, register multiple endpoints in the array.
//
const SDK_BUNDLE_MODE = process.env.SDK_BUNDLE_MODE || "NONE"; // "FULL" or "STD" or "NONE"

const _FULL = SDK_BUNDLE_MODE == "FULL";
const _STD = _FULL || SDK_BUNDLE_MODE == "STD";
const FULL = (arg) => (_FULL ? arg : {});
const STD = (arg) => (_STD ? arg : Array.isArray(arg) ? [] : {});

const SDK_DATA = {
  // Analytics
  ...FULL({ "client-athena": ["Athena", ["athena"]] }),
  ...FULL({ "client-firehose": ["Firehose", ["firehose"]] }),
  ...FULL({ "client-glue": ["Glue", ["glue"]] }),
  ...FULL({ "client-kinesis": ["Kinesis", ["kinesis"]] }),
  ...FULL({ "client-opensearch": ["OpenSearch", ["es"]] }),
  ...FULL({
    "client-opensearchserverless": ["OpenSearchServerless", ["aoss"]],
  }),
  // ApplicationIntegration
  ...STD({ "client-eventbridge": ["EventBridge", ["events"]] }),
  ...FULL({ "client-scheduler": ["Scheduler", ["scheduler"]] }),
  ...STD({ "client-sfn": ["SFN", ["states", "sync-states"]] }),
  ...STD({ "client-sns": ["SNS", ["sns"]] }),
  ...STD({ "client-sqs": ["SQS", ["sqs"]] }),
  // Blockchain
  ...{},
  // BusinessApplications
  ...STD({ "client-ses": ["SES", ["email"]] }),
  ...FULL({ "client-sesv2": ["SESv2", ["email"]] }),
  // CloudFinancialManagement
  ...{},
  // ComputeServices
  ...FULL({ "client-auto-scaling": ["AutoScaling", ["autoscaling"]] }),
  ...FULL({ "client-batch": ["Batch", ["batch"]] }),
  ...FULL({ "client-ec2": ["EC2", ["ec2"]] }),
  ...STD({ "client-lambda": ["Lambda", ["lambda"]] }),
  // CustomerEnablement
  ...{},
  // Containers
  ...FULL({ "client-ecr": ["ECR", ["ecr", "api.ecr"]] }),
  ...FULL({ "client-ecs": ["ECS", ["ecs"]] }),
  ...FULL({ "client-eks": ["EKS", ["eks"]] }),
  ...FULL({ "client-servicediscovery": ["ServiceDiscovery", ["discovery"]] }),
  // Databases
  ...STD({ "client-dynamodb": ["DynamoDB", ["dynamodb"]] }),
  ...FULL({
    "client-dynamodb-streams": ["DynamoDBStreams", ["streams.dynamodb"]],
  }),
  ...FULL({ "client-elasticache": ["ElastiCache", ["elasticache"]] }),
  ...FULL({ "client-rds": ["RDS", ["rds"]] }),
  ...FULL({ "client-rds-data": ["RDSData", ["rds-data"]] }),
  ...STD({ "lib-dynamodb": ["DynamoDBDocument", ["dynamodb"]] }),
  // DeveloperTools
  ...STD({ "client-xray": ["XRay", ["xray"]] }),
  // EndUserComputing
  ...{},
  // FrontendWebAndMobileServices
  ...FULL({ "client-amplify": ["Amplify", ["amplify"]] }),
  ...FULL({ "client-appsync": ["AppSync", ["appsync"]] }),
  ...FULL({ "client-location": ["Location", ["geo"]] }),
  // GameTech
  ...{},
  // InternetOfThings
  ...{},
  // MachineLearningAndArtificialIntelligence
  ...FULL({ "client-bedrock": ["Bedrock", ["bedrock"]] }),
  ...FULL({ "client-bedrock-agent": ["BedrockAgent", ["bedrock-agent"]] }),
  ...FULL({
    "client-bedrock-runtime": ["BedrockRuntime", ["bedrock-runtime"]],
  }),
  ...FULL({
    "client-bedrock-agent-runtime": [
      "BedrockAgentRuntime",
      ["bedrock-agent-runtime"],
    ],
  }),
  ...FULL({ "client-polly": ["Polly", ["polly"]] }),
  ...FULL({ "client-rekognition": ["Rekognition", ["rekognition"]] }),
  ...FULL({ "client-textract": ["Textract", ["textract"]] }),
  ...FULL({ "client-translate": ["Translate", ["translate"]] }),
  // ManagementAndGovernance
  ...FULL({ "client-appconfig": ["AppConfig", ["appconfig"]] }),
  ...FULL({ "client-appconfigdata": ["AppConfigData", ["appconfigdata"]] }),
  ...FULL({ "client-cloudformation": ["CloudFormation", ["cloudformation"]] }),
  ...FULL({ "client-cloudwatch": ["CloudWatch", ["monitoring"]] }),
  ...STD({ "client-cloudwatch-logs": ["CloudWatchLogs", ["logs"]] }),
  ...STD({ "client-cloudwatch-events": ["CloudWatchEvents", ["events"]] }),
  ...FULL({ "client-service-catalog": ["ServiceCatalog", ["servicecatalog"]] }),
  ...STD({ "client-ssm": ["SSM", ["ssm"]] }),
  // Media
  ...FULL({ "client-mediaconvert": ["MediaConvert", ["mediaconvert"]] }),
  // MigrationAndTransfer
  ...{},
  // NetworkingAndContentDelivery
  ...FULL({ "client-api-gateway": ["APIGateway", ["apigateway"]] }),
  ...FULL({ "client-apigatewayv2": ["ApiGatewayV2", ["apigateway"]] }),
  ...FULL({
    "client-elastic-load-balancing-v2": [
      "ElasticLoadBalancingV2",
      ["elasticloadbalancing"],
    ],
  }),
  // QuantumTechnologies
  ...{},
  // Robotics
  ...{},
  // Satellite
  ...{},
  // SecurityIdentityAndCompliance
  ...FULL({ "client-acm": ["ACM", ["acm"]] }),
  ...STD({
    "client-cognito-identity": ["CognitoIdentity", ["cognito-identity"]],
  }),
  ...STD({
    "client-cognito-identity-provider": [
      "CognitoIdentityProvider",
      ["cognito-idp"],
    ],
  }),
  ...FULL({ "client-iam": ["IAM", ["iam"]] }),
  ...STD({ "client-kms": ["KMS", ["kms"]] }),
  ...STD({ "client-secrets-manager": ["SecretsManager", ["secretsmanager"]] }),
  ...FULL({ "client-sso": ["SSO", ["sso", "identitystore"]] }),
  ...FULL({ "client-sso-admin": ["SSOAdmin", ["sso", "identitystore"]] }),
  ...FULL({ "client-sso-oidc": ["SSOOIDC", ["sso", "identitystore"]] }),
  ...STD({ "client-sts": ["STS", ["sts"]] }),
  // Storage
  ...FULL({ "client-efs": ["EFS", ["elasticfilesystem"]] }),
  ...STD({ "client-s3": ["S3", ["s3"]] }),
  ...STD({ "lib-storage": ["Upload", ["s3"]] }),
};

const ADDITIONAL_PACKAGES = STD([
  "@aws-sdk/core",
  "@aws-sdk/credential-providers",
  "@aws-sdk/s3-presigned-post",
  "@aws-sdk/s3-request-presigner",
  "@aws-sdk/util-dynamodb",
  "@aws-sdk/util-user-agent-browser",
  "@smithy/config-resolver",
  "@smithy/core",
  "@smithy/eventstream-codec",
  "@smithy/eventstream-serde-browser",
  "@smithy/eventstream-serde-config-resolver",
  "@smithy/eventstream-serde-universal",
  "@smithy/fetch-http-handler",
  "@smithy/invalid-dependency",
  "@smithy/is-array-buffer",
  "@smithy/middleware-compression",
  "@smithy/middleware-content-length",
  "@smithy/middleware-endpoint",
  "@smithy/middleware-retry",
  "@smithy/middleware-serde",
  "@smithy/middleware-stack",
  "@smithy/property-provider",
  "@smithy/protocol-http",
  "@smithy/querystring-builder",
  "@smithy/querystring-parser",
  "@smithy/service-error-classification",
  "@smithy/signature-v4",
  "@smithy/smithy-client",
  "@smithy/types",
  "@smithy/url-parser",
  "@smithy/util-base64",
  "@smithy/util-body-length-browser",
  "@smithy/util-config-provider",
  "@smithy/util-defaults-mode-browser",
  "@smithy/util-endpoints",
  "@smithy/util-hex-encoding",
  "@smithy/util-middleware",
  "@smithy/util-retry",
  "@smithy/util-stream",
  "@smithy/util-uri-escape",
  "@smithy/util-utf8",
  "@smithy/util-waiter",
]);

const REPLACEMENT_PACKAGES = STD({
  "@aws-crypto/sha1-browser": "shims/@aws-crypto/sha1-browser.js",
  "@aws-crypto/sha256-browser": "shims/@aws-crypto/sha256-browser.js",
  "@aws-crypto/crc32": "shims/@aws-crypto/crc32.js",
  "@aws-crypto/crc32c": "shims/@aws-crypto/crc32c.js",
  "@smithy/abort-controller": "shims/@smithy/abort-controller.js",
});

const SERVICE_ENDPOINTS_BY_PACKAGE = {};
const CLIENTS_BY_SDK = {};
const SDKS_BY_SDK_PACKAGES = {};
const SDK_PACKAGES = [...ADDITIONAL_PACKAGES];

Object.keys(SDK_DATA).forEach((sdk) => {
  const [clientName, serviceEndpoints] = SDK_DATA[sdk] || [];
  const sdkPackage = `@aws-sdk/${sdk}`;
  SDK_PACKAGES.push(sdkPackage);
  SDKS_BY_SDK_PACKAGES[sdkPackage] = sdk;
  SERVICE_ENDPOINTS_BY_PACKAGE[sdk] = serviceEndpoints;
  CLIENTS_BY_SDK[sdk] = clientName;
});

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

function defaultEndpointResolver(endpointParams, context = {}) {
  const paramsKey = calculateEndpointCacheKey(endpointParams);
  let endpoint = ENDPOINT_CACHE[paramsKey];
  if (!endpoint) {
    endpoint = resolveEndpoint(ruleSet, {
      endpointParams,
      logger: context.logger,
      serviceName,
    });
    ENDPOINT_CACHE[paramsKey] = endpoint;
  }

  if (serviceName === "s3") {
    const { hostname, protocol, pathname, search } = endpoint.url;
    const [bucket, host] = hostname.split(".s3.");
    if (host) {
      const newHref = `${protocol}//s3.${host}/${bucket}${pathname}${search ? `?${search}` : ""
        }`;
      endpoint.url.href = newHref;
    }
  }

  return endpoint;
}

const WRAPPERS = [
  {
    name: "resolveDefaultsModeConfig",
    filter: /resolveDefaultsModeConfig.js$/,
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

        source = `const ${awsJsonSharedCommand.name
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

      contents += `import { Command as $Command } from "@smithy/smithy-client";\n`;
      contents += `import { getEndpointPlugin } from "@smithy/middleware-endpoint";\n`;
      contents += `import { getSerdePlugin } from "@smithy/middleware-serde";\n`;
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
        let source = (await fs.readFile(filePath)).toString();
        source = source.replace(
          /export const defaultEndpointResolver =.*?};/s,
          ""
        );
        let contents = `import { ${calculateEndpointCacheKey.name} } from "${SDK_UTILS_PACKAGE}"\n`;
        contents += source;
        const serviceName = path
          .resolve(filePath, "../../../")
          .split("/")
          .pop()
          .substring("client-".length);
        contents += `const serviceName = "${serviceName}";\n`;
        contents += `const ENDPOINT_CACHE = {};\n`;
        contents += `export ${defaultEndpointResolver.toString()}`;

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
    loadShim(/@smithy\/util-base64/, "@smithy/util-base64.js"),
    loadShim(/mnemonist\/lru-cache\.js/, "mnemonist/lru-cache.js"),
    loadShim(/collect-stream-body\.js/, "collect-stream-body.js"),
    loadShim(/sdk-stream-mixin.browser\.js/, "sdk-stream-mixin.js"),
    loadShim(/stream-collector\.js/, "stream-collector.js"),
    loadShim(/splitStream.browser\.js/, "@smithy/split-stream.js"),
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
  });
}

async function buildSdks() {
  const sdkEntryList = await Promise.all(
    SDK_PACKAGES.map(async (pkg) => {
      const packagePath = path.join(TMP_DIR, pkg);
      const sdk = SDKS_BY_SDK_PACKAGES[pkg];
      const sdkIndexFile = path.join(packagePath, "index.js");
      const serviceNames = SERVICE_ENDPOINTS_BY_PACKAGE[sdk];

      await fs.mkdir(packagePath, { recursive: true });

      let sdkContents = `export * from "${pkg}";`;
      if (serviceNames) {
        for (const serviceName of serviceNames) {
          sdkContents += `\nif(__bootstrap.addAwsSdkInitTask){\n   __bootstrap.addAwsSdkInitTask("${serviceName}");}`;
        }
        sdkContents += "\n";
      }
      await fs.writeFile(sdkIndexFile, sdkContents);

      return [pkg, sdkIndexFile];
    })
  );

  const sdkEntryPoints = Object.fromEntries(sdkEntryList);

  await Promise.all([
    esbuild.build({
      entryPoints: sdkEntryPoints,
      plugins: [AWS_SDK_PLUGIN, esbuildShimPlugin([[/^bowser$/]])],
      alias: {
        "@aws-sdk/util-utf8-browser": "@smithy/util-utf8",
        "@aws-sdk/util-utf8": "@smithy/util-utf8",
        "@smithy/md5-js": "crypto",
        "fast-xml-parser": "llrt:xml",
        uuid: "llrt:uuid",
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
  await loadShims();
  await buildLibrary();
  await buildSdks();
} catch (e) {
  error = e;
}

await rmTmpDir();

if (error) {
  throw error;
}
