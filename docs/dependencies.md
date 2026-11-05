# Dependencies

## Using node_modules

Since LLRT is meant for performance critical application it's not recommended to deploy `node_modules` without bundling, minification and tree-shaking.

LLRT can work with any bundler of your choice. Below are some configurations for popular bundlers:

!!! warning
    LLRT implements native modules that are largely compatible with the following external packages.
    By implementing the following conversions in the bundler's alias function, your application may be faster, but we recommend that you test thoroughly as they are not fully compatible.

| Node.js         | LLRT     |
| --------------- | -------- |
| fast-xml-parser | llrt:xml |

### ESBuild

```shell
esbuild index.js --platform=browser --target=es2023 --format=esm --bundle --minify --external:@aws-sdk --external:@smithy
```

### Rollup

```javascript
import resolve from "@rollup/plugin-node-resolve";
import commonjs from "@rollup/plugin-commonjs";
import terser from "@rollup/plugin-terser";

export default {
  input: "index.js",
  output: {
    file: "dist/bundle.js",
    format: "esm",
    sourcemap: true,
    target: "es2023",
  },
  plugins: [resolve(), commonjs(), terser()],
  external: ["@aws-sdk", "@smithy"],
};
```

### Webpack

```javascript
import TerserPlugin from "terser-webpack-plugin";
import nodeExternals from "webpack-node-externals";

export default {
  entry: "./index.js",
  output: {
    path: "dist",
    filename: "bundle.js",
    libraryTarget: "module",
  },
  target: "web",
  mode: "production",
  resolve: {
    extensions: [".js"],
  },
  externals: [nodeExternals(), "@aws-sdk", "@smithy"],
  optimization: {
    minimize: true,
    minimizer: [
      new TerserPlugin({
        terserOptions: {
          ecma: 2023,
        },
      }),
    ],
  },
};
```

## Using AWS SDK (v3) with LLRT

