[![LLRT CI](https://github.com/awslabs/llrt/actions/workflows/ci.yml/badge.svg?branch=main)](https://github.com/awslabs/llrt/actions/workflows/ci.yml) [![LLRT Release](https://github.com/awslabs/llrt/actions/workflows/release.yml/badge.svg)](https://github.com/awslabs/llrt/actions/workflows/release.yml)

LLRT (**L**ow **L**atency **R**un**t**ime) is a lightweight JavaScript runtime designed to address the growing demand for fast and efficient Serverless applications. LLRT offers up to over **10x** faster startup and up to **2x** overall lower cost compared to other JavaScript runtimes running on **AWS Lambda**

It's built in Rust, utilizing QuickJS as JavaScript engine, ensuring efficient memory usage and swift startup.

> [!WARNING]
> LLRT is an **experimental** package. It is subject to change and intended only for evaluation purposes.

<sub>LLRT - [DynamoDB Put, ARM, 128MB](example/functions/src/v3-lib.mjs):<sub>
![DynamoDB Put LLRT](./benchmarks/llrt-ddb-put.png "LLRT DynamoDB Put")

<sub>Node.js 20 - [DynamoDB Put, ARM, 128MB](example/functions/src/v3-lib.mjs):<sub>
![DynamoDB Put Node20](./benchmarks/node20-ddb-put.png "Node20 DynamoDB Put")

HTTP benchmarks measured in **round trip time** for a cold start ([why?](#benchmark-methodology))

## Configure Lambda functions to use LLRT

Download the last LLRT release from <https://github.com/awslabs/llrt/releases>

### Option 1: Custom runtime (recommended)

Choose `Custom Runtime on Amazon Linux 2023` and package the LLRT `bootstrap` binary together with your JS code.

### Option 2: Use a layer

Choose `Custom Runtime on Amazon Linux 2023`, upload `llrt-lambda-arm64.zip` or `llrt-lambda-x86.zip` as a layer and add to your function

### Option 3: Package LLRT in a container image

See our [AWS SAM example](./example/llrt-sam-container-image) or: 
```dockerfile
FROM --platform=arm64 busybox
WORKDIR /var/task/
COPY app.mjs ./
ADD https://github.com/awslabs/llrt/releases/download/latest/llrt-container-arm64 /usr/bin/llrt
RUN chmod +x /usr/bin/llrt

ENV LAMBDA_HANDLER "app.handler"

CMD [ "llrt" ]
```

That's it 🎉

> [!IMPORTANT]
> Even though LLRT supports [ES2020](https://262.ecma-international.org/11.0/) it's **NOT** a drop in replacement for Node.js. Consult [Compatibility matrix](#compatibility-matrix) and [API](API.md) for more details.
> All dependencies should be bundled for a `browser` platform and mark included `@aws-sdk` packages as external.

### Option 3: AWS SAM

The following [example project](example/llrt-sam/) sets up a lambda
instrumented with a layer containing the llrt runtime.

### Option 4: AWS CDK

You can use [`cdk-lambda-llrt` construct library](https://github.com/tmokmss/cdk-lambda-llrt) to deploy LLRT Lambda functions with AWS CDK.

```ts
import { LlrtFunction } from 'cdk-lambda-llrt';

const handler = new LlrtFunction(this, 'Handler', {
    entry: 'lambda/index.ts',
});
```

See [Construct Hub](https://constructs.dev/packages/cdk-lambda-llrt/) and [its examples](https://github.com/tmokmss/cdk-lambda-llrt/tree/main/example) for more details.

## Testing & ensuring compatibility

The best way to ensure your code is compatible with LLRT is to write tests and execute them using the built-in test runner. The test runner currently supports Jest/Chai assertions. There are two main types of tests you can create:

Unit Tests
- Useful for validating specific modules and functions in isolation
- Allow focused testing of individual components

End-to-End (E2E) Tests
- Validate overall compatibility with AWS SDK and WinterCG compliance
- Test the integration between all components
- Confirm expected behavior from end-user perspective

For more information about the E2E Tests and how to run them, see [here](tests/e2e/README.md).

### Test runner

Test runner uses a lightweight Jest-like API and supports Jest/Chai assertions and the [assert module](https://nodejs.org/api/assert.html) from Node.js for test assertions. For examples on how to implement tests for LLRT see the `/tests` folder of this repository.

To run tests, execute the `llrt test` command. LLRT scans the current directory and sub-directories for files that ends with `*.test.js` or `*.test.mjs`. You can also provide a specific test directory to scan by using the `llrt test -d <directory>` option.

The test runner also has support for filters. Using filters is as simple as adding additional command line arguments, i.e: `llrt test crypto` will only run tests that match the filename containing `crypto`.

## Compatibility matrix

> [!NOTE]
> LLRT only support a fraction of the Node.js APIs. It is **NOT** a drop in replacement for Node.js, nor will it ever be. Below is a high level overview of partially supported APIs and modules. For more details consult the [API](API.md) documentation

|               | Node.js                                  | LLRT ⚠️  |
| ------------- | ---------------------------------------- | ----- |
| buffer        | ✔︎                                       | ✔︎️ |
| streams       | ✔︎                                       | ✔︎\*  |
| child_process | ✔︎                                       | ✔︎⏱ |
| net:sockets   | ✔︎                                       | ✔︎⏱ |
| net:server    | ✔︎                                       | ✔︎    |
| tls           | ✔︎                                       | ✘⏱    |
| fetch         | ✔︎                                        | ✔︎   |
| http         | ✔︎                                        | ✘⏱\*\*    |
| https         | ✔︎                                        | ✘⏱\*\*    |
| fs/promises   | ✔︎                                       | ✔︎    |
| fs            | ✔︎                                       | ✘⏱     |
| path          | ✔︎                                       | ✔︎    |
| timers        | ✔︎                                       | ✔︎    |
| uuid          | ✔︎                                       | ✔︎    |
| crypto        | ✔︎                                       | ✔︎  |
| process       | ✔︎                                       | ✔︎  |
| encoding      | ✔︎                                       | ✔︎    |
| console       | ✔︎                                       | ✔︎    |
| events        | ✔︎                                       | ✔︎    |
| ESM           | ✔︎                                       | ✔︎    |
| CJS           | ✔︎                                       | ✔︎    |
| async/await   | ✔︎                                       | ✔︎    |
| Other modules | ✔︎                                       | ✘    |


_⚠️ = partially supported in LLRT_
_⏱ = planned partial support_
_\* = Not native_
_\*\* = Use fetch instead_

## Using node_modules (dependencies) with LLRT

Since LLRT is meant for performance critical application it's not recommended to deploy `node_modules` without bundling, minification and tree-shaking.

LLRT can work with any bundler of your choice. Below are some configurations for popular bundlers:

### ESBuild

    esbuild index.js --platform=node --target=es2020 --format=esm --bundle --minify --external:@aws-sdk --external:@smithy --external:uuid

### Rollup

```javascript
import resolve from '@rollup/plugin-node-resolve';
import commonjs from '@rollup/plugin-commonjs';
import terser from '@rollup/plugin-terser';

export default {
  input: 'index.js',
  output: {
    file: 'dist/bundle.js',
    format: 'esm',
    sourcemap: true,
    target: 'es2020',
  },
  plugins: [
    resolve(),
    commonjs(),
    terser(),
  ],
  external: ["@aws-sdk","@smithy","uuid"],
};
```

### Webpack

```javascript
import TerserPlugin from 'terser-webpack-plugin';
import nodeExternals from 'webpack-node-externals';

export default {
  entry: './index.js',
  output: {
    path: "dist",
    filename: 'bundle.js',
    libraryTarget: 'module',
  },
  target: 'web',
  mode: 'production',
  resolve: {
    extensions: ['.js'],
  },
  externals: [nodeExternals(),"@aws-sdk","@smithy","uuid"],
  optimization: {
    minimize: true,
    minimizer: [
      new TerserPlugin({
        terserOptions: {
          ecma: 2020,
        },
      }),
    ],
  },
};

```

## Using AWS SDK (v3) with LLRT

LLRT includes many AWS SDK clients and utils as part of the runtime, built into the executable. These SDK Clients have been specifically fine-tuned to offer best performance while not compromising on compatibility. LLRT replaces some JavaScript dependencies used by the AWS SDK by native ones such as Hash calculations and XML parsing.
V3 SDK packages not included in the list below have to be bundled with your source code while marking the following packages as external:

| Bundled AWS SDK packages             |
| ------------------------------------- |
| @aws-sdk/client-dynamodb              |
| @aws-sdk/lib-dynamodb                 |
| @aws-sdk/client-kms                   |
| @aws-sdk/client-lambda                |
| @aws-sdk/client-s3                    |
| @aws-sdk/client-secrets-manager       |
| @aws-sdk/client-ses                   |
| @aws-sdk/client-sns                   |
| @aws-sdk/client-sqs                   |
| @aws-sdk/client-sts                   |
| @aws-sdk/client-ssm                   |
| @aws-sdk/client-cloudwatch-logs       |
| @aws-sdk/client-cloudwatch-events     |
| @aws-sdk/client-eventbridge           |
| @aws-sdk/client-sfn                   |
| @aws-sdk/client-xray                  |
| @aws-sdk/client-cognito-identity      |
| @aws-sdk/util-dynamodb                |
| @aws-sdk/credential-providers         |
| @smithy                               |

> [!IMPORTANT]
> LLRT currently does not support returning streams from SDK responses. Use `response.Body.transformToString();` or `response.Body.transformToByteArray();` as shown below.
> ```javascript
>const response = await client.send(command);
>// or 'transformToByteArray()'
>const str = await response.Body.transformToString();
>```

## Running TypeScript with LLRT

Same principle as dependencies applies when using TypeScript. TypeScript must be bundled and transpiled into ES2020 JavaScript.

> [!NOTE]
> LLRT will not support running TypeScript without transpilation. This is by design for performance reasons. Transpiling requires CPU and memory that adds latency and cost during execution. This can be avoided if done ahead of time during deployment.

## Rationale

What justifies the introduction of another JavaScript runtime in light of existing options such as [Node.js](https://nodejs.org/en), [Bun](https://bun.sh) & [Deno](https://deno.com/)?

Node.js, Bun, and Deno represent highly proficient JavaScript runtimes. However, they are designed with general-purpose applications in mind. These runtimes were not specifically tailored for the demands of a Serverless environment, characterized by short-lived runtime instances. They each depend on a ([Just-In-Time compiler (JIT)](https://en.wikipedia.org/wiki/Just-in-time_compilation) for dynamic code compilation and optimization during execution. While JIT compilation offers substantial long-term performance advantages, it carries a computational and memory overhead.

In contrast, LLRT distinguishes itself by not incorporating a JIT compiler, a strategic decision that yields two significant advantages:

A) JIT compilation is a notably sophisticated technological component, introducing increased system complexity and contributing substantially to the runtime's overall size.

B) Without the JIT overhead, LLRT conserves both CPU and memory resources that can be more efficiently allocated to code execution tasks, thereby reducing application startup times.

## Limitations

There are many cases where LLRT shows notable performance drawbacks compared with JIT-powered runtimes, such as large data processing, Monte Carlo simulations or performing tasks with hundreds of thousands or millions of iterations. LLRT is most effective when applied to smaller Serverless functions dedicated to tasks such as data transformation, real time processing, AWS service integrations, authorization, validation etc. It is designed to complement existing components rather than serve as a comprehensive replacement for everything. Notably, given its supported APIs are based on Node.js specification, transitioning back to alternative solutions requires minimal code adjustments.

## Building from source

Clone code and cd to directory

    git clone git@github.com:awslabs/llrt.git --recursive
    cd llrt

Install git submodules if you've not cloned the repository with `--recursive`

    git submodule update --init

Install rust

    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | bash -s -- -y
    source "$HOME/.cargo/env"

Install dependencies

    # MacOS
    brew install zig make cmake zstd node corepack

    # Ubuntu
    sudo apt -y install make zstd
    sudo snap install zig --classic --beta

Install Node.js packages

    corepack enable
    yarn

Install generate libs and setup rust targets & toolchains

    make stdlib && make libs

> [!NOTE]
> If these commands exit with an error that says `can't cd to zstd/lib`,
> you've not cloned this repository recursively. Run `git submodule update --init` to download the submodules and run the commands above again.

Build release for Lambda

    make release-arm64
    # or for x86-64, use
    make release-x64

Optionally build for your local machine (Mac or Linux)

    make release

You should now have a `llrt-lambda-arm64.zip` or `llrt-lambda-x86.zip`. You can manually upload this as a Lambda layer or use it via your Infrastructure-as-code pipeline

## Running Lambda emulator

Please note that in order to run the example you will need:
- Valid AWS credentials via a `~/.aws/credentials` or via environment variables.
```bash
export AWS_ACCESS_KEY_ID=XXX
export AWS_SECRET_ACCESS_KEY=YYY
export AWS_REGION=us-east-1
```
- A DynamoDB table (with `id` as the partition key) on `us-east-1`
- The `dynamodb:PutItem` IAM permission on this table. You can use this policy (don't forget to modify <YOUR_ACCOUNT_ID>):
```json
{
	"Version": "2012-10-17",
	"Statement": [
		{
			"Sid": "putItem",
			"Effect": "Allow",
			"Action": "dynamodb:PutItem",
			"Resource": "arn:aws:dynamodb:us-east-1:<YOUR_ACCOUNT_ID>:table/quickjs-table"
		}
	]
}
```

Start the `lambda-server.js` in a separate terminal

    node lambda-server.js

Then run llrt:

    make run

## Benchmark Methodology
Although Init Duration [reported by Lambda](https://docs.aws.amazon.com/lambda/latest/dg/lambda-runtime-environment.html) is commonly used to understand cold start impact on overall request latency, this metric does not include the time needed to copy code into the Lambda sandbox.

The technical definition of Init Duration ([source](https://docs.aws.amazon.com/lambda/latest/dg/nodejs-logging.html#node-logging-output)):
> For the first request served, the amount of time it took the runtime to load the function and run code outside of the handler method.

Measuring round-trip request duration provides a more complete picture of user facing cold-start latency.

Lambda invocation results (λ-labeled row) report the sum total of Init Duration + Function Duration.

## Security

See [CONTRIBUTING](CONTRIBUTING.md#security-issue-notifications) for more information.

## License

This library is licensed under the Apache-2.0 License. See the [LICENSE](LICENSE) file.
