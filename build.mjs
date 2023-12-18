import { build as esbuild } from "esbuild";
import { spawn } from "child_process";
import fs from "fs/promises";
import { createRequire } from "module";
import path from "path";

//reload itself with a loader
if (!process.env.__LLRT_BUILD) {
  const [node, ...args] = process.argv;

  await new Promise((resolve, reject) => {
    const child = spawn(
      node,
      ["--no-warnings", "--loader", "./esm.mjs", ...args],
      {
        stdio: "inherit",
        env: {
          ...process.env,
          __LLRT_BUILD: "1",
        },
      }
    );
    child.on("exit", process.exit);
    child.on("error", reject);
  });
}

const require = createRequire(import.meta.url);

process.env.NODE_PATH = ".";

const TMP_DIR = `.tmp-llrt-aws-sdk`;
const SRC_DIR = path.join("src", "js");
const TESTS_DIR = "tests";
const OUT_DIR = "bundle";
const SHIMS = new Map();
const TEST_FILES = await fs.readdir(TESTS_DIR);
const SPREAD_MODEL_REGEX = /\(\w*\)\s*=>\s*\({\s*\.\.\.\w*,?\s*}\)/g;
const PROPERTY_NOOP_ARROW_FUNCTION_REGEX = /(\w+:)\s*\(_\)\s*=>\s*_/;
const NOOP_ARROW_FUNCTION_REGEX = /\(_\)\s*=>\s*_/;
const SHARED_COMMAND_REGEX =
  /this\.middlewareStack\.use\(\s*getSerdePlugin\(configuration,\s*this\.serialize,\s*this\.deserialize\)\s*\);\s*this\.middlewareStack\.use\(\s*getEndpointPlugin\(\s*configuration,\s*\w+\.getEndpointParameterInstructions\(\)\s*\)\s*\);\s*const\s*stack\s*=\s*clientStack\.concat\(this\.middlewareStack\);\s*const\s*{\s*logger\s*}\s*=\s*configuration;\s*const\s*clientName\s*=\s*("\w+");\s*const\s*commandName\s*=\s*("\w+");\s*const\s*handlerExecutionContext\s*=\s*{\s*logger,\s*clientName,\s*commandName,\s*inputFilterSensitiveLog:\s*(\w+|\(_\) => _),\s*outputFilterSensitiveLog:\s*(\w+|\(_\) => _),?\s*,\s*\[SMITHY_CONTEXT_KEY]:\s*{\s*service:\s*("\w+"),\s*operation:\s*("\w+"),\s*},\s*};\s*const\s*{\s*requestHandler\s*}\s*=\s*configuration;\s*return\s*stack\.resolve\(\s*\(request\)\s*=>\s*requestHandler\.handle\(request\.request,\s*options\s*\|\|\s*{}\),\s*handlerExecutionContext\s*\);/gm;