LLRT includes many AWS SDK clients and utils as part of the runtime, built into the executable. These SDK Clients have been specifically fine-tuned to offer best performance while not compromising on compatibility. LLRT replaces some JavaScript dependencies used by the AWS SDK by native ones such as Hash calculations and XML parsing.
V3 SDK packages not included in the list below have to be bundled with your source code. For an example on how to use a non-included SDK, see [this example build script (buildExternalSdkFunction)](https://github.com/awslabs/llrt/blob/main/example/functions/build.mjs)

LLRT supports the following three bundles by default. Bundle types and suffixes are as follows.

| Bundle Type | Suffix      | Purpose of Use                                            |
| ----------- | ----------- | --------------------------------------------------------- |
| no-sdk      | \*-no-sdk   | Suitable for workloads that do not use `@aws-sdk`.        |
| std-sdk     | (none)      | Suitable for workloads that utilize the major `@aws-sdk`. |
| full-sdk    | \*-full-sdk | Suitable for workloads that utilize any `@aws-sdk`.       |

The relationship between the supported packages for each bundle type is as follows.

| Analytics                            | no-sdk | std-sdk | full-sdk |
| ------------------------------------ | ------ | ------- | -------- |
| @aws-sdk/client-athena               |        |         | Yes      |
| @aws-sdk/client-firehose             |        |         | Yes      |
| @aws-sdk/client-glue                 |        |         | Yes      |
| @aws-sdk/client-kinesis              |        |         | Yes      |
| @aws-sdk/client-opensearch           |        |         | Yes      |
| @aws-sdk/client-opensearchserverless |        |         | Yes      |

| Application integration     | no-sdk | std-sdk | full-sdk |
| --------------------------- | ------ | ------- | -------- |
| @aws-sdk/client-eventbridge |        | Yes     | Yes      |
| @aws-sdk/client-scheduler   |        |         | Yes      |
| @aws-sdk/client-sfn         |        | Yes     | Yes      |
| @aws-sdk/client-sns         |        | Yes     | Yes      |
| @aws-sdk/client-sqs         |        | Yes     | Yes      |

| Business applications | no-sdk | std-sdk | full-sdk |
| --------------------- | ------ | ------- | -------- |
| @aws-sdk/client-ses   |        | Yes     | Yes      |
| @aws-sdk/client-sesv2 |        |         | Yes      |

| Compute services             | no-sdk | std-sdk | full-sdk |
| ---------------------------- | ------ | ------- | -------- |
| @aws-sdk/client-auto-scaling |        |         | Yes      |
| @aws-sdk/client-batch        |        |         | Yes      |
| @aws-sdk/client-ec2          |        |         | Yes      |
| @aws-sdk/client-lambda       |        |         | Yes      |

| Containers                       | no-sdk | std-sdk | full-sdk |
| -------------------------------- | ------ | ------- | -------- |
| @aws-sdk/client-ecr              |        |         | Yes      |
| @aws-sdk/client-ecs              |        |         | Yes      |
| @aws-sdk/client-eks              |        |         | Yes      |
| @aws-sdk/client-servicediscovery |        |         | Yes      |

| Databases                        | no-sdk | std-sdk | full-sdk |
| -------------------------------- | ------ | ------- | -------- |
| @aws-sdk/client-dynamodb         |        | Yes     | Yes      |
| @aws-sdk/client-dynamodb-streams |        |         | Yes      |
| @aws-sdk/client-elasticache      |        |         | Yes      |
| @aws-sdk/client-rds              |        |         | Yes      |
| @aws-sdk/client-rds-data         |        |         | Yes      |

| Developer tools      | no-sdk | std-sdk | full-sdk |
| -------------------- | ------ | ------- | -------- |
| @aws-sdk/client-xray |        | Yes     | Yes      |

| Front-end web and mobile services | no-sdk | std-sdk | full-sdk |
| --------------------------------- | ------ | ------- | -------- |
| @aws-sdk/client-amplify           |        |         | Yes      |
| @aws-sdk/client-appsync           |        |         | Yes      |
| @aws-sdk/client-location          |        |         | Yes      |

| Machine Learning (ML) and Artificial Intelligence (AI) | no-sdk | std-sdk | full-sdk |
| ------------------------------------------------------ | ------ | ------- | -------- |
| @aws-sdk/client-bedrock                                |        |         | Yes      |
| @aws-sdk/client-bedrock-runtime                        |        |         | Yes      |
| @aws-sdk/client-bedrock-agent                          |        |         | Yes      |
| @aws-sdk/client-bedrock-agent-runtime                  |        |         | Yes      |
| @aws-sdk/client-polly                                  |        |         | Yes      |
| @aws-sdk/client-rekognition                            |        |         | Yes      |
| @aws-sdk/client-textract                               |        |         | Yes      |
| @aws-sdk/client-translate                              |        |         | Yes      |

| Management and governance         | no-sdk | std-sdk | full-sdk |
| --------------------------------- | ------ | ------- | -------- |
| @aws-sdk/client-appconfig         |        |         | Yes      |
| @aws-sdk/client-appconfigdata     |        |         | Yes      |
| @aws-sdk/client-cloudformation    |        |         | Yes      |
| @aws-sdk/client-cloudwatch        |        |         | Yes      |
| @aws-sdk/client-cloudwatch-events |        | Yes     | Yes      |
| @aws-sdk/client-cloudwatch-logs   |        | Yes     | Yes      |
| @aws-sdk/client-service-catalog   |        |         | Yes      |
| @aws-sdk/client-ssm               |        | Yes     | Yes      |

| Media                        | no-sdk | std-sdk | full-sdk |
| ---------------------------- | ------ | ------- | -------- |
| @aws-sdk/client-mediaconvert |        |         | Yes      |

| Networking and content delivery           | no-sdk | std-sdk | full-sdk |
| ----------------------------------------- | ------ | ------- | -------- |
| @aws-sdk/client-api-gateway               |        |         | Yes      |
| @aws-sdk/client-apigatewayv2              |        |         | Yes      |
| @aws-sdk/client-elastic-load-balancing-v2 |        |         | Yes      |

| Security, identity, and compliance        | no-sdk | std-sdk | full-sdk |
| ----------------------------------------- | ------ | ------- | -------- |
| @aws-sdk/client-acm                       |        |         | Yes      |
| @aws-sdk/client-cognito-identity          |        | Yes     | Yes      |
| @aws-sdk/client-cognito-identity-provider |        | Yes     | Yes      |
| @aws-sdk/client-iam                       |        |         | Yes      |
| @aws-sdk/client-kms                       |        | Yes     | Yes      |
| @aws-sdk/client-secrets-manager           |        | Yes     | Yes      |
| @aws-sdk/client-sso                       |        |         | Yes      |
| @aws-sdk/client-sso-admin                 |        |         | Yes      |
| @aws-sdk/client-sso-oidc                  |        |         | Yes      |
| @aws-sdk/client-sts                       |        | Yes     | Yes      |
| @aws-sdk/client-verifiedpermissions       |        |         | Yes      |

| Storage             | no-sdk | std-sdk | full-sdk |
| ------------------- | ------ | ------- | -------- |
| @aws-sdk/client-efs |        |         | Yes      |
| @aws-sdk/client-s3  |        | Yes     | Yes      |

| Other bundled packages           | no-sdk | std-sdk | full-sdk |
| -------------------------------- | ------ | ------- | -------- |
| @aws-crypto                      |        | Yes     | Yes      |
| @aws-sdk/credential-providers    |        | Yes     | Yes      |
| @aws-sdk/lib-dynamodb            |        | Yes     | Yes      |
| @aws-sdk/lib-storage             |        | Yes     | Yes      |
| @aws-sdk/s3-presigned-post       |        | Yes     | Yes      |
| @aws-sdk/s3-request-presigner    |        | Yes     | Yes      |
| @aws-sdk/util-dynamodb           |        | Yes     | Yes      |
| @aws-sdk/util-user-agent-browser |        | Yes     | Yes      |
| @smithy                          |        | Yes     | Yes      |

!!! note
    Node-only credential providers such as `fromNodeProviderChain` are not supported; rely on the default SDK credentials from environment variables or the Lambda execution role.

!!! tip
    LLRT now supports streaming SDK responses (since version 0.9). You can consume response bodies as streams or use the convenience methods:

    ```javascript
    const response = await client.send(command);

    // Option 1: Stream the response body
    for await (const chunk of response.Body) {
      // process chunk
    }

    // Option 2: Collect as string or bytes
    const str = await response.Body.transformToString();
    // or
    const bytes = await response.Body.transformToByteArray();
    ```

## Running TypeScript with LLRT

Same principle as dependencies applies when using TypeScript. TypeScript must be bundled and transpiled into ES2023 JavaScript.

!!! note
    LLRT will not support running TypeScript without transpilation. This is by design for performance reasons. Transpiling requires CPU and memory that adds latency and cost during execution. This can be avoided if done ahead of time during deployment.
