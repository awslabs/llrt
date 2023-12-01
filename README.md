![LLRT logo](./llrt-logo.svg "LLRT logo")


LLRT (**L**ow **L**atency **R**un**t**ime) is a lightweight JavaScript runtime designed to address the growing demand for fast and efficient Serverless applications. LLRT offers more than **10x** faster startup and up to **2x** overall lower cost compared to other JavaScript runtimes running on **AWS Lambda**

## Configure Lambda functions to use LLRT

Download the last LLRT release from <https://github.com/awslabs/llrt/releases>

### Option 1: Custom runtime (recommended)

Choose `Custom Runtime on Amazon Linux 2` and package the LLRT `bootstrap` binary together with your JS code.

### Option 2: Use a layer

Choose `Custom Runtime on Amazon Linux 2`, upload `llrt-lambda-arm64.zip` or `llrt-lambda-x86.zip` as a layer and add to your function

Thats it 🎉

## Testing & ensuring compatibility

The best way to ensure that your code is compatible with LLRT is to write tests and executing them via the built in test runner

### Test runner

Test runner uses a lightweight Jest-like API and uses the [assert module](https://nodejs.org/api/assert.html) from Node.js for test assertions. For examples how to implement tests for LLRT see the `/tests` folder of this repository.

To run tests, execute the `llrt test` command. LLRT scans the current directory and sub-directories for files that ends with `*.test.js` or `*.test.mjs`. You can also provide a specific test directory to scan by using the `llrt test -d <directory>` option.

The test runner also has support for filters. Using filters is as simple as adding additional command line arguments, i.e: `llrt test crypto` will only run tests that match the filename containing `crypto`.

## Compatibility matrix

_LLRT does not support all Node.js APIs. It is not a drop in replacement for Node.js, nor will it ever be. Below is a high level overview of supported APIs and modules. For more details consult the [API](API.md) documentation_

|               | Node.js                                  | LLRT  |
| ------------- | ---------------------------------------- | ----- |
| buffer        | ✔︎                                       | ✔︎⚠️⏱ |
| streams       | ✔︎                                       | ✔︎\*  |
| child_process | ✔︎                                       | ✔︎⚠️⏱ |
| net:sockets   | ✔︎                                       | ✔︎⚠️⏱ |
| net:server    | ✔︎                                       | ✔︎⚠️    |
| tls           | ✔︎                                       | ✘⏱    |
| fetch         | ✔︎                                        | ✔︎    |
| http         | ✔︎                                        | ✘⏱\*\*    |
| https         | ✔︎                                        | ✘⏱\*\*    |
| fs/promises   | ✔︎                                       | ✔︎    |
| fs            | ✔︎                                       | ✘⏱     |
| path          | ✔︎                                       | ✔︎    |
| timers        | ✔︎                                       | ✔︎    |
| uuid          | ✘ <sub><sup>(via dependency)</sup></sub> | ✔︎    |
| hex           | ✘ <sub><sup>(via dependency)</sup></sub> | ✔︎    |
| crypto        | ✔︎                                       | ✔︎⚠️  |
| process       | ✔︎                                       | ✔︎⚠️  |
| encoding      | ✔︎                                       | ✔︎    |
| console       | ✔︎                                       | ✔︎    |
| events        | ✔︎                                       | ✔︎    |
| ESM           | ✔︎                                       | ✔︎    |
| CJS           | ✔︎                                       | ✔︎    |
| async/await   | ✔︎                                       | ✔︎    |

_⚠️ = partial support_
_⏱ = planned_
_\* = Not native_
_\*\* = Use fetch instead_

## Using node_modules (dependencies) with LLRT

Since LLRT is meant for performance critical application it's not recommended to deploy `node_modules` without bundling, minification and tree-shaking.

LLRT can work with any bundler of your choice. Below are some configurations for popular bundlers:

### ESBuild

    esbuild index.js --platform=node --target=es2020 --format=esm --bundle --minify --external:@aws-sdk --external:uuid

### Rollup

```javascript
import resolve from 'rollup-plugin-node-resolve';
import commonjs from 'rollup-plugin-commonjs';
import { terser } from 'rollup-plugin-terser';

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
  external: ["@aws-sdk","uuid"],
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
  externals: [nodeExternals(),"@aws-sdk","uuid"],
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

**Bundled AWS SDK packages:**

* @aws-sdk/client-dynamodb
* @aws-sdk/lib-dynamodb
* @aws-sdk/client-kms
* @aws-sdk/client-lambda
* @aws-sdk/client-s3
* @aws-sdk/client-secrets-manager
* @aws-sdk/client-ses
* @aws-sdk/client-sns
* @aws-sdk/client-sqs
* @aws-sdk/client-sts
* @aws-sdk/client-ssm
* @aws-sdk/client-cloudwatch-logs
* @aws-sdk/client-cloudwatch-events
* @aws-sdk/client-eventbridge
* @aws-sdk/client-sfn
* @aws-sdk/client-xray
* @aws-sdk/client-cognito-identity
* @aws-sdk/util-dynamodb
* @aws-sdk/credential-providers
* @smithy/signature-v4

## Running TypeScript with LLRT

Same principle as dependencies applies when using TypeScript. TypeScript must be bundled and transpiled into ES2020 JavaScript.

_Note that LLRT will not support running TypeScript without transpilation. This is by design for performance reasons. Transpiling requires CPU and memory that adds latency and cost during execution. This can be avoided if done ahead of time during deployment._

## Rationale

What justifies the introduction of another JavaScript runtime in light of existing options such as [Node.js](https://nodejs.org/en), [Bun](https://bun.sh) & [Deno](https://deno.com/)?

Node.js, Bun, and Deno represent highly proficient JavaScript runtimes. However, they are designed with general-purpose applications in mind. These runtimes were not specifically tailored for the demands of a Serverless environment, characterized by short-lived runtime instances. They each depend on a ([Just-In-Time compiler (JIT)](https://en.wikipedia.org/wiki/Just-in-time_compilation) for dynamic code compilation and optimization during execution. While JIT compilation offers substantial long-term performance advantages, it carries a computational and memory overhead.

In contrast, LLRT distinguishes itself by not incorporating a JIT compiler, a strategic decision that yields two significant advantages:

A) JIT compilation is a notably sophisticated technological component, introducing increased system complexity and contributing substantially to the runtime's overall size.

B) Without the JIT overhead, LLRT conserves both CPU and memory resources that can be more efficiently allocated to code execution tasks, thereby reducing application startup times.

## Building from source

Install rust

    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | bash -s -- -y
    source "$HOME/.cargo/env"

Install dependencies

    # MacOS
    brew install zig make zstd node

    # Ubuntu
    sudo snap install zig --classic --beta node
    sudo apt -y install make zstd

Clone code and cd to directory

    git clone <repo-url> --recursive
    cd llrt

Install Node.js packages

    npm i

Install generate libs and setup rust targets & toolchains

    make stdlib && make libs

Build release

    make release-arm
    # or for x86, use
    make release-x86

You should now have a `llrt-arm.zip` or `llrt-x86.zip`. You can manually upload this as a Lambda layer or use it via your Infrastructure as code pipeline

## Running Lambda emulator

Please note that in order to run the example you will need:
- Valid AWS credentials via a `~/.aws/credentials` or via environment variables.
```bash
export AWS_ACCESS_KEY_ID=XXX
export AWS_SECRET_ACCESS_KEY=YYY
export AWS_REGION=us-east-1
```
- A DynamoDB table (with `id` as the primary key) on `us-east-1`
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

## Security

See [CONTRIBUTING](CONTRIBUTING.md#security-issue-notifications) for more information.

## License

This library is licensed under the MIT-0 License. See the  [LICENSE](LICENSE) file.