const SHARED_COMMAND_MARSHALL_REGEX = /return (\w+)\(/;
const AWS_JSON_SHARED_COMMAND_REGEX =
  /{\s*const\s*headers\s*=\s*sharedHeaders\(("\w+")\);\s*let body;\s*body\s*=\s*JSON.stringify\(_json\(input\)\);\s*return buildHttpRpcRequest\(context,\s*headers,\s*"\/",\s*undefined,\s*body\);\s*}/g;
const MINIFY_JS = process.env.JS_MINIFY !== "0";
const SDK_UTILS_PACKAGE = "sdk-utils";
const ENTRYPOINTS = ["@llrt/std", "stream", "@llrt/runtime", "@llrt/test"];
const COMMANDS_BY_SDK = {};

const ES_BUILD_OPTIONS = {
  splitting: MINIFY_JS,
  minify: MINIFY_JS,
  sourcemap: true,
  target: "es2020",
  outdir: OUT_DIR,
  bundle: true,
  logLevel: "info",
  platform: "browser",
  format: "esm",
  external: [
    "crypto",
    "uuid",
    "hex",
    "os",
    "fs/promises",
    "child_process",
    "timers",
    "stream",
    "path",
    "events",
    "buffer",
    "xml",
    "net",
  ],
};

const SDK_DATA = {
  "client-dynamodb": ["DynamoDB", "dynamodb"],
  "lib-dynamodb": ["DynamoDBDocument", "dynamodb"],
  "client-kms": ["KMS", "kms"],
  "client-lambda": ["Lambda", "lambda"],
  "client-s3": ["S3", "s3"],
  "client-secrets-manager": ["SecretsManager", "secretsmanager"],
  "client-ses": ["SES", "email"],
  "client-sns": ["SNS", "sns"],
  "client-sqs": ["SQS", "sqs"],
  "client-sts": ["STS", "sts"],
  "client-ssm": ["SSM", "ssm"],
  "client-cloudwatch-logs": ["CloudWatchLogs", "logs"],
  "client-cloudwatch-events": ["CloudWatchEvents", "events"],
  "client-eventbridge": ["EventBridge", "events"],
  "client-sfn": ["SFN", "sfn"],
  "client-xray": ["XRay", "xray"],
  "client-cognito-identity": ["CognitoIdentity", "cognito-idp"],
};

const ADDITIONAL_PACKAGES = [
  "@aws-sdk/util-dynamodb",
  "@smithy/signature-v4",
  "@aws-sdk/credential-providers",
];

const SDKS = [];
const SERVICE_ENDPOINT_BY_PACKAGE = {};
const CLIENTS_BY_SDK = {};
const SDKS_BY_SDK_PACKAGES = {};
const SDK_PACKAGES = [...ADDITIONAL_PACKAGES];

Object.keys(SDK_DATA).forEach((sdk) => {
  const [clientName, serviceEndpoint] = SDK_DATA[sdk] || [];
  const sdkPackage = `@aws-sdk/${sdk}`;
  SDKS.push(sdk);
  SDK_PACKAGES.push(sdkPackage);
  SDKS_BY_SDK_PACKAGES[sdkPackage] = sdk;
  SERVICE_ENDPOINT_BY_PACKAGE[sdk] = serviceEndpoint;
  CLIENTS_BY_SDK[sdk] = clientName;
});

const camelToSnakeCase = (str) =>
  str.replace(/[A-Z]/g, (letter) => `_${letter}`).toUpperCase();

class $Command {}
class BaseCommand extends $Command {
  constructor(
    input,
    clientName,
    commandName,
    inputFilterSensitiveLogFn,
    outputFilterSensitiveLogFn,
    serializeFn,
    deserializeFn,
    service,
    operation,
    paramInstructions
  ) {
    super();
    this.input = input;
    this.clientName = clientName;
    this.commandName = commandName;
    this.serializeFn = serializeFn;
    this.deserializeFn = deserializeFn;
    this.inputFilterSensitiveLogFn = inputFilterSensitiveLogFn;
    this.outputFilterSensitiveLogFn = outputFilterSensitiveLogFn;
    this.paramInstructions = paramInstructions;
    this.service = service;
    this.operation = operation;
  }

  static getEndpointParameterInstructions() {
    return defaultEndpointParameterInstructions();
  }

  resolveMiddleware(clientStack, configuration, options) {
    this.middlewareStack.use(
      getSerdePlugin(configuration, this.serializeFn, this.deserializeFn)
    );
    this.middlewareStack.use(
      getEndpointPlugin(configuration, this.paramInstructions)
    );
    const stack = clientStack.concat(this.middlewareStack);
    const { logger } = configuration;
    const handlerExecutionContext = {
      logger,
      clientName: this.clientName,
      commandName: this.commandName,
      inputFilterSensitiveLog: this.inputFilterSensitiveLogFn,
      outputFilterSensitiveLog: this.outputFilterSensitiveLogFn,
      [SMITHY_CONTEXT_KEY]: {
        service: this.service,
        operation: this.operation,
      },
    };
    const { requestHandler } = configuration;
    return stack.resolve(
      (request) => requestHandler.handle(request.request, options || {}),
      handlerExecutionContext
    );
  }

  serialize(input, context) {
    return this.serializeFn(input, context);
  }

  deserialize(output, context) {
    return this.deserializeFn(output, context);
  }
}
class S3Command extends BaseCommand {
  constructor(...args) {
    super(...args, S3Command.getEndpointParameterInstructions());
  }
  static getEndpointParameterInstructions() {
    return s3DefaultEndpointParameterInstructions();
  }
}

class DefaultCommand extends BaseCommand {
  constructor(...args) {
    super(...args, DefaultCommand.getEndpointParameterInstructions());
  }
  static getEndpointParameterInstructions() {
    return defaultEndpointParameterInstructions();
  }
}

function runtimeConfigWrapper(config) {
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
  return getRuntimeConfig(config);
}

const awsJsonSharedCommand = (name, input, context) => {
  const headers = sharedHeaders(name);
  const body = JSON.stringify(_json(input));
  return buildHttpRpcRequest(context, headers, "/", undefined, body);
};

const awsRestXmlSharedCommandError = async (output, context) => {
  const parsedOutput = {
    ...output,
    body: await parseErrorBody(output.body, context),
  };
  const errorCode = loadRestXmlErrorCode(output, parsedOutput.body);
  const parsedBody = parsedOutput.body;
  return throwDefaultError({
    output,
    parsedBody,
    errorCode,
  });
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

  if (serviceName == "s3") {
    const { hostname, protocol, pathname, search } = endpoint.url;
    const [bucket, host] = hostname.split(".s3.");
    if (host) {
      const newHref = `${protocol}//s3.${host}/${bucket}${pathname}${
        search ? `?${search}` : ""
      }`;
      endpoint.url.href = newHref;
    }
  }

  return endpoint;
}

const WRAPPERS = [
  {
    name: "getRuntimeConfig",
    filter: /runtimeConfig\.shared\.js$/,
    wrapper: runtimeConfigWrapper,
  },
];

function defaultEndpointParameterInstructions() {
  return {
    UseFIPS: { type: "builtInParams", name: "useFipsEndpoint" },
    Endpoint: { type: "builtInParams", name: "endpoint" },
    Region: { type: "builtInParams", name: "region" },
    UseDualStack: { type: "builtInParams", name: "useDualstackEndpoint" },
  };
}

function noopFilterSensitiveLog(output) {
  return output;
}

function s3DefaultEndpointParameterInstructions() {
  return {
    Bucket: { type: "contextParams", name: "Bucket" },
    ForcePathStyle: { type: "clientContextParams", name: "forcePathStyle" },
    UseArnRegion: { type: "clientContextParams", name: "useArnRegion" },
    DisableMultiRegionAccessPoints: {
      type: "clientContextParams",
      name: "disableMultiregionAccessPoints",
    },
    Accelerate: { type: "clientContextParams", name: "useAccelerateEndpoint" },
    UseGlobalEndpoint: { type: "builtInParams", name: "useGlobalEndpoint" },
    UseFIPS: { type: "builtInParams", name: "useFipsEndpoint" },
    Endpoint: { type: "builtInParams", name: "endpoint" },
    Region: { type: "builtInParams", name: "region" },
    UseDualStack: { type: "builtInParams", name: "useDualstackEndpoint" },
  };
}

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

function extractStaticJsObject(fn) {
  const lines = fn.toString().split("\n");

  const fnStart = lines.shift();
  const fnEnd = lines.pop();

  const extractionRegex = /return\s*({[\s\S]*?});/gm;
  const functionCode = lines.join("\n");

  const objectName = `${camelToSnakeCase(fn.name)}_OBJ`;
  let jsObject = null;
  let modifiedFunctionCode = functionCode.replace(
    extractionRegex,
    (original, match) => {
      jsObject = match;
      return `return ${objectName};`;
    }
  );

  return [
    `const ${objectName} = ${jsObject}`,
    `${fnStart}\n${modifiedFunctionCode}\n${fnEnd}`,
  ];
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

const awsSdkPlugin = {
  name: "aws-sdk-plugin",
  setup(build) {
    const tslib = require.resolve("tslib/tslib.es6.js");
    const defaultEndpointInstructionsRegex = codeToRegex(
      defaultEndpointParameterInstructions
    );

    const s3DefaultEndpointInstructionsRegex = codeToRegex(
      s3DefaultEndpointParameterInstructions
    );

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

    build.onLoad({ filter: /models\/models_/ }, async ({ path: filePath }) => {
      const source = (await fs.readFile(filePath)).toString();

      let contents = `import { cloneModel } from "${SDK_UTILS_PACKAGE}"\n`;
      contents += source.replace(SPREAD_MODEL_REGEX, "cloneModel\n");

      return {
        contents,
      };
    });

    build.onLoad(
      { filter: /protocols\/Aws_restXml\.js$/ },
      async ({ path: filePath }) => {
        const name = path.parse(filePath).name;
        let source = (await fs.readFile(filePath)).toString();

        const sharedCommandErrorRegex = codeToRegex(
          awsRestXmlSharedCommandError,
          true
        );

        const sourceLength = source.length;

        source = source.replace(
          sharedCommandErrorRegex,
          awsRestXmlSharedCommandError.name
        );

        if (sourceLength == source.length) {
          throw new Error(`Failed to optimize: ${name}`);
        }

        console.log("Optimized:", name);

        source = `const ${
          awsRestXmlSharedCommandError.name
        } = ${awsRestXmlSharedCommandError.toString()}\n\n${source}`;

        return {
          contents: source,
        };
      }
    );

    build.onLoad(
      { filter: /protocols\/Aws_json1_1\.js$/ },
      async ({ path: filePath }) => {
        const name = path.parse(filePath).name;

        let source = (await fs.readFile(filePath)).toString();

        const sourceLength = source.length;

        source = source.replace(AWS_JSON_SHARED_COMMAND_REGEX, (_, name) => {
          return `${awsJsonSharedCommand.name}(${name}, input, context)`;
        });

        if (sourceLength == source.length) {
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

      const paramInstructions = extractStaticJsObject(
        defaultEndpointParameterInstructions
      );

      const s3paramInstructions = extractStaticJsObject(
        s3DefaultEndpointParameterInstructions
      );

      contents += `import { Command as $Command } from "@smithy/smithy-client";\n`;
      contents += `import { getEndpointPlugin } from "@smithy/middleware-endpoint";\n`;
      contents += `import { getSerdePlugin } from "@smithy/middleware-serde";\n`;
      contents += `import { SMITHY_CONTEXT_KEY } from "@smithy/types";\n`;
      contents += `${paramInstructions[0]}\n`;
      contents += `${s3paramInstructions[0]}\n`;
      contents += `export ${paramInstructions[1]}\n`;
      contents += `export ${s3paramInstructions[1]}\n`;
      contents += `export ${noopFilterSensitiveLog.toString()}\n`;
      contents += `export ${executeClientCommand.toString()}\n`;
      contents += `${BaseCommand.toString()}`;
      contents += `export ${DefaultCommand.toString()}`;
      contents += `export ${S3Command.toString()}`;
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

    build.onLoad(
      { filter: /commands\/\w+Command\.js$/ },
      async ({ path: filePath }) => {
        try {
          const source = (await fs.readFile(filePath)).toString();

          const commandName = path.parse(filePath).name;
          const pkg = await findPackageName(filePath);

          let contents;

          const addNoOpFilterImport = (contents) =>
            `import { ${noopFilterSensitiveLog.name} } from "${SDK_UTILS_PACKAGE}"\n${contents}`;

          let isS3 = pkg == "@aws-sdk/client-s3";
          if (isS3) {
            contents = `import { ${s3DefaultEndpointParameterInstructions.name} } from "${SDK_UTILS_PACKAGE}"\n`;
            contents += source.replace(
              s3DefaultEndpointInstructionsRegex,
              `return ${s3DefaultEndpointParameterInstructions.name}();`
            );
          } else {
            contents = `import { ${defaultEndpointParameterInstructions.name} } from "${SDK_UTILS_PACKAGE}"\n`;
            contents += source.replace(
              defaultEndpointInstructionsRegex,
              `return ${defaultEndpointParameterInstructions.name}();`
            );
          }

          const commandClass = COMMANDS_BY_SDK[pkg][commandName];
          const commandPrototype = commandClass.prototype;

          const resolveCode = commandPrototype.resolveMiddleware.toString();

          SHARED_COMMAND_REGEX.lastIndex = -1;
          const regexResult = SHARED_COMMAND_REGEX.exec(resolveCode);
          if (!regexResult) {
            console.log(`Can't optimize: ${commandName}`);

            // let unoptimizedCommand = `unoptimized/${commandName}.js`;
            // await fs.mkdir(path.dirname(unoptimizedCommand), { recursive: true });
            // await fs.writeFile(unoptimizedCommand, resolveCode);

            let addNoopFilter = false;
            contents = contents.replace(
              PROPERTY_NOOP_ARROW_FUNCTION_REGEX,
              (_, match) => {
                addNoopFilter = true;
                return `${match} ${noopFilterSensitiveLog.name}`;
              }
            );

            if (addNoopFilter) {
              contents = addNoOpFilterImport(contents);
            }

            return {
              contents,
            };
          }

          const commandParameterData = Array.from(regexResult);
          commandParameterData.shift();
          let [
            paramClientName,
            paramCmdName,
            paramInputFilter,
            paramOutputFilter,
            service,
            operation,
          ] = commandParameterData;

          const serializeCode = commandPrototype.serialize.toString();
          const deserializeCode = commandPrototype.deserialize.toString();

          SHARED_COMMAND_MARSHALL_REGEX.lastIndex = -1;
          const serializeFunction =
            SHARED_COMMAND_MARSHALL_REGEX.exec(serializeCode)[1];

          SHARED_COMMAND_MARSHALL_REGEX.lastIndex = -1;
          const deserializeFunction =
            SHARED_COMMAND_MARSHALL_REGEX.exec(deserializeCode)[1];

          const defaultCommandName = isS3
            ? S3Command.name
            : DefaultCommand.name;

          const classDefIndex = contents.indexOf("export class ");
          let defaultClassContents = contents.substring(0, classDefIndex);
          let addNoopFilter = false;

          if (NOOP_ARROW_FUNCTION_REGEX.test(paramInputFilter)) {
            paramInputFilter = noopFilterSensitiveLog.name;
            addNoopFilter = true;
          }

          if (NOOP_ARROW_FUNCTION_REGEX.test(paramOutputFilter)) {
            paramOutputFilter = noopFilterSensitiveLog.name;
            addNoopFilter = true;
          }

          if (addNoopFilter) {
            defaultClassContents = addNoOpFilterImport(defaultClassContents);
          }

          defaultClassContents += `import { ${defaultCommandName} } from "${SDK_UTILS_PACKAGE}"\n`;
          defaultClassContents += `export class ${commandPrototype.constructor.name} extends ${defaultCommandName} {
  constructor(input) {
    super(
      input,
      ${paramClientName},
      ${paramCmdName},
      ${paramInputFilter},
      ${paramOutputFilter},
      ${serializeFunction},
      ${deserializeFunction},
      ${service},
      ${operation}
    );
  }
}
`;

          console.log(`Optimized: ${commandName}`);

          return {
            contents: defaultClassContents,
          };
        } catch (error) {
          console.error(error);
          await rmTmpDir();
          process.exit(1);
        }
      }
    );

    for (const { filter, wrapper, name } of WRAPPERS) {
      build.onLoad({ filter }, async ({ path }) => {
        let source = (await fs.readFile(path)).toString();
        let replaced = false;
        source = source.replace(
          RegExp(`export\\s*(const\\s*${name})`),
          (_, replacement) => {
            replaced = true;
            return replacement;
          }
        );
        if (!replaced) {
          throw new Error(`No replacement found for "${name}" in ${filter}`);
        }
        const wrapperName = `${name}Wrapper`;
        let contents = `${source}\n`;
        contents += `const ${wrapperName} = ${wrapper.toString()}\n`;
        contents += `export {${wrapperName} as ${name}}`;

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
    loadShim(/@smithy\/util-hex-encoding/, "util-hex-encoding.js"),
    loadShim(/@aws-sdk\/util-utf8-browser/, "util-utf8.js"),
    loadShim(/@smithy\/util-base64/, "util-base64.js"),
    //    loadShim(/@smithy\/md5-js/, "md5.js"),
    loadShim(/@aws-crypto/, "aws-crypto.js"),
    loadShim(/mnemonist\/lru-cache\.js/, "lru-cache.js"),
  ]);
}

const PACKAGE_NAME_CACHE = {};
async function findPackageName(filePath) {
  const firstDir = path.dirname(filePath);

  if (PACKAGE_NAME_CACHE[firstDir]) {
    return PACKAGE_NAME_CACHE[firstDir];
  }

  let currentDir = firstDir;
  while (true) {
    const packageJsonPath = path.join(currentDir, "package.json");

    const packageJsonExists = await fs
      .access(packageJsonPath)
      .then(() => true)
      .catch(() => false);

    if (packageJsonExists) {
      const packageJsonContent = await fs.readFile(packageJsonPath, "utf8");
      const packageJson = JSON.parse(packageJsonContent);

      if (packageJson && packageJson.name) {
        PACKAGE_NAME_CACHE[firstDir] = packageJson.name;
        return packageJson.name;
      }
    }

    const parentDir = path.dirname(currentDir);
    if (parentDir === currentDir) {
      return null;
    }

    currentDir = parentDir;
  }
}

async function buildLibrary() {
  const entryPoints = {};

  TEST_FILES.forEach((entry) => {
    entryPoints[path.join("__tests__", `${entry.slice(0, -3)}`)] = path.join(
      TESTS_DIR,
      entry
    );
  });

  ENTRYPOINTS.forEach((entry) => {
    entryPoints[entry] = path.join(SRC_DIR, entry);
  });

  await esbuild({
    entryPoints,
    chunkNames: "llrt-[name]-runtime-[hash]",
    ...ES_BUILD_OPTIONS,
    splitting: false,
    keepNames: true,
    nodePaths: ["."],
  });
}

async function buildSdks() {
  const sdkEntryList = await Promise.all(
    SDK_PACKAGES.map(async (pkg) => {
      const packagePath = path.join(TMP_DIR, pkg);
      const sdk = SDKS_BY_SDK_PACKAGES[pkg];
      const sdkIndexFile = path.join(packagePath, "index.js");
      const serviceName = SERVICE_ENDPOINT_BY_PACKAGE[sdk];

      await fs.mkdir(packagePath, { recursive: true });

      let sdkContents = `export * from "${pkg}";`;
      if (serviceName) {
        sdkContents += `\nif(__bootstrap.addAwsSdkInitTask){\n   __bootstrap.addAwsSdkInitTask("${serviceName}");\n}`;

        const commands = await import(`${pkg}/dist-es/commands/index.js`);
        COMMANDS_BY_SDK[pkg] = commands;
      }
      await fs.writeFile(sdkIndexFile, sdkContents);

      return [pkg, sdkIndexFile];
    })
  );

  const sdkEntryPoints = Object.fromEntries(sdkEntryList);

  await esbuild({
    entryPoints: sdkEntryPoints,
    plugins: [awsSdkPlugin, esbuildShimPlugin([[/^bowser$/]])],
    alias: {
      "@aws-sdk/util-utf8": "@aws-sdk/util-utf8-browser",
      "@aws-sdk/xml-builder": "xml",
      "fast-xml-parser": "xml",
      "@smithy/md5-js": "crypto",
    },
    chunkNames: "llrt-[name]-sdk-[hash]",
    ...ES_BUILD_OPTIONS,
  });
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
